//! Protocol-agnostic events and actions.
//!
//! [`CoreEvent`] represents what the backend tells core.
//! [`CoreAction`] represents what core tells the backend to do.

use crate::state::Geometry;
use crate::window::WindowId;
use crate::workspace::WorkspaceId;

/// Events that a backend sends to the core engine.
///
/// Backends translate protocol-specific events (Wayland surface map,
/// X11 `MapNotify`, etc.) into these protocol-agnostic variants.
#[derive(Debug, Clone)]
pub enum CoreEvent {
    /// A new window has been mapped (appeared).
    WindowMapped {
        id: WindowId,
        app_id: Option<String>,
        title: Option<String>,
        pid: Option<u32>,
        initial_geometry: Option<Geometry>,
        is_xwayland: bool,
    },

    /// A window has been unmapped (closed/destroyed).
    WindowUnmapped { id: WindowId },

    /// A window committed new state (e.g., resized itself).
    WindowCommit {
        id: WindowId,
        new_geometry_hint: Option<Geometry>,
    },

    /// A window is requesting focus (e.g., urgent hint).
    FocusRequested { id: WindowId },

    /// A new output (monitor) was connected.
    OutputAdded {
        id: u64,
        name: String,
        geometry: Geometry,
    },

    /// An output was disconnected.
    OutputRemoved { id: u64 },

    /// Pointer moved to absolute position.
    PointerMotion { x: f64, y: f64 },

    /// Pointer button press/release. `button` uses Linux event codes.
    PointerButton { button: u32, pressed: bool },

    /// Frame tick — drives relayout and visibility updates.
    Tick,
}

/// Actions that core returns to the backend for execution.
///
/// The backend must apply these to the display server (configure
/// surfaces, set keyboard focus, etc.).
#[derive(Debug, Clone, PartialEq)]
pub enum CoreAction {
    /// Set the geometry (position + size) of a window.
    SetWindowGeometry {
        id: WindowId,
        x: i32,
        y: i32,
        w: u32,
        h: u32,
    },

    /// Set keyboard focus to a window (or clear focus if `None`).
    SetFocus { id: Option<WindowId> },

    /// Request that the backend close a window.
    RequestClose { id: WindowId },

    /// Notify the backend that a window's floating state changed.
    SetFloating { id: WindowId, floating: bool },

    /// The active workspace changed.
    WorkspaceChanged { active: Option<WorkspaceId> },

    /// The backend should spawn a child process.
    SpawnProcess { command: String },

    /// The backend should trigger a config reload.
    ReloadConfig,

    /// The compositor should exit.
    Exit,
}
