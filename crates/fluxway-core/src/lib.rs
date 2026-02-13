//! Fluxway Core — Protocol-agnostic window manager engine
//!
//! This crate contains all window manager logic (state, layout, commands,
//! workspace management) with zero dependencies on display protocols.
//!
//! Backends (Smithay/Winit/X11) are adapters that translate protocol events
//! into [`CoreEvent`]s, feed them to [`Core`], and apply the returned
//! [`CoreAction`]s back to the display server.
//!
//! # Quick Start
//! ```
//! use fluxway_core::{Core, CoreEvent, Command};
//! use fluxway_core::config::Config;
//! use fluxway_core::input::WorkspaceTarget;
//!
//! let config = Config::default();
//! let mut core = Core::new(config);
//!
//! // Backend tells core a window appeared
//! let actions = core.handle_event(CoreEvent::WindowMapped {
//!     id: core.next_window_id(),
//!     app_id: Some("firefox".into()),
//!     title: Some("Mozilla Firefox".into()),
//!     pid: None,
//!     initial_geometry: None,
//!     is_xwayland: false,
//! });
//!
//! // User presses a keybinding resolved to a command
//! let actions = core.exec(Command::Workspace(WorkspaceTarget::Number(2)));
//! ```

pub mod config;
pub mod event;
pub mod input;
pub mod invariants;
pub mod layout;
pub mod state;
pub mod window;
pub mod workspace;

// Re-export primary API types at crate root
pub use event::{CoreAction, CoreEvent};
pub use input::Command;
pub use state::Geometry;
pub use window::WindowId;
pub use workspace::WorkspaceId;

use std::collections::HashMap;

use indexmap::IndexMap;
use tracing::{debug, error, info, warn};

use config::Config;
use input::{
    FocusTarget, InputManager, LayoutCmd, MoveTarget, ResizeDirection, SplitCmd, Toggle,
    WorkspaceTarget,
};
use layout::LayoutMode;
use state::{FocusState, GrabOperation, GrabbedWindow, Output, ResizeEdges, State};
use window::{Window, WindowState};
use workspace::Workspace;

/// The protocol-agnostic window manager engine.
///
/// Owns all WM state. Backends drive it via [`handle_event`](Core::handle_event)
/// and [`exec`](Core::exec), then apply the returned [`CoreAction`]s.
pub struct Core {
    /// All window-manager state
    pub state: State,
    /// Input/binding manager
    pub input_manager: InputManager,
    /// Scratchpad-visible set (windows currently shown from scratchpad)
    scratchpad_visible: Vec<WindowId>,
    /// Monotonic window ID counter
    next_wid: u64,
    /// Exit requested
    pub should_exit: bool,
}

impl Core {
    /// Create a new core engine with the given configuration.
    pub fn new(config: Config) -> Self {
        let mut input_manager = InputManager::new();
        input_manager.load_bindings(&config.bindings);

        let state = State::new(config);

        Self {
            state,
            input_manager,
            scratchpad_visible: Vec::new(),
            next_wid: 1,
            should_exit: false,
        }
    }

    /// Generate a fresh, unique `WindowId`.
    ///
    /// Backends call this when a new surface/window appears and before
    /// sending [`CoreEvent::WindowMapped`].
    pub fn next_window_id(&mut self) -> WindowId {
        let id = WindowId(self.next_wid);
        self.next_wid += 1;
        id
    }

    // ── Event handling (backend → core) ──────────────────────────────

