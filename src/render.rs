//! Rendering module for Fluxway compositor
//!
//! This module handles GPU-accelerated rendering of windows, borders,
//! tab bars, and other visual elements using Smithay 0.3's renderer.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use smithay::{
    backend::renderer::{Frame, ImportShm, Renderer, Texture, Transform},
    utils::{Logical, Physical, Point, Rectangle, Size},
};
use uuid::Uuid;

use crate::config::ColorConfig;
use crate::state::Geometry;
use crate::window::WindowId;

/// Render statistics
#[derive(Debug, Clone)]
pub struct RenderStats {
    /// Number of frames rendered
    pub frames_rendered: u64,
    /// Number of frames skipped
    pub frames_skipped: u64,
    /// Average frame time in milliseconds
    pub avg_frame_time_ms: f64,
    /// Peak frame time in milliseconds
    pub peak_frame_time_ms: f64,
    /// Total render time
    pub total_render_time: Duration,
    /// Time of last frame
    pub last_frame: Instant,
    /// Recent frame times for averaging
    frame_times: Vec<Duration>,
}

impl Default for RenderStats {
    fn default() -> Self {
        Self {
            frames_rendered: 0,
            frames_skipped: 0,
            avg_frame_time_ms: 0.0,
            peak_frame_time_ms: 0.0,
            total_render_time: Duration::ZERO,
            last_frame: Instant::now(),
            frame_times: Vec::with_capacity(60),
        }
    }
}

impl RenderStats {
    /// Record a frame render
    pub fn record_frame(&mut self, frame_time: Duration) {
        self.frames_rendered += 1;
        self.total_render_time += frame_time;
        self.last_frame = Instant::now();

        let ms = frame_time.as_secs_f64() * 1000.0;
        if ms > self.peak_frame_time_ms {
            self.peak_frame_time_ms = ms;
        }

        // Keep last 60 frames for averaging
        self.frame_times.push(frame_time);
        if self.frame_times.len() > 60 {
            self.frame_times.remove(0);
        }

        // Calculate average
        if !self.frame_times.is_empty() {
            let total: Duration = self.frame_times.iter().sum();
            self.avg_frame_time_ms = total.as_secs_f64() * 1000.0 / self.frame_times.len() as f64;
        }
    }

    /// Record a skipped frame
    pub fn record_skip(&mut self) {
        self.frames_skipped += 1;
    }

    /// Get current FPS
    pub fn fps(&self) -> f64 {
        if self.avg_frame_time_ms > 0.0 {
            1000.0 / self.avg_frame_time_ms
        } else {
            0.0
        }
    }
}

/// Per-output render state
#[derive(Debug)]
pub struct OutputRenderState {
    /// Output identifier
    pub output_id: Uuid,
    /// Frame counter
    pub frame_count: u64,
    /// Render statistics
    pub stats: RenderStats,
    /// Whether full redraw is needed
    pub needs_full_redraw: bool,
    /// Pending damage rectangles
    pub pending_damage: Vec<Rectangle<i32, Logical>>,
}

impl OutputRenderState {
    /// Create new output render state
    pub fn new(output_id: Uuid) -> Self {
        Self {
            output_id,
            frame_count: 0,
            stats: RenderStats::default(),
            needs_full_redraw: true,
            pending_damage: Vec::new(),
        }
    }

    /// Mark for full redraw
    pub fn request_full_redraw(&mut self) {
        self.needs_full_redraw = true;
    }

    /// Add damage region
    pub fn add_damage(&mut self, rect: Rectangle<i32, Logical>) {
        self.pending_damage.push(rect);
    }

    /// Clear pending damage
    pub fn clear_damage(&mut self) {
        self.pending_damage.clear();
        self.needs_full_redraw = false;
    }
}

/// RGBA color with 32-bit float components
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Color32F {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color32F {
    /// Create new color
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create from hex string (e.g., "#ff0000" or "ff0000ff")
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.trim_start_matches('#');
        let len = hex.len();

        let (r, g, b, a) = match len {
            6 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                (r, g, b, 255u8)
            },
            8 => {
                let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
                let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
                let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
                let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
                (r, g, b, a)
            },
            _ => return None,
        };

        Some(Self {
            r: f32::from(r) / 255.0,
            g: f32::from(g) / 255.0,
            b: f32::from(b) / 255.0,
            a: f32::from(a) / 255.0,
        })
    }

    /// Convert to array [r, g, b, a]
    pub const fn to_array(self) -> [f32; 4] {
        [self.r, self.g, self.b, self.a]
    }

    /// Black color
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    /// White color
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    /// Transparent
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
}

