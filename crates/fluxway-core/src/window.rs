//! Window management types.
//!
//! Protocol-agnostic window representation. No display-server handles leak here.

use bitflags::bitflags;
use serde::{Deserialize, Serialize};

use crate::state::Geometry;
use crate::workspace::WorkspaceId;

/// Unique, opaque identifier for a managed window.
///
/// Backends maintain a mapping from their protocol-specific surface handle
/// to this ID. Core never sees protocol handles.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WindowId(pub u64);

impl std::fmt::Display for WindowId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "win:{}", self.0)
    }
}

bitflags! {
    /// Window state flags.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct WindowState: u32 {
        const FOCUSED    = 0b0000_0000_0001;
        const FULLSCREEN = 0b0000_0000_0010;
        const MAXIMIZED  = 0b0000_0000_0100;
        const HIDDEN     = 0b0000_0000_1000;
        const FLOATING   = 0b0000_0001_0000;
        const STICKY     = 0b0000_0010_0000;
        const URGENT     = 0b0000_0100_0000;
        const MOVING     = 0b0000_1000_0000;
        const RESIZING   = 0b0001_0000_0000;
        const DIALOG     = 0b0010_0000_0000;
        const MODAL      = 0b0100_0000_0000;
    }
}

/// Window type hints.
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

/// Border style.
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

/// Window size constraints.
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
    /// Constrain a size to these hints.
    pub fn constrain(&self, width: u32, height: u32) -> (u32, u32) {
        let mut w = width;
        let mut h = height;

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

/// A managed window (protocol-agnostic).
#[derive(Debug, Clone)]
pub struct Window {
    pub id: WindowId,
    pub title: String,
    pub app_id: String,
    pub class: String,
    pub geometry: Geometry,
    pub saved_geometry: Option<Geometry>,
    pub state: WindowState,
    pub window_type: WindowType,
    pub border: BorderStyle,
    pub size_hints: SizeHints,
    pub workspace: Option<WorkspaceId>,
    pub pid: Option<u32>,
    pub marks: Vec<String>,
    pub parent: Option<WindowId>,
    pub children: Vec<WindowId>,
    pub is_xwayland: bool,
    pub opacity: f32,
}

impl Window {
    /// Create a new window with the given ID.
    pub fn new(id: WindowId, app_id: String, title: String) -> Self {
        Self {
            id,
            title,
            app_id,
            class: String::new(),
            geometry: Geometry::default(),
            saved_geometry: None,
            state: WindowState::empty(),
            window_type: WindowType::Normal,
            border: BorderStyle::default(),
            size_hints: SizeHints::default(),
            workspace: None,
            pid: None,
            marks: Vec::new(),
            parent: None,
            children: Vec::new(),
            is_xwayland: false,
            opacity: 1.0,
        }
    }

    /// Check if window should be floating by default.
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

    /// Toggle floating state.
    pub fn toggle_floating(&mut self) {
        if self.state.contains(WindowState::FLOATING) {
            self.state.remove(WindowState::FLOATING);
        } else {
            self.state.insert(WindowState::FLOATING);
        }
    }

    /// Enter/leave fullscreen.
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

    pub fn is_visible(&self) -> bool {
        !self.state.contains(WindowState::HIDDEN)
    }

    pub fn is_tiled(&self) -> bool {
        !self.state.contains(WindowState::FLOATING)
            && !self.state.contains(WindowState::FULLSCREEN)
    }

    pub fn is_focused(&self) -> bool {
        self.state.contains(WindowState::FOCUSED)
    }

    /// Set geometry respecting size hints.
    pub fn set_geometry(&mut self, geometry: Geometry) {
        let (w, h) = self.size_hints.constrain(geometry.width, geometry.height);
        self.geometry = Geometry::new(geometry.x, geometry.y, w, h);
    }

    /// Effective border width.
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
}

/// Window matching criteria (for rules).
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

    /// Check if a window matches these criteria.
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
        let mut window = Window::new(WindowId(1), "test".into(), "Test Window".into());
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
        let window = Window::new(WindowId(1), "firefox".into(), "Mozilla Firefox".into());
        let criteria = WindowCriteria::new().app_id("firefox");
        assert!(criteria.matches(&window));
        let criteria = WindowCriteria::new().app_id("chrome");
        assert!(!criteria.matches(&window));
    }
}
