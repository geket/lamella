//! Input handling
//!
//! Manages keyboard and mouse input, including keybindings and gestures.

use std::collections::HashMap;

use bitflags::bitflags;
use serde::{Deserialize, Serialize};
use thiserror::Error;

use crate::config::BindingConfig;

/// Input handling errors
#[derive(Debug, Error)]
pub enum InputError {
    #[error("Invalid key: {0}")]
    Key(String),
    #[error("Invalid modifier: {0}")]
    Modifier(String),
    #[error("Invalid binding: {0}")]
    Binding(String),
}

bitflags! {
    /// Keyboard modifiers
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifiers: u8 {
        const SHIFT     = 0b0000_0001;
        const CTRL      = 0b0000_0010;
        const ALT       = 0b0000_0100;
        const SUPER     = 0b0000_1000;
        const CAPS_LOCK = 0b0001_0000;
        const NUM_LOCK  = 0b0010_0000;
    }
}

impl Modifiers {
    /// Parse modifiers from a string like "Mod4+Shift"
    pub fn from_str_list(s: &str) -> Self {
        let mut mods = Self::empty();

        for part in s.split('+') {
            let part = part.trim().to_lowercase();
            match part.as_str() {
                "shift" => mods.insert(Self::SHIFT),
                "ctrl" | "control" => mods.insert(Self::CTRL),
                "alt" | "mod1" => mods.insert(Self::ALT),
                "super" | "mod4" | "logo" | "win" => mods.insert(Self::SUPER),
                _ => {}, // Ignore unknown, might be key
            }
        }

        mods
    }
}

/// A key code
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // Letters
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // Numbers
    Key1,
    Key2,
    Key3,
    Key4,
    Key5,
    Key6,
    Key7,
    Key8,
    Key9,
    Key0,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Special keys
    Escape,
    Tab,
    Space,
    Return,
    Backspace,
    Delete,
    Insert,
    Home,
    End,
    PageUp,
    PageDown,
    Left,
    Right,
    Up,
    Down,

    // Punctuation
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Semicolon,
    Apostrophe,
    Grave,
    Backslash,
    Comma,
    Period,
    Slash,

    // Modifiers (as keys)
    ShiftL,
    ShiftR,
    CtrlL,
    CtrlR,
    AltL,
    AltR,
    SuperL,
    SuperR,

    // Media keys
    AudioMute,
    AudioLowerVolume,
    AudioRaiseVolume,
    AudioPlay,
    AudioPause,
    AudioStop,
    AudioPrev,
    AudioNext,

    // Other
    Print,
    ScrollLock,
    Pause,
    NumLock,
    CapsLock,

    // Numeric keypad
    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpDecimal,
    KpDivide,
    KpMultiply,
    KpSubtract,
    KpAdd,
    KpEnter,

    /// Unknown/unmapped key
    Unknown(u32),
}