impl Default for Color32F {
    fn default() -> Self {
        Self::BLACK
    }
}

/// Border render configuration
#[derive(Debug, Clone)]
pub struct BorderRenderConfig {
    /// Border width in pixels
    pub width: u32,
    /// Border color
    pub color: Color32F,
    /// Whether borders are rounded
    pub rounded: bool,
    /// Corner radius if rounded
    pub radius: u32,
}

impl Default for BorderRenderConfig {
    fn default() -> Self {
        Self {
            width: 2,
            color: Color32F::from_hex("#4c7899").unwrap_or(Color32F::BLACK),
            rounded: false,
            radius: 0,
        }
    }
}

impl BorderRenderConfig {
    /// Create from color config
    pub fn from_colors(colors: &crate::config::WindowColors, width: u32) -> Self {
        Self {
            width,
            color: Color32F::from_hex(&colors.border).unwrap_or(Color32F::BLACK),
            rounded: false,
            radius: 0,
        }
    }

    /// Calculate border rectangles for a window geometry
    pub fn border_rects(&self, geo: Rectangle<i32, Logical>) -> Vec<Rectangle<i32, Logical>> {
        let w = self.width as i32;
        let mut rects = Vec::with_capacity(4);

        if w <= 0 {
            return rects;
        }

        // Top border
        rects.push(Rectangle::from_loc_and_size(
            (geo.loc.x - w, geo.loc.y - w),
            (geo.size.w + w * 2, w),
        ));

        // Bottom border
        rects.push(Rectangle::from_loc_and_size(
            (geo.loc.x - w, geo.loc.y + geo.size.h),
            (geo.size.w + w * 2, w),
        ));

        // Left border
        rects.push(Rectangle::from_loc_and_size(
            (geo.loc.x - w, geo.loc.y),
            (w, geo.size.h),
        ));

        // Right border
        rects.push(Rectangle::from_loc_and_size(
            (geo.loc.x + geo.size.w, geo.loc.y),
            (w, geo.size.h),
        ));

        rects
    }
}

/// Tab information for tab bar
#[derive(Debug, Clone)]
pub struct TabInfo {
    /// Window ID
    pub window_id: WindowId,
    /// Tab title
    pub title: String,
    /// Whether tab is active
    pub active: bool,
    /// Whether window is urgent
    pub urgent: bool,
}

/// Tab bar colors
#[derive(Debug, Clone)]
pub struct TabBarColors {
    pub background: Color32F,
    pub active_bg: Color32F,
    pub inactive_bg: Color32F,
    pub active_text: Color32F,
    pub inactive_text: Color32F,
    pub urgent_bg: Color32F,
}

impl Default for TabBarColors {
    fn default() -> Self {
        Self {
            background: Color32F::from_hex("#1d2021").unwrap_or(Color32F::BLACK),
            active_bg: Color32F::from_hex("#4c7899").unwrap_or(Color32F::BLACK),
            inactive_bg: Color32F::from_hex("#333333").unwrap_or(Color32F::BLACK),
            active_text: Color32F::WHITE,
            inactive_text: Color32F::from_hex("#888888").unwrap_or(Color32F::WHITE),
            urgent_bg: Color32F::from_hex("#900000").unwrap_or(Color32F::BLACK),
        }
    }
}

/// Tab bar element for rendering tabbed containers
#[derive(Debug)]
pub struct TabBarElement {
    /// Geometry of tab bar
    pub geometry: Rectangle<i32, Logical>,
    /// Tab height
    pub tab_height: i32,
    /// Tabs
    pub tabs: Vec<TabInfo>,
    /// Colors
    pub colors: TabBarColors,
}

impl TabBarElement {
    /// Create new tab bar
    pub fn new(geometry: Rectangle<i32, Logical>, tab_height: i32) -> Self {
        Self {
            geometry,
            tab_height,
            tabs: Vec::new(),
            colors: TabBarColors::default(),
        }
    }

    /// Add a tab
    pub fn add_tab(&mut self, tab: TabInfo) {
        self.tabs.push(tab);
    }

    /// Calculate tab rectangles
    pub fn tab_rects(&self) -> Vec<(Rectangle<i32, Logical>, Color32F)> {
        if self.tabs.is_empty() {
            return Vec::new();
        }

        let tab_width = self.geometry.size.w / self.tabs.len() as i32;
        let mut rects = Vec::with_capacity(self.tabs.len());

        for (i, tab) in self.tabs.iter().enumerate() {
            let x = self.geometry.loc.x + tab_width * i as i32;
            let rect = Rectangle::from_loc_and_size(
                (x, self.geometry.loc.y),
                (tab_width, self.tab_height),
            );

            let color = if tab.urgent {
                self.colors.urgent_bg
            } else if tab.active {
                self.colors.active_bg
            } else {
                self.colors.inactive_bg
            };

            rects.push((rect, color));
        }

        rects
    }
}

