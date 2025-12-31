//! Configuration system
//!
//! Provides a flexible configuration system inspired by i3/Sway config
//! with TOML file format for better maintainability.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use tracing::{info, warn};

use crate::input::KeyBinding;
use crate::layout::{LayoutMode, SplitDirection};
use crate::window::{BorderStyle, DecorationMode, WindowCriteria};

/// Main configuration structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// General settings
    pub general: GeneralConfig,

    /// Gap settings (inner and outer)
    pub gaps: GapConfig,

    /// Border settings
    pub border: BorderConfig,

    /// Color scheme
    pub colors: ColorConfig,

    /// Font configuration
    pub font: FontConfig,

    /// Input device configuration
    pub input: InputConfig,

    /// Output (monitor) configuration
    #[serde(default)]
    pub outputs: Vec<OutputConfig>,

    /// Workspace configuration
    #[serde(default)]
    pub workspaces: Vec<WorkspaceConfigEntry>,

    /// Key bindings
    #[serde(default)]
    pub bindings: Vec<BindingConfig>,

    /// Mouse bindings
    #[serde(default)]
    pub mouse_bindings: Vec<MouseBindingConfig>,

    /// Window rules
    #[serde(default)]
    pub rules: Vec<WindowRule>,

    /// Startup commands
    #[serde(default)]
    pub startup: Vec<StartupCommand>,

    /// Bar configuration
    pub bar: BarConfig,

    /// Animation settings
    pub animations: AnimationConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig::default(),
            gaps: GapConfig::default(),
            border: BorderConfig::default(),
            colors: ColorConfig::default(),
            font: FontConfig::default(),
            input: InputConfig::default(),
            outputs: Vec::new(),
            workspaces: Vec::new(),
            bindings: default_bindings(),
            mouse_bindings: default_mouse_bindings(),
            rules: Vec::new(),
            startup: Vec::new(),
            bar: BarConfig::default(),
            animations: AnimationConfig::default(),
        }
    }
}

impl Config {
    /// Load configuration from file
    pub fn load(path: Option<&str>) -> Result<Self> {
        let config_path = path.map(PathBuf::from).or_else(Self::find_config_file);

        match config_path {
            Some(path) if path.exists() => {
                info!("Loading configuration from {:?}", path);
                let content = fs::read_to_string(&path)
                    .with_context(|| format!("Failed to read config file: {:?}", path))?;

                let config: Config = toml::from_str(&content)
                    .with_context(|| format!("Failed to parse config file: {:?}", path))?;

                Ok(config)
            },
            Some(path) => {
                warn!("Config file not found at {:?}, using defaults", path);
                Ok(Self::default())
            },
            None => {
                info!("No config file found, using defaults");
                Ok(Self::default())
            },
        }
    }

    /// Find the configuration file
    fn find_config_file() -> Option<PathBuf> {
        // Check in order of preference
        let candidates = [
            // XDG config
            dirs::config_dir().map(|p| p.join("fluxway/config.toml")),
            // Home directory
            dirs::home_dir().map(|p| p.join(".config/fluxway/config.toml")),
            // Legacy locations
            dirs::home_dir().map(|p| p.join(".fluxway/config.toml")),
            // System-wide
            Some(PathBuf::from("/etc/fluxway/config.toml")),
        ];

        candidates.into_iter().flatten().find(|p| p.exists())
    }

    /// Generate default configuration as a string
    pub fn default_config_string() -> String {
        let config = Self::default();
        toml::to_string_pretty(&config)
            .unwrap_or_else(|_| String::from("# Error generating config"))
    }

    /// Get the socket path
    pub fn socket_path(&self) -> PathBuf {
        if let Some(ref path) = self.general.socket_path {
            PathBuf::from(path)
        } else {
            let runtime_dir =
                std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(runtime_dir).join("fluxway.sock")
        }
    }
}