    /// Process a backend event. Returns actions the backend must apply.
    pub fn handle_event(&mut self, event: CoreEvent) -> Vec<CoreAction> {
        let actions = match event {
            CoreEvent::WindowMapped {
                id,
                app_id,
                title,
                pid,
                initial_geometry,
                is_xwayland,
            } => self.on_window_mapped(id, app_id, title, pid, initial_geometry, is_xwayland),

            CoreEvent::WindowUnmapped { id } => self.on_window_unmapped(id),

            CoreEvent::WindowCommit {
                id,
                new_geometry_hint,
            } => self.on_window_commit(id, new_geometry_hint),

            CoreEvent::FocusRequested { id } => self.on_focus_requested(id),

            CoreEvent::OutputAdded {
                id,
                name,
                geometry,
            } => self.on_output_added(id, name, geometry),

            CoreEvent::OutputRemoved { id } => self.on_output_removed(id),

            CoreEvent::PointerMotion { x, y } => self.on_pointer_motion(x, y),

            CoreEvent::PointerButton { button, pressed } => {
                self.on_pointer_button(button, pressed)
            }

            CoreEvent::Tick => self.on_tick(),
        };

        #[cfg(debug_assertions)]
        if let Err(e) = self.state.validate_invariants() {
            warn!("Invariant violation after handle_event: {}", e);
        }

        actions
    }

    /// Execute a WM command (from keybinding, IPC, etc.). Returns actions.
    pub fn exec(&mut self, command: Command) -> Vec<CoreAction> {
        debug!("exec: {:?}", command);
        let actions = self.execute_command(command);

        #[cfg(debug_assertions)]
        if let Err(e) = self.state.validate_invariants() {
            warn!("Invariant violation after exec: {}", e);
        }

        actions
    }

    /// Run one tick of the compositor loop. Returns actions needed
    /// (e.g., geometry updates from relayout). Returns empty if nothing changed.
    pub fn tick(&mut self) -> Vec<CoreAction> {
        self.on_tick()
    }

    // ── Event handlers ───────────────────────────────────────────────

    fn on_window_mapped(
        &mut self,
        id: WindowId,
        app_id: Option<String>,
        title: Option<String>,
        pid: Option<u32>,
        initial_geometry: Option<Geometry>,
        is_xwayland: bool,
    ) -> Vec<CoreAction> {
        let mut window = Window::new(id, app_id.unwrap_or_default(), title.unwrap_or_default());
        window.pid = pid;
        window.is_xwayland = is_xwayland;

        if let Some(geo) = initial_geometry {
            window.geometry = geo;
        }

        // Apply window rules
        let should_float = window.should_float();
        if should_float {
            window.state.insert(WindowState::FLOATING);
        }

        let added_id = self.state.add_window(window);
        self.state.focus_window(added_id);

        // Produce actions
        let mut actions = Vec::new();
        actions.push(CoreAction::SetFocus {
            id: Some(added_id),
        });

        // Relayout and emit geometry actions
        actions.extend(self.relayout_actions());
        actions
    }

    fn on_window_unmapped(&mut self, id: WindowId) -> Vec<CoreAction> {
        let mut actions = Vec::new();

        if self.state.remove_window(id).is_some() {
            // Report new focus
            actions.push(CoreAction::SetFocus {
                id: self.state.focus.focused_window,
            });
            actions.extend(self.relayout_actions());
        }

        actions
    }

    fn on_window_commit(
        &mut self,
        id: WindowId,
        new_geometry_hint: Option<Geometry>,
    ) -> Vec<CoreAction> {
        if let Some(geo) = new_geometry_hint {
            if let Some(window) = self.state.windows.get_mut(&id) {
                if window.state.contains(WindowState::FLOATING) {
                    window.set_geometry(geo);
                    return vec![CoreAction::SetWindowGeometry {
                        id,
                        x: geo.x,
                        y: geo.y,
                        w: geo.width,
                        h: geo.height,
                    }];
                }
            }
        }
        Vec::new()
    }

    fn on_focus_requested(&mut self, id: WindowId) -> Vec<CoreAction> {
        if self.state.windows.contains_key(&id) {
            self.state.focus_window(id);
            vec![CoreAction::SetFocus { id: Some(id) }]
        } else {
            Vec::new()
        }
    }

    fn on_output_added(
        &mut self,
        id: u64,
        name: String,
        geometry: Geometry,
    ) -> Vec<CoreAction> {
        let output = Output {
            id,
            name,
            geometry,
            scale: 1.0,
            refresh_rate: 60000,
            workspaces: Vec::new(),
            active_workspace: None,
        };
        self.state.outputs.insert(id, output);

        // Set workspace geometry to first output
        for ws in self.state.workspaces.values_mut() {
            if ws.geometry == Geometry::default() {
                ws.set_geometry(geometry);
            }
        }

        self.relayout_actions()
    }

