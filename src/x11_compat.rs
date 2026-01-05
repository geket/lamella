//! X11 Compatibility Module
//!
//! This module provides backwards compatibility with X11/Xorg.

/// X11 atom names commonly used by window managers
pub mod atoms {
    pub const WM_PROTOCOLS: &str = "WM_PROTOCOLS";
    pub const WM_DELETE_WINDOW: &str = "WM_DELETE_WINDOW";
    pub const WM_STATE: &str = "WM_STATE";
    pub const WM_CLASS: &str = "WM_CLASS";
    pub const WM_NAME: &str = "WM_NAME";

    pub const NET_SUPPORTED: &str = "_NET_SUPPORTED";
    pub const NET_WM_NAME: &str = "_NET_WM_NAME";
    pub const NET_WM_STATE: &str = "_NET_WM_STATE";
    pub const NET_WM_STATE_FULLSCREEN: &str = "_NET_WM_STATE_FULLSCREEN";
    pub const NET_WM_STATE_MAXIMIZED_VERT: &str = "_NET_WM_STATE_MAXIMIZED_VERT";
    pub const NET_WM_STATE_MAXIMIZED_HORZ: &str = "_NET_WM_STATE_MAXIMIZED_HORZ";
    pub const NET_WM_STATE_HIDDEN: &str = "_NET_WM_STATE_HIDDEN";
    pub const NET_WM_WINDOW_TYPE: &str = "_NET_WM_WINDOW_TYPE";
    pub const NET_WM_WINDOW_TYPE_NORMAL: &str = "_NET_WM_WINDOW_TYPE_NORMAL";
    pub const NET_WM_WINDOW_TYPE_DIALOG: &str = "_NET_WM_WINDOW_TYPE_DIALOG";
    pub const NET_ACTIVE_WINDOW: &str = "_NET_ACTIVE_WINDOW";
    pub const NET_CURRENT_DESKTOP: &str = "_NET_CURRENT_DESKTOP";
}

/// Display server backend trait
pub trait DisplayBackend {
    fn init(&mut self) -> Result<(), BackendError>;
    fn run(&mut self) -> Result<(), BackendError>;
    fn shutdown(&mut self) -> Result<(), BackendError>;
    fn backend_type(&self) -> BackendType;
}

/// Backend type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendType {
    Wayland,
    WaylandWithXWayland,
    X11Native,
}

/// Backend error type
#[derive(Debug, thiserror::Error)]
pub enum BackendError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("X11 error: {0}")]
    X11Error(String),
    #[error("Wayland error: {0}")]
    WaylandError(String),
    #[error("Feature not available: {0}")]
    FeatureNotAvailable(String),
}

/// Check if running under X11
pub fn is_x11_session() -> bool {
    std::env::var("DISPLAY").is_ok() && std::env::var("WAYLAND_DISPLAY").is_err()
}

/// Check if running under Wayland
pub fn is_wayland_session() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
}

/// Detect the current session type
pub fn detect_session_type() -> SessionType {
    if is_wayland_session() {
        SessionType::Wayland
    } else if is_x11_session() {
        SessionType::X11
    } else {
        SessionType::Tty
    }
}

/// Session type enumeration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionType {
    Wayland,
    X11,
    Tty,
}

impl std::fmt::Display for SessionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Wayland => write!(f, "Wayland"),
            Self::X11 => write!(f, "X11"),
            Self::Tty => write!(f, "TTY"),
        }
    }
}

#[cfg(feature = "x11")]
pub mod x11_backend {
    use super::{BackendError, BackendType, DisplayBackend};

    #[derive(Default)]
    pub struct X11Backend {
        _placeholder: (),
    }

    impl X11Backend {
        pub fn new() -> Result<Self, BackendError> {
            Ok(Self { _placeholder: () })
        }
    }

    impl DisplayBackend for X11Backend {
        fn init(&mut self) -> Result<(), BackendError> {
            tracing::info!("Initializing X11 backend");
            Ok(())
        }

        fn run(&mut self) -> Result<(), BackendError> {
            tracing::info!("Running X11 event loop");
            Ok(())
        }

        fn shutdown(&mut self) -> Result<(), BackendError> {
            tracing::info!("Shutting down X11 backend");
            Ok(())
        }

        fn backend_type(&self) -> BackendType {
            BackendType::X11Native
        }
    }
}

#[cfg(feature = "xwayland")]
pub mod xwayland_bridge {
    use super::BackendError;

    pub struct XWaylandBridge {
        _process: Option<std::process::Child>,
        display_number: Option<u32>,
    }

    impl XWaylandBridge {
        pub const fn new() -> Self {
            Self {
                _process: None,
                display_number: None,
            }
        }

        pub fn start(&mut self) -> Result<(), BackendError> {
            tracing::info!("Starting XWayland server");
            let display_num = Self::find_free_display()?;
            self.display_number = Some(display_num);
            Ok(())
        }

        pub fn stop(&mut self) {
            if let Some(ref mut process) = self._process {
                let _ = process.kill();
            }
            self._process = None;
            self.display_number = None;
        }

        pub fn display(&self) -> Option<String> {
            self.display_number.map(|n| format!(":{n}"))
        }

        fn find_free_display() -> Result<u32, BackendError> {
            for i in 0..64 {
                let socket_path = format!("/tmp/.X11-unix/X{i}");
                if !std::path::Path::new(&socket_path).exists() {
                    return Ok(i);
                }
            }
            Err(BackendError::X11Error("No free X11 display".to_string()))
        }
    }

    impl Default for XWaylandBridge {
        fn default() -> Self {
            Self::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_detection() {
        let session = detect_session_type();
        println!("Detected session type: {session}");
    }

    #[test]
    fn test_atom_names() {
        assert_eq!(atoms::WM_PROTOCOLS, "WM_PROTOCOLS");
        assert_eq!(atoms::NET_WM_STATE_FULLSCREEN, "_NET_WM_STATE_FULLSCREEN");
    }
}