/// Animation easing curve
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum EasingCurve {
    Linear,
    #[default]
    EaseOutCubic,
    EaseOutQuad,
    EaseInOutCubic,
}

impl EasingCurve {
    /// Apply easing to t in [0, 1]
    pub fn apply(self, t: f64) -> f64 {
        match self {
            Self::Linear => t,
            Self::EaseOutCubic => 1.0 - (1.0 - t).powi(3),
            Self::EaseOutQuad => 1.0 - (1.0 - t).powi(2),
            Self::EaseInOutCubic => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            },
        }
    }
}

/// Single animation
#[derive(Debug, Clone)]
pub struct Animation {
    pub from: f64,
    pub to: f64,
    pub start: Instant,
    pub duration: Duration,
    pub curve: EasingCurve,
}

impl Animation {
    /// Create new animation
    pub fn new(from: f64, to: f64, duration: Duration, curve: EasingCurve) -> Self {
        Self {
            from,
            to,
            start: Instant::now(),
            duration,
            curve,
        }
    }

    /// Get current animated value
    pub fn value(&self) -> f64 {
        let elapsed = self.start.elapsed();
        if elapsed >= self.duration {
            return self.to;
        }

        let t = elapsed.as_secs_f64() / self.duration.as_secs_f64();
        let eased = self.curve.apply(t);
        self.from + (self.to - self.from) * eased
    }

    /// Check if animation is complete
    pub fn is_complete(&self) -> bool {
        self.start.elapsed() >= self.duration
    }
}

/// Window animation state
#[derive(Debug, Clone)]
pub struct WindowAnimation {
    pub x: Option<Animation>,
    pub y: Option<Animation>,
    pub width: Option<Animation>,
    pub height: Option<Animation>,
    pub opacity: Option<Animation>,
}

impl WindowAnimation {
    /// Create empty animation state
    pub fn new() -> Self {
        Self {
            x: None,
            y: None,
            width: None,
            height: None,
            opacity: None,
        }
    }

    /// Check if all animations are complete
    pub fn is_complete(&self) -> bool {
        self.x.as_ref().map_or(true, |a| a.is_complete())
            && self.y.as_ref().map_or(true, |a| a.is_complete())
            && self.width.as_ref().map_or(true, |a| a.is_complete())
            && self.height.as_ref().map_or(true, |a| a.is_complete())
            && self.opacity.as_ref().map_or(true, |a| a.is_complete())
    }

    /// Get current geometry
    pub fn current_geometry(&self, base: Geometry) -> Geometry {
        Geometry {
            x: self.x.as_ref().map_or(base.x, |a| a.value() as i32),
            y: self.y.as_ref().map_or(base.y, |a| a.value() as i32),
            width: self.width.as_ref().map_or(base.width, |a| a.value() as u32),
            height: self
                .height
                .as_ref()
                .map_or(base.height, |a| a.value() as u32),
        }
    }

    /// Get current opacity
    pub fn current_opacity(&self) -> f32 {
        self.opacity.as_ref().map_or(1.0, |a| a.value() as f32)
    }
}

impl Default for WindowAnimation {
    fn default() -> Self {
        Self::new()
    }
}

/// Animation manager
#[derive(Debug)]
pub struct AnimationManager {
    /// Window animations
    animations: HashMap<WindowId, WindowAnimation>,
    /// Whether animations are enabled
    pub enabled: bool,
    /// Default animation duration
    pub duration: Duration,
    /// Default easing curve
    pub curve: EasingCurve,
}

impl AnimationManager {
    /// Create new animation manager
    pub fn new() -> Self {
        Self {
            animations: HashMap::new(),
            enabled: true,
            duration: Duration::from_millis(150),
            curve: EasingCurve::EaseOutCubic,
        }
    }

    /// Start geometry animation for a window
    pub fn animate_geometry(&mut self, window_id: WindowId, from: Geometry, to: Geometry) {
        if !self.enabled {
            return;
        }

        let anim = self.animations.entry(window_id).or_default();

        if from.x != to.x {
            anim.x = Some(Animation::new(
                from.x as f64,
                to.x as f64,
                self.duration,
                self.curve,
            ));
        }
        if from.y != to.y {
            anim.y = Some(Animation::new(
                from.y as f64,
                to.y as f64,
                self.duration,
                self.curve,
            ));
        }
        if from.width != to.width {
            anim.width = Some(Animation::new(
                from.width as f64,
                to.width as f64,
                self.duration,
                self.curve,
            ));
        }
        if from.height != to.height {
            anim.height = Some(Animation::new(
                from.height as f64,
                to.height as f64,
                self.duration,
                self.curve,
            ));
        }
    }