impl KeyCode {
    /// Parse a key name to `KeyCode`
    pub fn from_name(name: &str) -> Result<Self, InputError> {
        let name_lower = name.to_lowercase();

        let key = match name_lower.as_str() {
            // Letters
            "a" => Self::A,
            "b" => Self::B,
            "c" => Self::C,
            "d" => Self::D,
            "e" => Self::E,
            "f" => Self::F,
            "g" => Self::G,
            "h" => Self::H,
            "i" => Self::I,
            "j" => Self::J,
            "k" => Self::K,
            "l" => Self::L,
            "m" => Self::M,
            "n" => Self::N,
            "o" => Self::O,
            "p" => Self::P,
            "q" => Self::Q,
            "r" => Self::R,
            "s" => Self::S,
            "t" => Self::T,
            "u" => Self::U,
            "v" => Self::V,
            "w" => Self::W,
            "x" => Self::X,
            "y" => Self::Y,
            "z" => Self::Z,

            // Numbers
            "1" | "key1" => Self::Key1,
            "2" | "key2" => Self::Key2,
            "3" | "key3" => Self::Key3,
            "4" | "key4" => Self::Key4,
            "5" | "key5" => Self::Key5,
            "6" | "key6" => Self::Key6,
            "7" | "key7" => Self::Key7,
            "8" | "key8" => Self::Key8,
            "9" | "key9" => Self::Key9,
            "0" | "key0" => Self::Key0,

            // Function keys
            "f1" => Self::F1,
            "f2" => Self::F2,
            "f3" => Self::F3,
            "f4" => Self::F4,
            "f5" => Self::F5,
            "f6" => Self::F6,
            "f7" => Self::F7,
            "f8" => Self::F8,
            "f9" => Self::F9,
            "f10" => Self::F10,
            "f11" => Self::F11,
            "f12" => Self::F12,

            // Special
            "escape" | "esc" => Self::Escape,
            "tab" => Self::Tab,
            "space" => Self::Space,
            "return" | "enter" => Self::Return,
            "backspace" => Self::Backspace,
            "delete" => Self::Delete,
            "insert" => Self::Insert,
            "home" => Self::Home,
            "end" => Self::End,
            "pageup" | "page_up" | "prior" => Self::PageUp,
            "pagedown" | "page_down" | "next" => Self::PageDown,
            "left" => Self::Left,
            "right" => Self::Right,
            "up" => Self::Up,
            "down" => Self::Down,

            // Punctuation
            "minus" | "-" => Self::Minus,
            "equal" | "=" => Self::Equal,
            "bracketleft" | "[" => Self::BracketLeft,
            "bracketright" | "]" => Self::BracketRight,
            "semicolon" | ";" => Self::Semicolon,
            "apostrophe" | "'" => Self::Apostrophe,
            "grave" | "`" => Self::Grave,
            "backslash" | "\\" => Self::Backslash,
            "comma" | "," => Self::Comma,
            "period" | "." => Self::Period,
            "slash" | "/" => Self::Slash,

            // Media
            "xf86audiomute" | "audiomute" => Self::AudioMute,
            "xf86audiolowervolume" | "audiolowervolume" => Self::AudioLowerVolume,
            "xf86audioraisevolume" | "audioraisevolume" => Self::AudioRaiseVolume,
            "xf86audioplay" | "audioplay" => Self::AudioPlay,
            "xf86audiopause" | "audiopause" => Self::AudioPause,
            "xf86audiostop" | "audiostop" => Self::AudioStop,
            "xf86audioprev" | "audioprev" => Self::AudioPrev,
            "xf86audionext" | "audionext" => Self::AudioNext,

            // Other
            "print" => Self::Print,
            "scroll_lock" | "scrolllock" => Self::ScrollLock,
            "pause" => Self::Pause,
            "num_lock" | "numlock" => Self::NumLock,
            "caps_lock" | "capslock" => Self::CapsLock,

            _ => return Err(InputError::Key(name.to_string())),
        };

        Ok(key)
    }
}

/// Mouse button
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    ScrollUp,
    ScrollDown,
    ScrollLeft,
    ScrollRight,
    Extra1,
    Extra2,
}

impl MouseButton {
    pub fn from_name(name: &str) -> Result<Self, InputError> {
        match name.to_lowercase().as_str() {
            "button1" | "left" | "lmb" => Ok(Self::Left),
            "button2" | "middle" | "mmb" => Ok(Self::Middle),
            "button3" | "right" | "rmb" => Ok(Self::Right),
            "button4" | "scrollup" => Ok(Self::ScrollUp),
            "button5" | "scrolldown" => Ok(Self::ScrollDown),
            "button6" | "scrollleft" => Ok(Self::ScrollLeft),
            "button7" | "scrollright" => Ok(Self::ScrollRight),
            "button8" | "extra1" => Ok(Self::Extra1),
            "button9" | "extra2" => Ok(Self::Extra2),
            _ => Err(InputError::Key(name.to_string())),
        }
    }
}

/// A key binding (modifiers + key)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyBinding {
    pub modifiers: Modifiers,
    pub key: KeyCode,
}

impl KeyBinding {
    pub const fn new(modifiers: Modifiers, key: KeyCode) -> Self {
        Self { modifiers, key }
    }

    /// Parse a binding string like "Mod4+Shift+Return"
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let parts: Vec<&str> = s.split('+').collect();

        if parts.is_empty() {
            return Err(InputError::Binding(s.to_string()));
        }

        let mut modifiers = Modifiers::empty();
        let mut key_part: Option<&str> = None;

