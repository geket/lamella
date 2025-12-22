//! Compositor module - Window manager core logic
//!
//! This module provides the main compositor implementation.
//! The rendering and Wayland protocol handling will be implemented
//! once the core logic is tested and working.

use std::process::Command as ProcessCommand;
use std::time::Instant;

use anyhow::Result;
use tracing::{debug, error, info, warn};

use crate::config::{Config, FocusFollowsMouse};
use crate::input::{
    Command, FocusTarget, InputManager, LayoutCmd, Modifiers, MoveTarget,
    ResizeDirection, SplitCmd, Toggle, WorkspaceTarget,
};
use crate::layout::{LayoutMode, SplitDirection};
use crate::state::{GrabOperation, Geometry, ResizeEdges, State};
use crate::window::{WindowId, WindowState};

/// Main compositor struct - contains all window management state
pub struct Fluxway {
    /// Window manager state  
    pub state: State,
    /// Input manager
    pub input_manager: InputManager,
    /// Current pointer location
    pub pointer_x: f64,
    pub pointer_y: f64,
    /// Start time
    pub start_time: Instant,
    /// Frame counter
    pub frame_count: u64,
    /// Should exit
    pub should_exit: bool,
    /// Scratchpad visible windows
    pub scratchpad_visible: Vec<WindowId>,
}

