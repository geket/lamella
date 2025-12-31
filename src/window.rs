//! Window management
//!
//! Handles individual window state, properties, and decorations.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::Geometry;
use crate::workspace::WorkspaceId;

/// Unique identifier for windows
pub type WindowId = Uuid;

bitflags! {
    /// Window state flags
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct WindowState: u32 {
        /// Window is currently focused
        const FOCUSED = 0b00000001;
        /// Window is fullscreen
        const FULLSCREEN = 0b00000010;
        /// Window is maximized
        const MAXIMIZED = 0b00000100;
        /// Window is minimized/hidden
        const HIDDEN = 0b00001000;
        /// Window is floating (not tiled)
        const FLOATING = 0b00010000;
        /// Window is sticky (visible on all workspaces)
        const STICKY = 0b00100000;
        /// Window is urgent (demands attention)
        const URGENT = 0b01000000;
        /// Window is being moved
        const MOVING = 0b10000000;
        /// Window is being resized
        const RESIZING = 0b100000000;
        /// Window is a dialog
        const DIALOG = 0b1000000000;
        /// Window is a modal
        const MODAL = 0b10000000000;
    }
}

/// Window type hints
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WindowType {
    #[default]
    Normal,
    Dialog,
    Utility,
    Toolbar,
    Splash,
    Menu,
    DropdownMenu,
    PopupMenu,
    Tooltip,
    Notification,
    Dock,
    Desktop,
}

/// Window decoration mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DecorationMode {
    /// Full decorations with title bar
    #[default]
    Full,
    /// Only borders, no title bar
    Border,
    /// No decorations at all
    None,
    /// Server-side decorations
    ServerSide,
    /// Client-side decorations
    ClientSide,
}

/// Border style
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BorderStyle {
    Normal,
    Pixel(u32),
    None,
}

impl Default for BorderStyle {
    fn default() -> Self {
        Self::Pixel(2)
    }
}

/// Window size constraints
#[derive(Debug, Clone, Default)]
pub struct SizeHints {
    pub min_width: Option<u32>,
    pub min_height: Option<u32>,
    pub max_width: Option<u32>,
    pub max_height: Option<u32>,
    pub base_width: Option<u32>,
    pub base_height: Option<u32>,
    pub width_increment: Option<u32>,
    pub height_increment: Option<u32>,
    pub aspect_ratio: Option<(u32, u32)>,
}

impl SizeHints {
    /// Constrain a size to these hints
    pub fn constrain(&self, width: u32, height: u32) -> (u32, u32) {
        let mut w = width;
        let mut h = height;

        // Apply min/max
        if let Some(min) = self.min_width {
            w = w.max(min);
        }
        if let Some(max) = self.max_width {
            w = w.min(max);
        }
        if let Some(min) = self.min_height {
            h = h.max(min);
        }
        if let Some(max) = self.max_height {
            h = h.min(max);
        }

        // Apply size increments
        if let (Some(base_w), Some(inc_w)) = (self.base_width, self.width_increment) {
            if inc_w > 0 {
                let steps = (w.saturating_sub(base_w)) / inc_w;
                w = base_w + steps * inc_w;
            }
        }
        if let (Some(base_h), Some(inc_h)) = (self.base_height, self.height_increment) {
            if inc_h > 0 {
                let steps = (h.saturating_sub(base_h)) / inc_h;
                h = base_h + steps * inc_h;
            }
        }

        (w, h)
    }
}

/// Represents a managed window
#[derive(Debug, Clone)]
pub struct Window {
    /// Unique identifier
    pub id: WindowId,

    /// Window title
    pub title: String,

    /// Application ID (app_id in Wayland, WM_CLASS in X11)
    pub app_id: String,

    /// Window class (WM_CLASS instance)
    pub class: String,

    /// Current geometry
    pub geometry: Geometry,

    /// Geometry before fullscreen/maximize
    pub saved_geometry: Option<Geometry>,

    /// Window state flags
    pub state: WindowState,

    /// Window type
    pub window_type: WindowType,

    /// Decoration mode
    pub decoration: DecorationMode,

    /// Border style
    pub border: BorderStyle,

    /// Size hints from the client
    pub size_hints: SizeHints,