    /// Start opacity animation
    pub fn animate_opacity(&mut self, window_id: WindowId, from: f32, to: f32) {
        if !self.enabled {
            return;
        }

        let anim = self.animations.entry(window_id).or_default();
        anim.opacity = Some(Animation::new(
            from as f64,
            to as f64,
            self.duration,
            self.curve,
        ));
    }

    /// Update animations, removing completed ones
    pub fn update(&mut self) {
        self.animations.retain(|_, anim| !anim.is_complete());
    }

    /// Get animation for a window
    pub fn get(&self, window_id: WindowId) -> Option<&WindowAnimation> {
        self.animations.get(&window_id)
    }

    /// Check if window is being animated
    pub fn is_animating(&self, window_id: WindowId) -> bool {
        self.animations.contains_key(&window_id)
    }
}

impl Default for AnimationManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Render context for a frame
pub struct RenderContext<'a> {
    /// Color configuration
    pub colors: &'a ColorConfig,
    /// Border configuration
    pub border_width: u32,
    /// Animation manager
    pub animations: &'a AnimationManager,
    /// Output scale
    pub scale: f64,
    /// Whether to render borders
    pub render_borders: bool,
    /// Whether to render shadows
    pub render_shadows: bool,
}

impl<'a> RenderContext<'a> {
    /// Get focused window border config
    pub fn focused_border(&self) -> BorderRenderConfig {
        BorderRenderConfig::from_colors(&self.colors.focused, self.border_width)
    }

    /// Get unfocused window border config
    pub fn unfocused_border(&self) -> BorderRenderConfig {
        BorderRenderConfig::from_colors(&self.colors.unfocused, self.border_width)
    }

    /// Get urgent window border config
    pub fn urgent_border(&self) -> BorderRenderConfig {
        BorderRenderConfig::from_colors(&self.colors.urgent, self.border_width)
    }
}

/// Wallpaper display mode
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum WallpaperMode {
    Stretch,
    Fit,
    #[default]
    Fill,
    Center,
    Tile,
}

/// Wallpaper configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Wallpaper {
    /// Path to wallpaper image
    pub path: Option<String>,
    /// Solid color fallback
    pub color: String,
    /// Display mode
    pub mode: WallpaperMode,
}

impl Default for Wallpaper {
    fn default() -> Self {
        Self {
            path: None,
            color: "#1d2021".to_string(),
            mode: WallpaperMode::Fill,
        }
    }
}

impl Wallpaper {
    /// Create solid color wallpaper
    pub fn solid(color: &str) -> Self {
        Self {
            path: None,
            color: color.to_string(),
            mode: WallpaperMode::Fill,
        }
    }

    /// Create image wallpaper
    pub fn image(path: &str, mode: WallpaperMode) -> Self {
        Self {
            path: Some(path.to_string()),
            color: "#000000".to_string(),
            mode,
        }
    }

    /// Get color as `Color32F`
    pub fn as_color32f(&self) -> Color32F {
        Color32F::from_hex(&self.color).unwrap_or(Color32F::BLACK)
    }
}

/// Frame scheduler for consistent frame timing
#[derive(Debug)]
pub struct FrameScheduler {
    /// Target FPS
    pub target_fps: u32,
    /// Target frame duration
    target_duration: Duration,
    /// Last frame time
    pub last_frame: Instant,
    /// Whether `VSync` is enabled
    pub vsync: bool,
    /// Accumulated time
    accumulated: Duration,
}

impl FrameScheduler {
    /// Create new frame scheduler
    pub fn new(target_fps: u32, vsync: bool) -> Self {
        Self {
            target_fps,
            target_duration: Duration::from_secs_f64(1.0 / target_fps as f64),
            last_frame: Instant::now(),
            vsync,
            accumulated: Duration::ZERO,
        }
    }

    /// Check if we should render a frame
    pub fn should_render(&mut self) -> bool {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame);
        self.accumulated += elapsed;

        if self.accumulated >= self.target_duration {
            self.accumulated = Duration::ZERO;
            true
        } else {
            false
        }
    }

    /// Get time until next frame
    pub fn time_to_next_frame(&self) -> Duration {
        if self.accumulated >= self.target_duration {
            Duration::ZERO
        } else {
            self.target_duration - self.accumulated
        }
    }

    /// Mark frame as complete
    pub fn frame_complete(&mut self) {
        self.last_frame = Instant::now();
    }
}