/// General settings
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GeneralConfig {
    /// Focus follows mouse
    pub focus_follows_mouse: FocusFollowsMouse,
    /// Mouse warping behavior
    pub mouse_warping: MouseWarping,
    /// Workspace back and forth
    pub workspace_back_and_forth: bool,
    /// Auto back and forth
    pub workspace_auto_back_and_forth: bool,
    /// Force xwayland
    pub xwayland: XWaylandMode,
    /// Default layout mode
    pub default_layout: LayoutMode,
    /// Default split direction
    pub default_orientation: Orientation,
    /// Modifier key for floating window drag
    pub floating_modifier: String,
    /// Socket path for IPC
    pub socket_path: Option<String>,
    /// Popup during fullscreen behavior
    pub popup_during_fullscreen: PopupDuringFullscreen,
    /// Focus wrapping
    pub focus_wrapping: FocusWrapping,
    /// Smart gaps
    pub smart_gaps: bool,
    /// Smart borders
    pub smart_borders: SmartBorders,
    /// Hide edge borders
    pub hide_edge_borders: HideEdgeBorders,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            focus_follows_mouse: FocusFollowsMouse::Yes,
            mouse_warping: MouseWarping::Output,
            workspace_back_and_forth: false,
            workspace_auto_back_and_forth: false,
            xwayland: XWaylandMode::Enable,
            default_layout: LayoutMode::Split,
            default_orientation: Orientation::Auto,
            floating_modifier: "Mod4".to_string(),
            socket_path: None,
            popup_during_fullscreen: PopupDuringFullscreen::Smart,
            focus_wrapping: FocusWrapping::Yes,
            smart_gaps: false,
            smart_borders: SmartBorders::Off,
            hide_edge_borders: HideEdgeBorders::None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FocusFollowsMouse {
    #[default]
    Yes,
    No,
    Always,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum MouseWarping {
    #[default]
    Output,
    Container,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum XWaylandMode {
    #[default]
    Enable,
    Disable,
    Force,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Orientation {
    Horizontal,
    Vertical,
    #[default]
    Auto,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PopupDuringFullscreen {
    #[default]
    Smart,
    Ignore,
    Leave,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FocusWrapping {
    #[default]
    Yes,
    No,
    Force,
    Workspace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SmartBorders {
    #[default]
    Off,
    On,
    NoGaps,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum HideEdgeBorders {
    #[default]
    None,
    Vertical,
    Horizontal,
    Both,
    Smart,
    SmartNoGaps,
}

/// Gap configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct GapConfig {
    /// Inner gaps (between windows)
    pub inner: u32,
    /// Outer gaps (between windows and screen edge)
    pub outer: u32,
    /// Top gap
    pub top: Option<u32>,
    /// Bottom gap
    pub bottom: Option<u32>,
    /// Left gap
    pub left: Option<u32>,
    /// Right gap
    pub right: Option<u32>,
}

impl Default for GapConfig {
    fn default() -> Self {
        Self {
            inner: 4,
            outer: 4,
            top: None,
            bottom: None,
            left: None,
            right: None,
        }
    }
}

/// Border configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BorderConfig {
    /// Default border width
    pub width: u32,
    /// Border style for new windows
    pub style: BorderStyle,
    /// Border style for floating windows
    pub floating_style: BorderStyle,
}

impl Default for BorderConfig {
    fn default() -> Self {
        Self {
            width: 2,
            style: BorderStyle::Pixel(2),
            floating_style: BorderStyle::Normal,
        }
    }
}

/// Color configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ColorConfig {
    /// Focused window
    pub focused: WindowColors,
    /// Focused inactive window
    pub focused_inactive: WindowColors,
    /// Unfocused window
    pub unfocused: WindowColors,
    /// Urgent window
    pub urgent: WindowColors,
    /// Background color
    pub background: String,
}

impl Default for ColorConfig {
    fn default() -> Self {
        Self {
            focused: WindowColors {
                border: "#4c7899".to_string(),
                background: "#285577".to_string(),
                text: "#ffffff".to_string(),
                indicator: "#2e9ef4".to_string(),
                child_border: "#285577".to_string(),
            },
            focused_inactive: WindowColors {
                border: "#333333".to_string(),
                background: "#5f676a".to_string(),
                text: "#ffffff".to_string(),
                indicator: "#484e50".to_string(),
                child_border: "#5f676a".to_string(),
            },
            unfocused: WindowColors {
                border: "#333333".to_string(),
                background: "#222222".to_string(),
                text: "#888888".to_string(),
                indicator: "#292d2e".to_string(),
                child_border: "#222222".to_string(),
            },
            urgent: WindowColors {
                border: "#2f343a".to_string(),
                background: "#900000".to_string(),
                text: "#ffffff".to_string(),
                indicator: "#900000".to_string(),
                child_border: "#900000".to_string(),
            },
            background: "#000000".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowColors {
    pub border: String,
    pub background: String,
    pub text: String,
    pub indicator: String,
    pub child_border: String,
}

/// Font configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct FontConfig {
    /// Font family
    pub family: String,
    /// Font size
    pub size: f32,
    /// Font style
    pub style: String,
}

impl Default for FontConfig {
    fn default() -> Self {
        Self {
            family: "monospace".to_string(),
            size: 10.0,
            style: "Regular".to_string(),
        }
    }
}

/// Input device configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    /// Keyboard repeat delay (ms)
    pub repeat_delay: u32,
    /// Keyboard repeat rate (per second)
    pub repeat_rate: u32,
    /// Keyboard layout
    pub xkb_layout: String,
    /// Keyboard variant
    pub xkb_variant: String,
    /// Keyboard options
    pub xkb_options: String,
    /// Natural scrolling
    pub natural_scroll: bool,
    /// Tap to click
    pub tap: bool,
    /// Drag lock
    pub drag_lock: bool,
    /// Pointer acceleration
    pub accel_profile: AccelProfile,
    /// Pointer speed
    pub pointer_speed: f64,
    /// Scroll factor
    pub scroll_factor: f64,
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            repeat_delay: 300,
            repeat_rate: 30,
            xkb_layout: "us".to_string(),
            xkb_variant: String::new(),
            xkb_options: String::new(),
            natural_scroll: false,
            tap: true,
            drag_lock: false,
            accel_profile: AccelProfile::Adaptive,
            pointer_speed: 0.0,
            scroll_factor: 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccelProfile {
    #[default]
    Adaptive,
    Flat,
}

/// Output (monitor) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputConfig {
    /// Output name (e.g., "eDP-1", "HDMI-A-1")
    pub name: String,
    /// Resolution (e.g., "1920x1080")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution: Option<String>,
    /// Refresh rate
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh: Option<f32>,
    /// Position
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<(i32, i32)>,
    /// Scale factor
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scale: Option<f64>,
    /// Transform (rotation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transform: Option<Transform>,
    /// Background/wallpaper
    #[serde(skip_serializing_if = "Option::is_none")]
    pub background: Option<BackgroundConfig>,
    /// Disable output
    #[serde(default)]
    pub disable: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Transform {
    Normal,
    #[serde(rename = "90")]
    Rotate90,
    #[serde(rename = "180")]
    Rotate180,
    #[serde(rename = "270")]
    Rotate270,
    Flipped,
    #[serde(rename = "flipped-90")]
    Flipped90,
    #[serde(rename = "flipped-180")]
    Flipped180,
    #[serde(rename = "flipped-270")]
    Flipped270,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackgroundConfig {
    /// Path to image or solid color
    pub source: String,
    /// Fill mode
    #[serde(default)]
    pub mode: BackgroundMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BackgroundMode {
    #[default]
    Fill,
    Fit,
    Stretch,
    Center,
    Tile,
    Solid,
}

/// Workspace configuration entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfigEntry {
    /// Workspace name/number
    pub name: String,
    /// Assign to specific output
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Custom gaps
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<u32>,
}

/// Key binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BindingConfig {
    /// Key combination (e.g., "Mod4+Return")
    pub keys: String,
    /// Command to execute
    pub command: String,
    /// Binding mode (default is "default")
    #[serde(default = "default_mode")]
    pub mode: String,
}

fn default_mode() -> String {
    "default".to_string()
}

/// Mouse binding configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MouseBindingConfig {
    /// Button and modifiers
    pub button: String,
    /// Command to execute
    pub command: String,
}

/// Window rule
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowRule {
    /// Matching criteria
    pub criteria: WindowCriteria,
    /// Commands to execute
    pub commands: Vec<String>,
}

/// Startup command
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartupCommand {
    /// Command to run
    pub command: String,
    /// Run always or only once
    #[serde(default)]
    pub always: bool,
}

/// Bar configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    /// Enable built-in bar
    pub enabled: bool,
    /// Position
    pub position: BarPosition,
    /// Height
    pub height: u32,
    /// Status command
    pub status_command: Option<String>,
    /// Font
    pub font: Option<String>,
    /// Colors
    pub colors: BarColors,
    /// Show workspace buttons
    pub workspace_buttons: bool,
    /// Binding mode indicator
    pub mode_indicator: bool,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            position: BarPosition::Bottom,
            height: 24,
            status_command: None,
            font: None,
            colors: BarColors::default(),
            workspace_buttons: true,
            mode_indicator: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
    Top,
    #[default]
    Bottom,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BarColors {
    pub background: String,
    pub statusline: String,
    pub separator: String,
    pub focused_workspace: BarWorkspaceColors,
    pub active_workspace: BarWorkspaceColors,
    pub inactive_workspace: BarWorkspaceColors,
    pub urgent_workspace: BarWorkspaceColors,
}

impl Default for BarColors {
    fn default() -> Self {
        Self {
            background: "#000000".to_string(),
            statusline: "#ffffff".to_string(),
            separator: "#666666".to_string(),
            focused_workspace: BarWorkspaceColors {
                border: "#4c7899".to_string(),
                background: "#285577".to_string(),
                text: "#ffffff".to_string(),
            },
            active_workspace: BarWorkspaceColors {
                border: "#333333".to_string(),
                background: "#5f676a".to_string(),
                text: "#ffffff".to_string(),
            },
            inactive_workspace: BarWorkspaceColors {
                border: "#333333".to_string(),
                background: "#222222".to_string(),
                text: "#888888".to_string(),
            },
            urgent_workspace: BarWorkspaceColors {
                border: "#2f343a".to_string(),
                background: "#900000".to_string(),
                text: "#ffffff".to_string(),
            },
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BarWorkspaceColors {
    pub border: String,
    pub background: String,
    pub text: String,
}

/// Animation configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AnimationConfig {
    /// Enable animations
    pub enabled: bool,
    /// Animation duration in ms
    pub duration: u32,
    /// Animation curve
    pub curve: AnimationCurve,
}

impl Default for AnimationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            duration: 200,
            curve: AnimationCurve::EaseOutCubic,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum AnimationCurve {
    Linear,
    #[default]
    EaseOutCubic,
    EaseOutQuad,
    EaseInOutCubic,
}

/// Generate default key bindings (i3-style)
fn default_bindings() -> Vec<BindingConfig> {
    vec![
        // Launch terminal
        BindingConfig {
            keys: "Mod4+Return".to_string(),
            command: "exec alacritty".to_string(),
            mode: "default".to_string(),
        },
        // Kill focused window
        BindingConfig {
            keys: "Mod4+Shift+q".to_string(),
            command: "kill".to_string(),
            mode: "default".to_string(),
        },
        // Application launcher
        BindingConfig {
            keys: "Mod4+d".to_string(),
            command: "exec wofi --show drun".to_string(),
            mode: "default".to_string(),
        },
        // Focus movement
        BindingConfig {
            keys: "Mod4+h".to_string(),
            command: "focus left".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+j".to_string(),
            command: "focus down".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+k".to_string(),
            command: "focus up".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+l".to_string(),
            command: "focus right".to_string(),
            mode: "default".to_string(),
        },
        // Move windows
        BindingConfig {
            keys: "Mod4+Shift+h".to_string(),
            command: "move left".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+j".to_string(),
            command: "move down".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+k".to_string(),
            command: "move up".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+l".to_string(),
            command: "move right".to_string(),
            mode: "default".to_string(),
        },
        // Split orientation
        BindingConfig {
            keys: "Mod4+b".to_string(),
            command: "split horizontal".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+v".to_string(),
            command: "split vertical".to_string(),
            mode: "default".to_string(),
        },
        // Fullscreen
        BindingConfig {
            keys: "Mod4+f".to_string(),
            command: "fullscreen toggle".to_string(),
            mode: "default".to_string(),
        },
        // Floating
        BindingConfig {
            keys: "Mod4+Shift+space".to_string(),
            command: "floating toggle".to_string(),
            mode: "default".to_string(),
        },
        // Focus floating/tiling
        BindingConfig {
            keys: "Mod4+space".to_string(),
            command: "focus mode_toggle".to_string(),
            mode: "default".to_string(),
        },
        // Layout modes (Fluxbox-style tabbing)
        BindingConfig {
            keys: "Mod4+s".to_string(),
            command: "layout stacked".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+w".to_string(),
            command: "layout tabbed".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+e".to_string(),
            command: "layout toggle split".to_string(),
            mode: "default".to_string(),
        },
        // Workspaces
        BindingConfig {
            keys: "Mod4+1".to_string(),
            command: "workspace 1".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+2".to_string(),
            command: "workspace 2".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+3".to_string(),
            command: "workspace 3".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+4".to_string(),
            command: "workspace 4".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+5".to_string(),
            command: "workspace 5".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+6".to_string(),
            command: "workspace 6".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+7".to_string(),
            command: "workspace 7".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+8".to_string(),
            command: "workspace 8".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+9".to_string(),
            command: "workspace 9".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+0".to_string(),
            command: "workspace 10".to_string(),
            mode: "default".to_string(),
        },
        // Move to workspace
        BindingConfig {
            keys: "Mod4+Shift+1".to_string(),
            command: "move container to workspace 1".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+2".to_string(),
            command: "move container to workspace 2".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+3".to_string(),
            command: "move container to workspace 3".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+4".to_string(),
            command: "move container to workspace 4".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+5".to_string(),
            command: "move container to workspace 5".to_string(),
            mode: "default".to_string(),
        },
        // Scratchpad
        BindingConfig {
            keys: "Mod4+Shift+minus".to_string(),
            command: "move scratchpad".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+minus".to_string(),
            command: "scratchpad show".to_string(),
            mode: "default".to_string(),
        },
        // Reload/exit
        BindingConfig {
            keys: "Mod4+Shift+c".to_string(),
            command: "reload".to_string(),
            mode: "default".to_string(),
        },
        BindingConfig {
            keys: "Mod4+Shift+e".to_string(),
            command: "exit".to_string(),
            mode: "default".to_string(),
        },
        // Resize mode
        BindingConfig {
            keys: "Mod4+r".to_string(),
            command: "mode resize".to_string(),
            mode: "default".to_string(),
        },
        // Resize mode bindings
        BindingConfig {
            keys: "h".to_string(),
            command: "resize shrink width 10 px".to_string(),
            mode: "resize".to_string(),
        },
        BindingConfig {
            keys: "j".to_string(),
            command: "resize grow height 10 px".to_string(),
            mode: "resize".to_string(),
        },
        BindingConfig {
            keys: "k".to_string(),
            command: "resize shrink height 10 px".to_string(),
            mode: "resize".to_string(),
        },
        BindingConfig {
            keys: "l".to_string(),
            command: "resize grow width 10 px".to_string(),
            mode: "resize".to_string(),
        },
        BindingConfig {
            keys: "Escape".to_string(),
            command: "mode default".to_string(),
            mode: "resize".to_string(),
        },
        BindingConfig {
            keys: "Return".to_string(),
            command: "mode default".to_string(),
            mode: "resize".to_string(),
        },
    ]
}

/// Generate default mouse bindings
fn default_mouse_bindings() -> Vec<MouseBindingConfig> {
    vec![
        MouseBindingConfig {
            button: "Mod4+button1".to_string(),
            command: "move".to_string(),
        },
        MouseBindingConfig {
            button: "Mod4+button3".to_string(),
            command: "resize".to_string(),
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(config.gaps.inner == 4);
        assert!(!config.bindings.is_empty());
    }

    #[test]
    fn test_config_serialization() {
        let config = Config::default();
        let toml_str = toml::to_string_pretty(&config).unwrap();
        let parsed: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.gaps.inner, config.gaps.inner);
    }
}