        for part in &parts {
            let part = part.trim();
            match part.to_lowercase().as_str() {
                "shift" => modifiers.insert(Modifiers::SHIFT),
                "ctrl" | "control" => modifiers.insert(Modifiers::CTRL),
                "alt" | "mod1" => modifiers.insert(Modifiers::ALT),
                "super" | "mod4" | "logo" | "win" => modifiers.insert(Modifiers::SUPER),
                _ => {
                    // This should be the key
                    key_part = Some(part);
                },
            }
        }

        let key = match key_part {
            Some(k) => KeyCode::from_name(k)?,
            None => return Err(InputError::Binding(s.to_string())),
        };

        Ok(Self { modifiers, key })
    }
}

/// A mouse binding (modifiers + button)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MouseBinding {
    pub modifiers: Modifiers,
    pub button: MouseButton,
}

impl MouseBinding {
    pub fn parse(s: &str) -> Result<Self, InputError> {
        let parts: Vec<&str> = s.split('+').collect();

        if parts.is_empty() {
            return Err(InputError::Binding(s.to_string()));
        }

        let mut modifiers = Modifiers::empty();
        let mut button_part: Option<&str> = None;

        for part in &parts {
            let part = part.trim();
            match part.to_lowercase().as_str() {
                "shift" => modifiers.insert(Modifiers::SHIFT),
                "ctrl" | "control" => modifiers.insert(Modifiers::CTRL),
                "alt" | "mod1" => modifiers.insert(Modifiers::ALT),
                "super" | "mod4" | "logo" | "win" => modifiers.insert(Modifiers::SUPER),
                _ => {
                    button_part = Some(part);
                },
            }
        }

        let button = match button_part {
            Some(b) => MouseButton::from_name(b)?,
            None => return Err(InputError::Binding(s.to_string())),
        };

        Ok(Self { modifiers, button })
    }
}