    /// Workspace this window belongs to
    pub workspace: Option<WorkspaceId>,

    /// PID of the owning process
    pub pid: Option<u32>,

    /// User-assigned marks (like vim marks)
    pub marks: Vec<String>,

    /// Parent window (for transients/dialogs)
    pub parent: Option<WindowId>,

    /// Child windows
    pub children: Vec<WindowId>,

    /// Is this an XWayland window?
    pub is_xwayland: bool,

    /// Opacity (0.0 - 1.0)
    pub opacity: f32,

    /// Initial/requested position (for floating)
    pub requested_position: Option<(i32, i32)>,

    /// Initial/requested size (for floating)
    pub requested_size: Option<(u32, u32)>,
}

impl Window {
    /// Create a new window
    pub fn new(app_id: String, title: String) -> Self {
        Self {
            id: Uuid::new_v4(),
            title,
            app_id,
            class: String::new(),
            geometry: Geometry::default(),
            saved_geometry: None,
            state: WindowState::empty(),
            window_type: WindowType::Normal,
            decoration: DecorationMode::default(),
            border: BorderStyle::default(),
            size_hints: SizeHints::default(),
            workspace: None,
            pid: None,
            marks: Vec::new(),
            parent: None,
            children: Vec::new(),
            is_xwayland: false,
            opacity: 1.0,
            requested_position: None,
            requested_size: None,
        }
    }

    /// Check if window should be floating by default
    pub fn should_float(&self) -> bool {
        matches!(
            self.window_type,
            WindowType::Dialog
                | WindowType::Utility
                | WindowType::Splash
                | WindowType::Menu
                | WindowType::PopupMenu
                | WindowType::Tooltip
                | WindowType::Notification
        ) || self.parent.is_some()
            || self.state.contains(WindowState::MODAL)
    }

    /// Toggle floating state
    pub fn toggle_floating(&mut self) {
        if self.state.contains(WindowState::FLOATING) {
            self.state.remove(WindowState::FLOATING);
        } else {
            self.state.insert(WindowState::FLOATING);
        }
    }

    /// Enter fullscreen mode
    pub fn set_fullscreen(&mut self, fullscreen: bool, output_geometry: Geometry) {
        if fullscreen && !self.state.contains(WindowState::FULLSCREEN) {
            self.saved_geometry = Some(self.geometry);
            self.geometry = output_geometry;
            self.state.insert(WindowState::FULLSCREEN);
        } else if !fullscreen && self.state.contains(WindowState::FULLSCREEN) {
            if let Some(saved) = self.saved_geometry.take() {
                self.geometry = saved;
            }
            self.state.remove(WindowState::FULLSCREEN);
        }
    }

    /// Toggle fullscreen
    pub fn toggle_fullscreen(&mut self, output_geometry: Geometry) {
        let is_fullscreen = self.state.contains(WindowState::FULLSCREEN);
        self.set_fullscreen(!is_fullscreen, output_geometry);
    }

    /// Set maximized state
    pub fn set_maximized(&mut self, maximized: bool, work_area: Geometry) {
        if maximized && !self.state.contains(WindowState::MAXIMIZED) {
            self.saved_geometry = Some(self.geometry);
            self.geometry = work_area;
            self.state.insert(WindowState::MAXIMIZED);
        } else if !maximized && self.state.contains(WindowState::MAXIMIZED) {
            if let Some(saved) = self.saved_geometry.take() {
                self.geometry = saved;
            }
            self.state.remove(WindowState::MAXIMIZED);
        }
    }

    /// Toggle maximized
    pub fn toggle_maximized(&mut self, work_area: Geometry) {
        let is_maximized = self.state.contains(WindowState::MAXIMIZED);
        self.set_maximized(!is_maximized, work_area);
    }

    /// Check if window is visible
    pub fn is_visible(&self) -> bool {
        !self.state.contains(WindowState::HIDDEN)
    }

    /// Check if window is tiled (not floating)
    pub fn is_tiled(&self) -> bool {
        !self.state.contains(WindowState::FLOATING) && !self.state.contains(WindowState::FULLSCREEN)
    }

    /// Check if window is focused
    pub fn is_focused(&self) -> bool {
        self.state.contains(WindowState::FOCUSED)
    }