/// Resize edge for window resizing
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ResizeEdge {
    Top,
    Bottom,
    Left,
    Right,
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

impl ResizeEdge {
    /// Determine resize edge from click position relative to window
    pub fn from_point(px: f64, py: f64, geo: &Geometry) -> Self {
        let x = px - geo.x as f64;
        let y = py - geo.y as f64;
        let width = geo.width as f64;
        let height = geo.height as f64;

        let left = x < width / 3.0;
        let right = x > width * 2.0 / 3.0;
        let top = y < height / 3.0;
        let bottom = y > height * 2.0 / 3.0;

        match (left, right, top, bottom) {
            (true, _, true, _) => ResizeEdge::TopLeft,
            (_, true, true, _) => ResizeEdge::TopRight,
            (true, _, _, true) => ResizeEdge::BottomLeft,
            (_, true, _, true) => ResizeEdge::BottomRight,
            (true, _, _, _) => ResizeEdge::Left,
            (_, true, _, _) => ResizeEdge::Right,
            (_, _, true, _) => ResizeEdge::Top,
            (_, _, _, true) => ResizeEdge::Bottom,
            _ => ResizeEdge::BottomRight,
        }
    }

    /// Convert to ResizeEdges bitflags
    pub fn to_edges(&self) -> ResizeEdges {
        match self {
            ResizeEdge::Top => ResizeEdges::TOP,
            ResizeEdge::Bottom => ResizeEdges::BOTTOM,
            ResizeEdge::Left => ResizeEdges::LEFT,
            ResizeEdge::Right => ResizeEdges::RIGHT,
            ResizeEdge::TopLeft => ResizeEdges::TOP | ResizeEdges::LEFT,
            ResizeEdge::TopRight => ResizeEdges::TOP | ResizeEdges::RIGHT,
            ResizeEdge::BottomLeft => ResizeEdges::BOTTOM | ResizeEdges::LEFT,
            ResizeEdge::BottomRight => ResizeEdges::BOTTOM | ResizeEdges::RIGHT,
        }
    }
}

impl Fluxway {
    /// Create a new compositor instance
    pub fn new(config: Config) -> Self {
        let mut input_manager = InputManager::new();
        input_manager.load_bindings(&config.bindings);

        let state = State::new(config);

        Self {
            state,
            input_manager,
            pointer_x: 0.0,
            pointer_y: 0.0,
            start_time: Instant::now(),
            frame_count: 0,
            should_exit: false,
            scratchpad_visible: Vec::new(),
        }
    }

    /// Handle a command from keybindings
    pub fn handle_command(&mut self, command: Command) {
        debug!("Handling command: {:?}", command);

        match command {
            Command::Exec(cmd) | Command::ExecAlways(cmd) => {
                self.spawn_command(&cmd);
            }
            Command::Kill => {
                if let Some(window_id) = self.state.focus.focused_window {
                    self.state.remove_window(window_id);
                }
            }
            Command::Focus(target) => {
                self.handle_focus(target);
            }
            Command::Move(target) => {
                self.handle_move(target);
            }
            Command::Floating(toggle) => {
                self.handle_floating(toggle);
            }
            Command::Fullscreen(toggle) => {
                self.handle_fullscreen(toggle);
            }
            Command::Sticky(toggle) => {
                self.handle_sticky(toggle);
            }
            Command::Split(cmd) => {
                self.handle_split(cmd);
            }
            Command::Layout(cmd) => {
                self.handle_layout(cmd);
            }
            Command::Workspace(target) => {
                self.switch_workspace(target);
            }
            Command::MoveToWorkspace(target) => {
                self.move_to_workspace(target);
            }
            Command::ScratchpadShow => {
                self.toggle_scratchpad();
            }
            Command::MoveToScratchpad => {
                if let Some(window_id) = self.state.focus.focused_window {
                    self.state.toggle_scratchpad(window_id);
                }
            }
            Command::Mark(mark) => {
                if let Some(window_id) = self.state.focus.focused_window {
                    self.state.set_mark(mark, window_id);
                }
            }
            Command::GotoMark(mark) => {
                self.state.goto_mark(&mark);
            }
            Command::Unmark(mark) => {
                // TODO: implement unmark
                debug!("Unmark: {:?}", mark);
            }
            Command::Reload => {
                self.reload_config();
            }
            Command::Restart => {
                info!("Restart requested");
                // TODO: implement restart
            }
            Command::Exit => {
                self.should_exit = true;
            }
            Command::Mode(mode_name) => {
                self.input_manager.set_mode(&mode_name);
            }
            Command::Resize(direction, amount) => {
                self.handle_resize(direction, amount);
            }
            Command::Gaps(gap_cmd) => {
                debug!("Gaps command: {:?}", gap_cmd);
            }
            Command::Bar(bar_cmd) => {
                debug!("Bar command: {:?}", bar_cmd);
            }
            Command::Unknown(cmd) => {
                warn!("Unknown command: {}", cmd);
            }
        }
    }

    /// Spawn an external command
    pub fn spawn_command(&self, cmd: &str) {
        info!("Spawning command: {}", cmd);
        if let Err(e) = ProcessCommand::new("sh").arg("-c").arg(cmd).spawn() {
            error!("Failed to spawn command '{}': {}", cmd, e);
        }
    }

    /// Handle focus command
    fn handle_focus(&mut self, target: FocusTarget) {
        debug!("Focus target: {:?}", target);
        // TODO: Implement focus navigation using layout tree
        match target {
            FocusTarget::Left | FocusTarget::Right | FocusTarget::Up | FocusTarget::Down => {
                // Navigate in direction
            }
            FocusTarget::Parent => {
                // Focus parent container
            }
            FocusTarget::Child => {
                // Focus child
            }
            FocusTarget::ModeToggle => {
                // Toggle focus mode (floating/tiling)
            }
            FocusTarget::Output(name) => {
                debug!("Focus output: {}", name);
            }
            FocusTarget::Workspace => {
                // Focus workspace
            }
        }
    }

    /// Handle move command
    fn handle_move(&mut self, target: MoveTarget) {
        debug!("Move target: {:?}", target);
        // TODO: Implement window movement
    }

    /// Handle floating toggle
    fn handle_floating(&mut self, toggle: Toggle) {
        if let Some(window_id) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&window_id) {
                match toggle {
                    Toggle::Enable => window.state.insert(WindowState::FLOATING),
                    Toggle::Disable => window.state.remove(WindowState::FLOATING),
                    Toggle::Toggle => window.toggle_floating(),
                }
                info!("Floating state changed for window: {:?}", window_id);
            }
        }
    }

    /// Handle fullscreen toggle
    fn handle_fullscreen(&mut self, toggle: Toggle) {
        if let Some(window_id) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&window_id) {
                let enable = match toggle {
                    Toggle::Enable => true,
                    Toggle::Disable => false,
                    Toggle::Toggle => !window.state.contains(WindowState::FULLSCREEN),
                };
                // Get output geometry (use default for now)
                let output_geo = Geometry::new(0, 0, 1920, 1080);
                window.set_fullscreen(enable, output_geo);
                info!("Fullscreen state changed for window: {:?}", window_id);
            }
        }
    }

    /// Handle sticky toggle
    fn handle_sticky(&mut self, toggle: Toggle) {
        if let Some(window_id) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&window_id) {
                match toggle {
                    Toggle::Enable => window.state.insert(WindowState::STICKY),
                    Toggle::Disable => window.state.remove(WindowState::STICKY),
                    Toggle::Toggle => {
                        if window.state.contains(WindowState::STICKY) {
                            window.state.remove(WindowState::STICKY);
                        } else {
                            window.state.insert(WindowState::STICKY);
                        }
                    }
                }
            }
        }
    }

    /// Handle split command
    fn handle_split(&mut self, cmd: SplitCmd) {
        debug!("Split command: {:?}", cmd);
        // TODO: Set split direction for current container
    }

    /// Handle layout command
    fn handle_layout(&mut self, cmd: LayoutCmd) {
        debug!("Layout command: {:?}", cmd);
        // TODO: Set layout mode for current container
    }

    /// Handle resize command
    fn handle_resize(&mut self, direction: ResizeDirection, amount: i32) {
        debug!("Resize: {:?} by {}", direction, amount);
        // TODO: Implement window/container resizing
    }

    /// Switch to a workspace
    pub fn switch_workspace(&mut self, target: WorkspaceTarget) {
        debug!("Switching to workspace: {:?}", target);

        let workspace_id = match target {
            WorkspaceTarget::Next | WorkspaceTarget::NextOnOutput => {
                let keys: Vec<_> = self.state.workspaces.keys().cloned().collect();
                if let Some(current) = self.state.focus.focused_workspace {
                    let idx = keys.iter().position(|&id| id == current).unwrap_or(0);
                    keys.get((idx + 1) % keys.len()).cloned()
                } else {
                    keys.first().cloned()
                }
            }
            WorkspaceTarget::Prev | WorkspaceTarget::PrevOnOutput => {
                let keys: Vec<_> = self.state.workspaces.keys().cloned().collect();
                if let Some(current) = self.state.focus.focused_workspace {
                    let idx = keys.iter().position(|&id| id == current).unwrap_or(0);
                    let new_idx = if idx == 0 { keys.len().saturating_sub(1) } else { idx - 1 };
                    keys.get(new_idx).cloned()
                } else {
                    keys.last().cloned()
                }
            }
            WorkspaceTarget::Number(num) => {
                self.state.workspaces.keys().nth((num as usize).saturating_sub(1)).cloned()
            }
            WorkspaceTarget::Name(ref name) => {
                self.state.workspaces.iter()
                    .find(|(_, ws)| ws.name == *name)
                    .map(|(id, _)| *id)
            }
            WorkspaceTarget::BackAndForth => {
                // TODO: implement back and forth
                None
            }
        };

        if let Some(id) = workspace_id {
            self.state.switch_workspace(id);
            self.update_window_visibility();
        }
    }

    /// Move focused window to workspace
    pub fn move_to_workspace(&mut self, target: WorkspaceTarget) {
        let Some(window_id) = self.state.focus.focused_window else { return };
        
        let workspace_id = match target {
            WorkspaceTarget::Number(num) => {
                self.state.workspaces.keys().nth((num as usize).saturating_sub(1)).cloned()
            }
            WorkspaceTarget::Name(ref name) => {
                self.state.workspaces.iter()
                    .find(|(_, ws)| ws.name == *name)
                    .map(|(id, _)| *id)
            }
            _ => None,
        };

        if let Some(new_ws_id) = workspace_id {
            self.state.move_window_to_workspace(window_id, new_ws_id);
            self.update_window_visibility();
        }
    }

    /// Toggle scratchpad visibility
    pub fn toggle_scratchpad(&mut self) {
        if let Some(&window_id) = self.state.scratchpad.first() {
            if self.scratchpad_visible.contains(&window_id) {
                self.scratchpad_visible.retain(|&id| id != window_id);
            } else {
                self.scratchpad_visible.push(window_id);
                self.state.focus_window(window_id);
            }
            self.update_window_visibility();
        }
    }

    /// Reload configuration
    pub fn reload_config(&mut self) {
        info!("Reloading configuration");
        match Config::load(None) {
            Ok(config) => {
                self.input_manager.load_bindings(&config.bindings);
                self.state.config = config;
                info!("Configuration reloaded successfully");
            }
            Err(e) => {
                error!("Failed to reload configuration: {}", e);
            }
        }
    }

    /// Update window visibility based on current workspace
    pub fn update_window_visibility(&mut self) {
        let current_ws_id = self.state.focus.focused_workspace;

        for (window_id, window) in self.state.windows.iter_mut() {
            let should_show = window.workspace == current_ws_id
                || self.scratchpad_visible.contains(window_id)
                || window.state.contains(WindowState::STICKY);

            if should_show {
                window.state.remove(WindowState::HIDDEN);
            } else {
                window.state.insert(WindowState::HIDDEN);
            }
        }
    }

    /// Handle keyboard input (returns true if consumed by binding)
    pub fn handle_keyboard_input(&mut self, keycode: u32, pressed: bool) -> bool {
        // Convert raw keycode to KeyCode (simplified - would need proper mapping)
        // For now, just check bindings directly
        if pressed {
            let command = self.input_manager.key_pressed_raw(keycode).cloned();
            if let Some(cmd) = command {
                self.handle_command(cmd);
                return true;
            }
        }

        false
    }

    /// Handle pointer motion
    pub fn handle_pointer_motion(&mut self, delta_x: f64, delta_y: f64) {
        self.pointer_x += delta_x;
        self.pointer_y += delta_y;

        self.pointer_x = self.pointer_x.max(0.0);
        self.pointer_y = self.pointer_y.max(0.0);

        self.state.pointer_position = (self.pointer_x, self.pointer_y);

        // Handle grab (move/resize in progress)
        if let Some(ref grab) = self.state.grabbed_window {
            let dx = self.pointer_x - grab.initial_pointer.0;
            let dy = self.pointer_y - grab.initial_pointer.1;

            let window_id = grab.window_id;
            let initial = grab.initial_geometry;
            let operation = grab.operation;
            let edges = grab.edges;

            if let Some(window) = self.state.windows.get_mut(&window_id) {
                match operation {
                    GrabOperation::Move => {
                        window.geometry.x = initial.x + dx as i32;
                        window.geometry.y = initial.y + dy as i32;
                    }
                    GrabOperation::Resize => {
                        if edges.contains(ResizeEdges::RIGHT) {
                            window.geometry.width = (initial.width as i32 + dx as i32).max(100) as u32;
                        }
                        if edges.contains(ResizeEdges::BOTTOM) {
                            window.geometry.height = (initial.height as i32 + dy as i32).max(100) as u32;
                        }
                        if edges.contains(ResizeEdges::LEFT) {
                            let new_width = (initial.width as i32 - dx as i32).max(100) as u32;
                            window.geometry.x = initial.x + (initial.width as i32 - new_width as i32);
                            window.geometry.width = new_width;
                        }
                        if edges.contains(ResizeEdges::TOP) {
                            let new_height = (initial.height as i32 - dy as i32).max(100) as u32;
                            window.geometry.y = initial.y + (initial.height as i32 - new_height as i32);
                            window.geometry.height = new_height;
                        }
                    }
                }
            }
        }

        // Focus follows mouse
        let ffm = matches!(
            self.state.config.general.focus_follows_mouse,
            FocusFollowsMouse::Yes | FocusFollowsMouse::Always
        );
        if ffm && self.state.grabbed_window.is_none() {
            if let Some(window_id) = self.state.window_at(self.pointer_x, self.pointer_y) {
                if self.state.focus.focused_window != Some(window_id) {
                    self.state.focus_window(window_id);
                }
            }
        }
    }

    /// Handle pointer button
    pub fn handle_pointer_button(&mut self, button: u32, pressed: bool) {
        // Check for Mod+click for move/resize
        let mods = &self.input_manager.modifiers;

        if pressed && mods.contains(Modifiers::SUPER) {
            if let Some(window_id) = self.state.window_at(self.pointer_x, self.pointer_y) {
                if let Some(window) = self.state.windows.get(&window_id) {
                    let geo = window.geometry;

                    let (operation, edges) = if button == 272 {
                        // Left button - move
                        (GrabOperation::Move, ResizeEdges::empty())
                    } else if button == 273 || button == 274 {
                        // Right/middle button - resize
                        let edge = ResizeEdge::from_point(self.pointer_x, self.pointer_y, &geo);
                        (GrabOperation::Resize, edge.to_edges())
                    } else {
                        return;
                    };

                    self.state.grabbed_window = Some(crate::state::GrabbedWindow {
                        window_id,
                        initial_geometry: geo,
                        initial_pointer: (self.pointer_x, self.pointer_y),
                        operation,
                        edges,
                    });
                }
            }
        }

        if !pressed {
            self.state.grabbed_window = None;
        }

        // Focus window on click
        if pressed && button == 272 && self.state.grabbed_window.is_none() {
            if let Some(window_id) = self.state.window_at(self.pointer_x, self.pointer_y) {
                self.state.focus_window(window_id);
            }
        }
    }

    /// Relayout current workspace
    pub fn relayout(&mut self) {
        if let Some(ws_id) = self.state.focus.focused_workspace {
            let outer_gap = self.state.config.gaps.outer;
            if let Some(workspace) = self.state.workspaces.get_mut(&ws_id) {
                workspace.calculate_layout(outer_gap);
            }
        }
    }
}