    fn on_output_removed(&mut self, id: u64) -> Vec<CoreAction> {
        self.state.outputs.shift_remove(&id);
        Vec::new()
    }

    fn on_pointer_motion(&mut self, x: f64, y: f64) -> Vec<CoreAction> {
        self.state.pointer_position = (x, y);
        let mut actions = Vec::new();

        // Handle grab (move/resize in progress)
        if let Some(ref grab) = self.state.grabbed_window {
            let dx = x - grab.initial_pointer.0;
            let dy = y - grab.initial_pointer.1;
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
                            window.geometry.width =
                                (initial.width as i32 + dx as i32).max(100) as u32;
                        }
                        if edges.contains(ResizeEdges::BOTTOM) {
                            window.geometry.height =
                                (initial.height as i32 + dy as i32).max(100) as u32;
                        }
                        if edges.contains(ResizeEdges::LEFT) {
                            let new_w = (initial.width as i32 - dx as i32).max(100) as u32;
                            window.geometry.x =
                                initial.x + (initial.width as i32 - new_w as i32);
                            window.geometry.width = new_w;
                        }
                        if edges.contains(ResizeEdges::TOP) {
                            let new_h = (initial.height as i32 - dy as i32).max(100) as u32;
                            window.geometry.y =
                                initial.y + (initial.height as i32 - new_h as i32);
                            window.geometry.height = new_h;
                        }
                    }
                }
                let g = window.geometry;
                actions.push(CoreAction::SetWindowGeometry {
                    id: window_id,
                    x: g.x,
                    y: g.y,
                    w: g.width,
                    h: g.height,
                });
            }
        }

        // Focus-follows-mouse
        use config::FocusFollowsMouse;
        let ffm = matches!(
            self.state.config.general.focus_follows_mouse,
            FocusFollowsMouse::Yes | FocusFollowsMouse::Always
        );
        if ffm && self.state.grabbed_window.is_none() {
            if let Some(window_id) = self.state.window_at(x, y) {
                if self.state.focus.focused_window != Some(window_id) {
                    self.state.focus_window(window_id);
                    actions.push(CoreAction::SetFocus {
                        id: Some(window_id),
                    });
                }
            }
        }

        actions
    }

    fn on_pointer_button(&mut self, button: u32, pressed: bool) -> Vec<CoreAction> {
        let mut actions = Vec::new();
        let (px, py) = self.state.pointer_position;

        if pressed && self.input_manager.modifiers.contains(input::Modifiers::SUPER) {
            if let Some(window_id) = self.state.window_at(px, py) {
                if let Some(window) = self.state.windows.get(&window_id) {
                    let geo = window.geometry;
                    let (operation, edges) = if button == 272 {
                        (GrabOperation::Move, ResizeEdges::empty())
                    } else if button == 273 || button == 274 {
                        let edge = ResizeEdge::from_point(px, py, &geo);
                        (GrabOperation::Resize, edge.to_edges())
                    } else {
                        return actions;
                    };

                    self.state.grabbed_window = Some(GrabbedWindow {
                        window_id,
                        initial_geometry: geo,
                        initial_pointer: (px, py),
                        operation,
                        edges,
                    });
                }
            }
        }

        if !pressed {
            self.state.grabbed_window = None;
        }

        // Focus on click
        if pressed && button == 272 && self.state.grabbed_window.is_none() {
            if let Some(window_id) = self.state.window_at(px, py) {
                self.state.focus_window(window_id);
                actions.push(CoreAction::SetFocus {
                    id: Some(window_id),
                });
            }
        }

        actions
    }

    fn on_tick(&mut self) -> Vec<CoreAction> {
        if self.state.needs_layout() {
            let actions = self.relayout_actions();
            self.state.layout_dirty = false;
            self.update_window_visibility();
            actions
        } else {
            Vec::new()
        }
    }

    // ── Command execution ────────────────────────────────────────────

    fn execute_command(&mut self, command: Command) -> Vec<CoreAction> {
        let mut actions = Vec::new();

        match command {
            Command::Exec(cmd) | Command::ExecAlways(cmd) => {
                actions.push(CoreAction::SpawnProcess { command: cmd });
            }
            Command::Kill => {
                if let Some(wid) = self.state.focus.focused_window {
                    actions.push(CoreAction::RequestClose { id: wid });
                }
            }
            Command::Focus(target) => {
                actions.extend(self.cmd_focus(target));
            }
            Command::Move(target) => {
                actions.extend(self.cmd_move(target));
            }
            Command::Floating(toggle) => {
                actions.extend(self.cmd_floating(toggle));
            }
            Command::Fullscreen(toggle) => {
                actions.extend(self.cmd_fullscreen(toggle));
            }
            Command::Sticky(toggle) => {
                self.cmd_sticky(toggle);
            }
            Command::Split(_cmd) => {
                // TODO: Set split direction for current container
            }
            Command::Layout(_cmd) => {
                // TODO: Set layout mode for current container
            }
            Command::Workspace(target) => {
                actions.extend(self.cmd_switch_workspace(target));
            }
            Command::MoveToWorkspace(target) => {
                actions.extend(self.cmd_move_to_workspace(target));
            }
            Command::ScratchpadShow => {
                actions.extend(self.cmd_toggle_scratchpad());
            }
            Command::MoveToScratchpad => {
                if let Some(wid) = self.state.focus.focused_window {
                    self.state.toggle_scratchpad(wid);
                    actions.extend(self.relayout_actions());
                }
            }
            Command::Mark(mark) => {
                if let Some(wid) = self.state.focus.focused_window {
                    self.state.set_mark(mark, wid);
                }
            }
            Command::GotoMark(mark) => {
                let prev_ws = self.state.focus.focused_workspace;
                self.state.goto_mark(&mark);
                if self.state.focus.focused_workspace != prev_ws {
                    actions.push(CoreAction::WorkspaceChanged {
                        active: self.state.focus.focused_workspace,
                    });
                }
                actions.push(CoreAction::SetFocus {
                    id: self.state.focus.focused_window,
                });
            }
            Command::Unmark(mark) => {
                if let Some(m) = mark {
                    self.state.marks.remove(&m);
                } else {
                    self.state.marks.clear();
                }
            }
            Command::Reload => {
                actions.push(CoreAction::ReloadConfig);
            }
            Command::Restart => {
                info!("Restart requested");
            }
            Command::Exit => {
                self.should_exit = true;
                actions.push(CoreAction::Exit);
            }
            Command::Mode(mode_name) => {
                self.input_manager.set_mode(&mode_name);
            }
            Command::Resize(direction, amount) => {
                // TODO: implement container resizing
                let _ = (direction, amount);
            }
            Command::Gaps(_gap_cmd) => {}
            Command::Bar(_bar_cmd) => {}
            Command::Unknown(cmd) => {
                warn!("Unknown command: {}", cmd);
            }
        }

        actions
    }

    fn cmd_focus(&mut self, target: FocusTarget) -> Vec<CoreAction> {
        // TODO: full directional focus navigation using layout tree
        debug!("Focus target: {:?}", target);
        Vec::new()
    }

    fn cmd_move(&mut self, _target: MoveTarget) -> Vec<CoreAction> {
        // TODO: implement window movement
        Vec::new()
    }

    fn cmd_floating(&mut self, toggle: Toggle) -> Vec<CoreAction> {
        let mut actions = Vec::new();

        if let Some(wid) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&wid) {
                let was_floating = window.state.contains(WindowState::FLOATING);
                match toggle {
                    Toggle::Enable => window.state.insert(WindowState::FLOATING),
                    Toggle::Disable => window.state.remove(WindowState::FLOATING),
                    Toggle::Switch => window.toggle_floating(),
                }
                let is_floating = window.state.contains(WindowState::FLOATING);

                if was_floating != is_floating {
                    actions.push(CoreAction::SetFloating {
                        id: wid,
                        floating: is_floating,
                    });
                    // Move between tiled/floating lists on workspace
                    if let Some(ws_id) = window.workspace {
                        if is_floating {
                            if let Some(ws) = self.state.workspaces.get_mut(&ws_id) {
                                ws.float_window(wid);
                            }
                        } else if let Some(ws) = self.state.workspaces.get_mut(&ws_id) {
                            ws.tile_window(wid, &self.state.config);
                        }
                    }
                    self.state.layout_dirty = true;
                    actions.extend(self.relayout_actions());
                }
            }
        }

        actions
    }

    fn cmd_fullscreen(&mut self, toggle: Toggle) -> Vec<CoreAction> {
        if let Some(wid) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&wid) {
                let enable = match toggle {
                    Toggle::Enable => true,
                    Toggle::Disable => false,
                    Toggle::Switch => !window.state.contains(WindowState::FULLSCREEN),
                };
                // Default output geometry — backend should provide real one via OutputAdded
                let output_geo = self
                    .state
                    .outputs
                    .values()
                    .next()
                    .map(|o| o.geometry)
                    .unwrap_or(Geometry::new(0, 0, 1920, 1080));
                window.set_fullscreen(enable, output_geo);
                let g = window.geometry;
                return vec![CoreAction::SetWindowGeometry {
                    id: wid,
                    x: g.x,
                    y: g.y,
                    w: g.width,
                    h: g.height,
                }];
            }
        }
        Vec::new()
    }

    fn cmd_sticky(&mut self, toggle: Toggle) {
        if let Some(wid) = self.state.focus.focused_window {
            if let Some(window) = self.state.windows.get_mut(&wid) {
                match toggle {
                    Toggle::Enable => window.state.insert(WindowState::STICKY),
                    Toggle::Disable => window.state.remove(WindowState::STICKY),
                    Toggle::Switch => {
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

    fn cmd_switch_workspace(&mut self, target: WorkspaceTarget) -> Vec<CoreAction> {
        let workspace_id = self.resolve_workspace_target(&target);

        if let Some(id) = workspace_id {
            self.state.switch_workspace(id);
            self.update_window_visibility();
            let mut actions = vec![CoreAction::WorkspaceChanged {
                active: Some(id),
            }];
            actions.push(CoreAction::SetFocus {
                id: self.state.focus.focused_window,
            });
            actions.extend(self.relayout_actions());
            actions
        } else {
            Vec::new()
        }
    }

    fn cmd_move_to_workspace(&mut self, target: WorkspaceTarget) -> Vec<CoreAction> {
        let Some(wid) = self.state.focus.focused_window else {
            return Vec::new();
        };
        let Some(ws_id) = self.resolve_workspace_target(&target) else {
            return Vec::new();
        };

        self.state.move_window_to_workspace(wid, ws_id);
        self.update_window_visibility();
        self.relayout_actions()
    }

    fn cmd_toggle_scratchpad(&mut self) -> Vec<CoreAction> {
        let mut actions = Vec::new();

        if let Some(&wid) = self.state.scratchpad.first() {
            if self.scratchpad_visible.contains(&wid) {
                self.scratchpad_visible.retain(|&id| id != wid);
            } else {
                self.scratchpad_visible.push(wid);
                self.state.focus_window(wid);
                actions.push(CoreAction::SetFocus { id: Some(wid) });
            }
            self.update_window_visibility();
        }

        actions
    }

    // ── Helpers ──────────────────────────────────────────────────────

    fn resolve_workspace_target(&self, target: &WorkspaceTarget) -> Option<WorkspaceId> {
        match target {
            WorkspaceTarget::Next | WorkspaceTarget::NextOnOutput => {
                let keys: Vec<_> = self.state.workspaces.keys().copied().collect();
                if let Some(current) = self.state.focus.focused_workspace {
                    let idx = keys.iter().position(|&id| id == current).unwrap_or(0);
                    keys.get((idx + 1) % keys.len()).copied()
                } else {
                    keys.first().copied()
                }
            }
            WorkspaceTarget::Prev | WorkspaceTarget::PrevOnOutput => {
                let keys: Vec<_> = self.state.workspaces.keys().copied().collect();
                if let Some(current) = self.state.focus.focused_workspace {
                    let idx = keys.iter().position(|&id| id == current).unwrap_or(0);
                    let new_idx = if idx == 0 {
                        keys.len().saturating_sub(1)
                    } else {
                        idx - 1
                    };
                    keys.get(new_idx).copied()
                } else {
                    keys.last().copied()
                }
            }
            WorkspaceTarget::Number(num) => self
                .state
                .workspaces
                .keys()
                .nth((*num as usize).saturating_sub(1))
                .copied(),
            WorkspaceTarget::Name(ref name) => self
                .state
                .workspaces
                .iter()
                .find(|(_, ws)| ws.name == *name)
                .map(|(id, _)| *id),
            WorkspaceTarget::BackAndForth => None,
        }
    }

    /// Relayout current workspace and produce geometry actions for all tiled windows.
    fn relayout_actions(&mut self) -> Vec<CoreAction> {
        let ws_id = match self.state.focus.focused_workspace {
            Some(id) => id,
            None => return Vec::new(),
        };

        let outer_gap = self.state.config.gaps.outer;
        if let Some(workspace) = self.state.workspaces.get_mut(&ws_id) {
            workspace.calculate_layout(outer_gap);
        }

        let mut actions = Vec::new();
        if let Some(workspace) = self.state.workspaces.get(&ws_id) {
            for &wid in &workspace.tiled_windows {
                if let Some(geo) = workspace.window_geometry(wid) {
                    actions.push(CoreAction::SetWindowGeometry {
                        id: wid,
                        x: geo.x,
                        y: geo.y,
                        w: geo.width,
                        h: geo.height,
                    });
                }
            }
        }

        self.state.layout_dirty = false;
        actions
    }

    fn update_window_visibility(&mut self) {
        let current_ws_id = self.state.focus.focused_workspace;
        for (window_id, window) in &mut self.state.windows {
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

    /// Reload configuration from the given config value.
    pub fn reload_config(&mut self, config: Config) {
        self.input_manager.load_bindings(&config.bindings);
        self.state.config = config;
        self.state.layout_dirty = true;
    }

    /// Access the focused workspace ID.
    pub fn focused_workspace(&self) -> Option<WorkspaceId> {
        self.state.focus.focused_workspace
    }

    /// Access the focused window ID.
    pub fn focused_window(&self) -> Option<WindowId> {
        self.state.focus.focused_window
    }
}

/// Resize edge for window resizing (protocol-agnostic).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Determine resize edge from click position relative to window geometry.
    pub fn from_point(px: f64, py: f64, geo: &Geometry) -> Self {
        let x = px - f64::from(geo.x);
        let y = py - f64::from(geo.y);
        let width = f64::from(geo.width);
        let height = f64::from(geo.height);

        let left = x < width / 3.0;
        let right = x > width * 2.0 / 3.0;
        let top = y < height / 3.0;
        let bottom = y > height * 2.0 / 3.0;

        match (left, right, top, bottom) {
            (true, _, true, _) => Self::TopLeft,
            (_, true, true, _) => Self::TopRight,
            (true, _, _, true) => Self::BottomLeft,
            (_, true, _, true) => Self::BottomRight,
            (true, _, _, _) => Self::Left,
            (_, true, _, _) => Self::Right,
            (_, _, true, _) => Self::Top,
            (_, _, _, true) => Self::Bottom,
            _ => Self::BottomRight,
        }
    }

    /// Convert to `ResizeEdges` bitflags.
    pub fn to_edges(self) -> ResizeEdges {
        match self {
            Self::Top => ResizeEdges::TOP,
            Self::Bottom => ResizeEdges::BOTTOM,
            Self::Left => ResizeEdges::LEFT,
            Self::Right => ResizeEdges::RIGHT,
            Self::TopLeft => ResizeEdges::TOP | ResizeEdges::LEFT,
            Self::TopRight => ResizeEdges::TOP | ResizeEdges::RIGHT,
            Self::BottomLeft => ResizeEdges::BOTTOM | ResizeEdges::LEFT,
            Self::BottomRight => ResizeEdges::BOTTOM | ResizeEdges::RIGHT,
        }
    }
}