    /// Apply size hints to geometry
    pub fn apply_size_hints(&mut self) {
        let (w, h) = self
            .size_hints
            .constrain(self.geometry.width, self.geometry.height);
        self.geometry.width = w;
        self.geometry.height = h;
    }

    /// Resize the window
    pub fn resize(&mut self, width: u32, height: u32) {
        let (w, h) = self.size_hints.constrain(width, height);
        self.geometry.width = w;
        self.geometry.height = h;
    }

    /// Move the window
    pub fn move_to(&mut self, x: i32, y: i32) {
        self.geometry.x = x;
        self.geometry.y = y;
    }

    /// Move and resize
    pub fn set_geometry(&mut self, geometry: Geometry) {
        let (w, h) = self.size_hints.constrain(geometry.width, geometry.height);
        self.geometry = Geometry::new(geometry.x, geometry.y, w, h);
    }

    /// Get effective border width
    pub fn border_width(&self) -> u32 {
        if self.state.contains(WindowState::FULLSCREEN) {
            return 0;
        }
        match self.border {
            BorderStyle::None => 0,
            BorderStyle::Pixel(w) => w,
            BorderStyle::Normal => 2,
        }
    }

    /// Get content geometry (inside borders)
    pub fn content_geometry(&self) -> Geometry {
        let border = self.border_width() as i32;
        Geometry::new(
            self.geometry.x + border,
            self.geometry.y + border,
            self.geometry.width.saturating_sub(border as u32 * 2),
            self.geometry.height.saturating_sub(border as u32 * 2),
        )
    }
}

/// Window matching criteria (for rules)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WindowCriteria {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub class: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title_regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_type: Option<WindowType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub floating: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiling: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub urgent: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub focused: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub workspace: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub con_mark: Option<String>,
}

impl WindowCriteria {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn app_id(mut self, app_id: impl Into<String>) -> Self {
        self.app_id = Some(app_id.into());
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Check if a window matches these criteria
    pub fn matches(&self, window: &Window) -> bool {
        if let Some(ref app_id) = self.app_id {
            if !window.app_id.contains(app_id) {
                return false;
            }
        }

        if let Some(ref class) = self.class {
            if !window.class.contains(class) {
                return false;
            }
        }

        if let Some(ref title) = self.title {
            if !window.title.contains(title) {
                return false;
            }
        }

        if let Some(ref title_regex) = self.title_regex {
            // In a real implementation, we'd use regex crate
            if !window.title.contains(title_regex) {
                return false;
            }
        }

        if let Some(window_type) = self.window_type {
            if window.window_type != window_type {
                return false;
            }
        }

        if let Some(floating) = self.floating {
            if window.state.contains(WindowState::FLOATING) != floating {
                return false;
            }
        }

        if let Some(urgent) = self.urgent {
            if window.state.contains(WindowState::URGENT) != urgent {
                return false;
            }
        }

        if let Some(focused) = self.focused {
            if window.state.contains(WindowState::FOCUSED) != focused {
                return false;
            }
        }

        if let Some(ref mark) = self.con_mark {
            if !window.marks.contains(mark) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_window_state_flags() {
        let mut window = Window::new("test".into(), "Test Window".into());
        assert!(!window.is_focused());

        window.state.insert(WindowState::FOCUSED);
        assert!(window.is_focused());

        window.state.insert(WindowState::FLOATING);
        assert!(!window.is_tiled());
    }

    #[test]
    fn test_size_hints() {
        let hints = SizeHints {
            min_width: Some(100),
            min_height: Some(100),
            max_width: Some(500),
            max_height: Some(500),
            ..Default::default()
        };

        assert_eq!(hints.constrain(50, 50), (100, 100));
        assert_eq!(hints.constrain(1000, 1000), (500, 500));
        assert_eq!(hints.constrain(200, 300), (200, 300));
    }

    #[test]
    fn test_window_criteria() {
        let window = Window::new("firefox".into(), "Mozilla Firefox".into());

        let criteria = WindowCriteria::new().app_id("firefox");
        assert!(criteria.matches(&window));

        let criteria = WindowCriteria::new().app_id("chrome");
        assert!(!criteria.matches(&window));
    }
}