impl Default for FrameScheduler {
    fn default() -> Self {
        Self::new(60, true)
    }
}

/// Render a solid color rectangle
pub fn render_solid_rect<R: Renderer>(
    renderer: &mut R,
    rect: Rectangle<i32, Physical>,
    color: Color32F,
) {
    // Smithay 0.3 renderer implementation would go here
    // This is a placeholder for the actual rendering call
}

/// Render window border
pub fn render_border<R: Renderer>(
    renderer: &mut R,
    geo: Rectangle<i32, Physical>,
    config: &BorderRenderConfig,
) {
    let rects = config.border_rects(Rectangle::from_loc_and_size(
        (geo.loc.x, geo.loc.y),
        (geo.size.w, geo.size.h),
    ));

    for rect in rects {
        let phys_rect =
            Rectangle::from_loc_and_size((rect.loc.x, rect.loc.y), (rect.size.w, rect.size.h));
        render_solid_rect(renderer, phys_rect, config.color);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color32f_from_hex() {
        let color = Color32F::from_hex("#ff0000").unwrap();
        assert!((color.r - 1.0).abs() < 0.01);
        assert!(color.g.abs() < 0.01);
        assert!(color.b.abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);

        let color = Color32F::from_hex("00ff00ff").unwrap();
        assert!(color.r.abs() < 0.01);
        assert!((color.g - 1.0).abs() < 0.01);
        assert!(color.b.abs() < 0.01);
        assert!((color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_easing_curves() {
        // Linear should be identity
        assert!((EasingCurve::Linear.apply(0.5) - 0.5).abs() < 0.001);

        // All curves should start at 0 and end at 1
        for curve in [
            EasingCurve::Linear,
            EasingCurve::EaseOutCubic,
            EasingCurve::EaseOutQuad,
            EasingCurve::EaseInOutCubic,
        ] {
            assert!(curve.apply(0.0).abs() < 0.001);
            assert!((curve.apply(1.0) - 1.0).abs() < 0.001);
        }
    }

    #[test]
    fn test_render_stats() {
        let mut stats = RenderStats::default();
        assert_eq!(stats.frames_rendered, 0);

        stats.record_frame(Duration::from_millis(16));
        assert_eq!(stats.frames_rendered, 1);
        assert!(stats.avg_frame_time_ms > 0.0);
    }

    #[test]
    fn test_frame_scheduler() {
        let scheduler = FrameScheduler::new(60, false);
        assert!(scheduler.time_to_next_frame() > Duration::ZERO);
    }

    #[test]
    fn test_border_rects() {
        let config = BorderRenderConfig {
            width: 2,
            color: Color32F::BLACK,
            rounded: false,
            radius: 0,
        };

        let geo = Rectangle::from_loc_and_size((100, 100), (200, 200));
        let rects = config.border_rects(geo);

        assert_eq!(rects.len(), 4);
    }

    #[test]
    fn test_animation() {
        let anim = Animation::new(0.0, 100.0, Duration::from_millis(100), EasingCurve::Linear);
        assert!((anim.value() - 0.0).abs() < 1.0); // Should be near start
        assert!(!anim.is_complete());
    }

    #[test]
    fn test_animation_manager() {
        let mut manager = AnimationManager::new();
        let window_id = Uuid::new_v4();

        let from = Geometry {
            x: 0,
            y: 0,
            width: 100,
            height: 100,
        };
        let to = Geometry {
            x: 50,
            y: 50,
            width: 200,
            height: 200,
        };

        manager.animate_geometry(window_id, from, to);
        assert!(manager.is_animating(window_id));
    }

    #[test]
    fn test_tab_bar_rects() {
        let mut tab_bar = TabBarElement::new(Rectangle::from_loc_and_size((0, 0), (300, 30)), 30);

        tab_bar.add_tab(TabInfo {
            window_id: Uuid::new_v4(),
            title: "Tab 1".to_string(),
            active: true,
            urgent: false,
        });
        tab_bar.add_tab(TabInfo {
            window_id: Uuid::new_v4(),
            title: "Tab 2".to_string(),
            active: false,
            urgent: false,
        });

        let rects = tab_bar.tab_rects();
        assert_eq!(rects.len(), 2);
        assert_eq!(rects[0].0.size.w, 150); // Each tab is half width
    }

    #[test]
    fn test_wallpaper() {
        let wp = Wallpaper::solid("#ff0000");
        let color = wp.as_color32f();
        assert!((color.r - 1.0).abs() < 0.01);
    }
}