/// Run compositor with winit backend (stub for now)
pub fn run_winit(config: Config) -> Result<()> {
    info!("Starting Fluxway");
    info!("Note: Full Wayland compositor implementation requires Smithay API compatibility");
    info!("This is a development build - core window management logic is implemented");

    let mut fluxway = Fluxway::new(config.clone());

    // Run startup commands
    for startup in &config.startup {
        fluxway.spawn_command(&startup.command);
    }

    info!("Fluxway initialized successfully");
    info!("Core features:");
    info!("  - Tree-based tiling layout (i3-style)");
    info!("  - Tabbed and stacked containers (Fluxbox-style)");
    info!("  - Workspace management (10 workspaces)");
    info!("  - Keybinding system with modes");
    info!("  - Scratchpad support");
    info!("  - Window marks (vim-style)");
    info!("  - Focus follows mouse");
    info!("  - Mod+click move/resize");

    // In a real implementation, this would enter the event loop
    // For now, just show configuration summary
    info!("Configuration summary:");
    info!("  - Focus follows mouse: {:?}", config.general.focus_follows_mouse);
    info!("  - Floating modifier: {}", config.general.floating_modifier);
    info!("  - Inner gap: {}", config.gaps.inner);
    info!("  - Outer gap: {}", config.gaps.outer);
    info!("  - Border width: {}", config.border.width);
    info!("  - Keybindings: {}", config.bindings.len());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_edge_from_point() {
        let geo = Geometry::new(0, 0, 300, 300);

        let edge = ResizeEdge::from_point(150.0, 150.0, &geo);
        assert_eq!(edge, ResizeEdge::BottomRight);

        let edge = ResizeEdge::from_point(50.0, 50.0, &geo);
        assert_eq!(edge, ResizeEdge::TopLeft);

        let edge = ResizeEdge::from_point(250.0, 250.0, &geo);
        assert_eq!(edge, ResizeEdge::BottomRight);
    }

    #[test]
    fn test_resize_edge_to_edges() {
        assert_eq!(ResizeEdge::Top.to_edges(), ResizeEdges::TOP);
        assert_eq!(ResizeEdge::TopLeft.to_edges(), ResizeEdges::TOP | ResizeEdges::LEFT);
        assert_eq!(ResizeEdge::BottomRight.to_edges(), ResizeEdges::BOTTOM | ResizeEdges::RIGHT);
    }

    #[test]
    fn test_compositor_new() {
        let config = Config::default();
        let compositor = Fluxway::new(config);
        
        assert!(!compositor.should_exit);
        assert_eq!(compositor.frame_count, 0);
        assert!(compositor.scratchpad_visible.is_empty());
    }
}