/// Command to execute from a binding
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    // Execution
    Exec(String),
    ExecAlways(String),

    // Window management
    Kill,
    Focus(FocusTarget),
    Move(MoveTarget),
    Resize(ResizeDirection, i32),
    Floating(Toggle),
    Fullscreen(Toggle),
    Sticky(Toggle),

    // Layout
    Split(SplitCmd),
    Layout(LayoutCmd),

    // Workspace
    Workspace(WorkspaceTarget),
    MoveToWorkspace(WorkspaceTarget),

    // Scratchpad
    ScratchpadShow,
    MoveToScratchpad,

    // Marks
    Mark(String),
    Unmark(Option<String>),
    GotoMark(String),

    // Mode
    Mode(String),

    // System
    Reload,
    Restart,
    Exit,

    // Gaps
    Gaps(GapCmd),

    // Bar
    Bar(BarCmd),

    // Unknown command
    Unknown(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toggle {
    Enable,
    Disable,
    Switch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FocusTarget {
    Left,
    Right,
    Up,
    Down,
    Parent,
    Child,
    ModeToggle,
    Output(String),
    Workspace,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MoveTarget {
    Left,
    Right,
    Up,
    Down,
    Position(i32, i32),
    Center,
    Absolute(i32, i32),
    Output(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeDirection {
    Width(ResizeOp),
    Height(ResizeOp),
    Left,
    Right,
    Up,
    Down,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeOp {
    Grow,
    Shrink,
    Set,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitCmd {
    Horizontal,
    Vertical,
    Toggle,
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutCmd {
    Default,
    Tabbed,
    Stacked,
    SplitV,
    SplitH,
    Toggle,
    ToggleSplit,
    ToggleAll,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkspaceTarget {
    Name(String),
    Number(u32),
    Next,
    Prev,
    NextOnOutput,
    PrevOnOutput,
    BackAndForth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapCmd {
    Inner(GapOp),
    Outer(GapOp),
    Horizontal(GapOp),
    Vertical(GapOp),
    Top(GapOp),
    Right(GapOp),
    Bottom(GapOp),
    Left(GapOp),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GapOp {
    Set(i32),
    Plus(i32),
    Minus(i32),
    Toggle(i32),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarCmd {
    Mode(String),
    Hidden(Toggle),
}

impl Command {
    /// Parse a command string
    pub fn parse(s: &str) -> Self {
        let s = s.trim();
        let parts: Vec<&str> = s.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let args = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "exec" => Self::Exec(args.to_string()),
            "exec_always" => Self::ExecAlways(args.to_string()),

            "kill" => Self::Kill,

            "focus" => match args.to_lowercase().as_str() {
                "left" => Self::Focus(FocusTarget::Left),
                "right" => Self::Focus(FocusTarget::Right),
                "up" => Self::Focus(FocusTarget::Up),
                "down" => Self::Focus(FocusTarget::Down),
                "parent" => Self::Focus(FocusTarget::Parent),
                "child" => Self::Focus(FocusTarget::Child),
                "mode_toggle" => Self::Focus(FocusTarget::ModeToggle),
                _ => Self::Unknown(s.to_string()),
            },

            "move" => Self::parse_move(args),

            "floating" => match args.to_lowercase().as_str() {
                "enable" => Self::Floating(Toggle::Enable),
                "disable" => Self::Floating(Toggle::Disable),
                "toggle" | "" => Self::Floating(Toggle::Switch),
                _ => Self::Unknown(s.to_string()),
            },

            "fullscreen" => match args.to_lowercase().as_str() {
                "enable" => Self::Fullscreen(Toggle::Enable),
                "disable" => Self::Fullscreen(Toggle::Disable),
                "toggle" | "" => Self::Fullscreen(Toggle::Switch),
                _ => Self::Unknown(s.to_string()),
            },

            "split" => match args.to_lowercase().as_str() {
                "horizontal" | "h" => Self::Split(SplitCmd::Horizontal),
                "vertical" | "v" => Self::Split(SplitCmd::Vertical),
                "toggle" | "t" => Self::Split(SplitCmd::Toggle),
                "none" | "n" => Self::Split(SplitCmd::None),
                _ => Self::Unknown(s.to_string()),
            },

            "layout" => match args.to_lowercase().as_str() {
                "default" => Self::Layout(LayoutCmd::Default),
                "tabbed" => Self::Layout(LayoutCmd::Tabbed),
                "stacked" | "stacking" => Self::Layout(LayoutCmd::Stacked),
                "splitv" => Self::Layout(LayoutCmd::SplitV),
                "splith" => Self::Layout(LayoutCmd::SplitH),
                "toggle" => Self::Layout(LayoutCmd::Toggle),
                "toggle split" => Self::Layout(LayoutCmd::ToggleSplit),
                "toggle all" => Self::Layout(LayoutCmd::ToggleAll),
                _ => Self::Unknown(s.to_string()),
            },

            "workspace" => match args.to_lowercase().as_str() {
                "next" => Self::Workspace(WorkspaceTarget::Next),
                "prev" | "previous" => Self::Workspace(WorkspaceTarget::Prev),
                "next_on_output" => Self::Workspace(WorkspaceTarget::NextOnOutput),
                "prev_on_output" => Self::Workspace(WorkspaceTarget::PrevOnOutput),
                "back_and_forth" => Self::Workspace(WorkspaceTarget::BackAndForth),
                _ => {
                    if let Ok(num) = args.parse::<u32>() {
                        Self::Workspace(WorkspaceTarget::Number(num))
                    } else {
                        Self::Workspace(WorkspaceTarget::Name(args.to_string()))
                    }
                },
            },

            "scratchpad" => match args.to_lowercase().as_str() {
                "show" => Self::ScratchpadShow,
                _ => Self::Unknown(s.to_string()),
            },

            "mark" => Self::Mark(args.to_string()),
            "unmark" => Self::Unmark(if args.is_empty() {
                None
            } else {
                Some(args.to_string())
            }),

            "mode" => Self::Mode(args.to_string()),

            "reload" => Self::Reload,
            "restart" => Self::Restart,
            "exit" => Self::Exit,

            "resize" => Self::parse_resize(args),

            _ => Self::Unknown(s.to_string()),
        }
    }

    fn parse_move(args: &str) -> Self {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.is_empty() {
            return Self::Unknown(format!("move {args}"));
        }

        match parts[0].to_lowercase().as_str() {
            "left" => Self::Move(MoveTarget::Left),
            "right" => Self::Move(MoveTarget::Right),
            "up" => Self::Move(MoveTarget::Up),
            "down" => Self::Move(MoveTarget::Down),
            "center" => Self::Move(MoveTarget::Center),
            "scratchpad" => Self::MoveToScratchpad,
            "container" | "window" => {
                if parts.len() >= 4 && parts[1] == "to" && parts[2] == "workspace" {
                    let ws = parts[3..].join(" ");
                    if let Ok(num) = ws.parse::<u32>() {
                        Self::MoveToWorkspace(WorkspaceTarget::Number(num))
                    } else {
                        Self::MoveToWorkspace(WorkspaceTarget::Name(ws))
                    }
                } else {
                    Self::Unknown(format!("move {args}"))
                }
            },
            "position" => {
                if parts.len() >= 3 {
                    if let (Ok(x), Ok(y)) = (parts[1].parse::<i32>(), parts[2].parse::<i32>()) {
                        Self::Move(MoveTarget::Position(x, y))
                    } else {
                        Self::Unknown(format!("move {args}"))
                    }
                } else {
                    Self::Unknown(format!("move {args}"))
                }
            },
            _ => Self::Unknown(format!("move {args}")),
        }
    }

    fn parse_resize(args: &str) -> Self {
        let parts: Vec<&str> = args.split_whitespace().collect();

        if parts.len() < 2 {
            return Self::Unknown(format!("resize {args}"));
        }

        let op = match parts[0].to_lowercase().as_str() {
            "grow" => ResizeOp::Grow,
            "shrink" => ResizeOp::Shrink,
            "set" => ResizeOp::Set,
            _ => return Self::Unknown(format!("resize {args}")),
        };

        let direction = match parts[1].to_lowercase().as_str() {
            "width" => ResizeDirection::Width(op),
            "height" => ResizeDirection::Height(op),
            "left" => ResizeDirection::Left,
            "right" => ResizeDirection::Right,
            "up" => ResizeDirection::Up,
            "down" => ResizeDirection::Down,
            _ => return Self::Unknown(format!("resize {args}")),
        };

        let amount = if parts.len() >= 3 {
            parts[2].trim_end_matches("px").parse::<i32>().unwrap_or(10)
        } else {
            10
        };

        Self::Resize(direction, amount)
    }
}

/// Binding mode (like resize mode in i3)
#[derive(Debug, Clone)]
pub struct BindingMode {
    pub name: String,
    pub bindings: HashMap<KeyBinding, Command>,
    pub mouse_bindings: HashMap<MouseBinding, Command>,
}

impl BindingMode {
    pub fn new(name: String) -> Self {
        Self {
            name,
            bindings: HashMap::new(),
            mouse_bindings: HashMap::new(),
        }
    }

    pub fn add_binding(&mut self, binding: KeyBinding, command: Command) {
        self.bindings.insert(binding, command);
    }

    pub fn add_mouse_binding(&mut self, binding: MouseBinding, command: Command) {
        self.mouse_bindings.insert(binding, command);
    }
}

/// Input state manager
#[derive(Debug)]
pub struct InputManager {
    /// Current binding mode
    pub current_mode: String,
    /// All binding modes
    pub modes: HashMap<String, BindingMode>,
    /// Current modifier state
    pub modifiers: Modifiers,
    /// Currently pressed keys
    pub pressed_keys: Vec<KeyCode>,
    /// Currently pressed mouse buttons
    pub pressed_buttons: Vec<MouseButton>,
}

impl Default for InputManager {
    fn default() -> Self {
        Self::new()
    }
}

impl InputManager {
    pub fn new() -> Self {
        let mut modes = HashMap::new();
        modes.insert(
            "default".to_string(),
            BindingMode::new("default".to_string()),
        );

        Self {
            current_mode: "default".to_string(),
            modes,
            modifiers: Modifiers::empty(),
            pressed_keys: Vec::new(),
            pressed_buttons: Vec::new(),
        }
    }

    /// Load bindings from configuration
    pub fn load_bindings(&mut self, bindings: &[BindingConfig]) {
        for binding_config in bindings {
            if let Ok(key_binding) = KeyBinding::parse(&binding_config.keys) {
                let command = Command::parse(&binding_config.command);
                let mode_name = &binding_config.mode;

                // Get or create mode
                let mode = self
                    .modes
                    .entry(mode_name.clone())
                    .or_insert_with(|| BindingMode::new(mode_name.clone()));

                mode.add_binding(key_binding, command);
            }
        }
    }

    /// Handle a key press
    pub fn key_pressed(&mut self, key: KeyCode) -> Option<&Command> {
        // Update pressed keys
        if !self.pressed_keys.contains(&key) {
            self.pressed_keys.push(key);
        }

        // Check for binding
        let binding = KeyBinding::new(self.modifiers, key);
        self.modes.get(&self.current_mode)?.bindings.get(&binding)
    }

    /// Handle a raw keycode press (for compositor integration)
    pub fn key_pressed_raw(&mut self, keycode: u32) -> Option<&Command> {
        // Convert raw keycode to KeyCode
        // This is a simplified mapping - real implementation would use xkbcommon
        let key = match keycode {
            // Letters (assuming Linux keycodes)
            16 => KeyCode::Q,
            17 => KeyCode::W,
            18 => KeyCode::E,
            19 => KeyCode::R,
            20 => KeyCode::T,
            21 => KeyCode::Y,
            22 => KeyCode::U,
            23 => KeyCode::I,
            24 => KeyCode::O,
            25 => KeyCode::P,
            30 => KeyCode::A,
            31 => KeyCode::S,
            32 => KeyCode::D,
            33 => KeyCode::F,
            34 => KeyCode::G,
            35 => KeyCode::H,
            36 => KeyCode::J,
            37 => KeyCode::K,
            38 => KeyCode::L,
            44 => KeyCode::Z,
            45 => KeyCode::X,
            46 => KeyCode::C,
            47 => KeyCode::V,
            48 => KeyCode::B,
            49 => KeyCode::N,
            50 => KeyCode::M,
            // Numbers
            2 => KeyCode::Key1,
            3 => KeyCode::Key2,
            4 => KeyCode::Key3,
            5 => KeyCode::Key4,
            6 => KeyCode::Key5,
            7 => KeyCode::Key6,
            8 => KeyCode::Key7,
            9 => KeyCode::Key8,
            10 => KeyCode::Key9,
            11 => KeyCode::Key0,
            // Special keys
            1 => KeyCode::Escape,
            28 => KeyCode::Return,
            57 => KeyCode::Space,
            14 => KeyCode::Backspace,
            15 => KeyCode::Tab,
            // Arrow keys
            103 => KeyCode::Up,
            108 => KeyCode::Down,
            105 => KeyCode::Left,
            106 => KeyCode::Right,
            // Function keys
            59 => KeyCode::F1,
            60 => KeyCode::F2,
            61 => KeyCode::F3,
            62 => KeyCode::F4,
            63 => KeyCode::F5,
            64 => KeyCode::F6,
            65 => KeyCode::F7,
            66 => KeyCode::F8,
            67 => KeyCode::F9,
            68 => KeyCode::F10,
            87 => KeyCode::F11,
            88 => KeyCode::F12,
            _ => return None,
        };

        self.key_pressed(key)
    }

    /// Handle a key release
    pub fn key_released(&mut self, key: KeyCode) {
        self.pressed_keys.retain(|&k| k != key);
    }

    /// Update modifier state
    pub fn set_modifiers(&mut self, modifiers: Modifiers) {
        self.modifiers = modifiers;
    }

    /// Switch to a different binding mode
    pub fn set_mode(&mut self, mode: &str) {
        if self.modes.contains_key(mode) {
            self.current_mode = mode.to_string();
        }
    }

    /// Get current mode name
    pub fn current_mode(&self) -> &str {
        &self.current_mode
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_binding_parse() {
        let binding = KeyBinding::parse("Mod4+Return").unwrap();
        assert!(binding.modifiers.contains(Modifiers::SUPER));
        assert_eq!(binding.key, KeyCode::Return);

        let binding = KeyBinding::parse("Mod4+Shift+q").unwrap();
        assert!(binding.modifiers.contains(Modifiers::SUPER));
        assert!(binding.modifiers.contains(Modifiers::SHIFT));
        assert_eq!(binding.key, KeyCode::Q);
    }

    #[test]
    fn test_command_parse() {
        let cmd = Command::parse("exec alacritty");
        assert!(matches!(cmd, Command::Exec(s) if s == "alacritty"));

        let cmd = Command::parse("focus left");
        assert!(matches!(cmd, Command::Focus(FocusTarget::Left)));

        let cmd = Command::parse("workspace 3");
        assert!(matches!(
            cmd,
            Command::Workspace(WorkspaceTarget::Number(3))
        ));
    }

    #[test]
    fn test_modifiers() {
        let mods = Modifiers::from_str_list("Mod4+Shift");
        assert!(mods.contains(Modifiers::SUPER));
        assert!(mods.contains(Modifiers::SHIFT));
        assert!(!mods.contains(Modifiers::CTRL));
    }
}
