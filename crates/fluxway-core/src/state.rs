//! Core compositor state.

use std::collections::HashMap;

use indexmap::IndexMap;

use crate::config::Config;
use crate::layout::{Container, ContainerId};
use crate::window::{Window, WindowId, WindowState};
use crate::workspace::{Workspace, WorkspaceId};

/// Geometry of a rectangular region.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    pub const fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    #[allow(clippy::cast_possible_wrap)]
    pub const fn contains(self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    #[allow(clippy::cast_possible_wrap)]
    pub const fn intersects(self, other: Self) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    pub fn split_horizontal(self, ratio: f64) -> (Self, Self) {
        let left_width = (f64::from(self.width) * ratio) as u32;
        let right_width = self.width - left_width;
        let left = Self::new(self.x, self.y, left_width, self.height);
        let right = Self::new(self.x + left_width as i32, self.y, right_width, self.height);
        (left, right)
    }

    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss, clippy::cast_possible_wrap)]
    pub fn split_vertical(self, ratio: f64) -> (Self, Self) {
        let top_height = (f64::from(self.height) * ratio) as u32;
        let bottom_height = self.height - top_height;
        let top = Self::new(self.x, self.y, self.width, top_height);
        let bottom = Self::new(self.x, self.y + top_height as i32, self.width, bottom_height);
        (top, bottom)
    }
}

/// Output (monitor) information.
#[derive(Debug, Clone)]
pub struct Output {
    pub id: u64,
    pub name: String,
    pub geometry: Geometry,
    pub scale: f64,
    pub refresh_rate: u32,
    pub workspaces: Vec<WorkspaceId>,
    pub active_workspace: Option<WorkspaceId>,
}

/// Focus tracking.
#[derive(Debug, Clone, Default)]
pub struct FocusState {
    pub focused_window: Option<WindowId>,
    pub previous_window: Option<WindowId>,
    pub focused_workspace: Option<WorkspaceId>,
    pub focus_history: Vec<WindowId>,
}

impl FocusState {
    pub fn set_focused(&mut self, window_id: WindowId) {
        if self.focused_window != Some(window_id) {
            self.previous_window = self.focused_window;
            if let Some(prev) = self.focused_window {
                self.focus_history.retain(|&id| id != prev);
                self.focus_history.push(prev);
                if self.focus_history.len() > 100 {
                    self.focus_history.remove(0);
                }
            }
            self.focused_window = Some(window_id);
        }
    }

    pub fn clear_focused(&mut self) {
        self.previous_window = self.focused_window;
        self.focused_window = None;
    }
}

/// State for window move/resize operations.
#[derive(Debug, Clone)]
pub struct GrabbedWindow {
    pub window_id: WindowId,
    pub initial_geometry: Geometry,
    pub initial_pointer: (f64, f64),
    pub operation: GrabOperation,
    pub edges: ResizeEdges,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrabOperation {
    Move,
    Resize,
}

bitflags::bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ResizeEdges: u8 {
        const TOP    = 0b0001;
        const BOTTOM = 0b0010;
        const LEFT   = 0b0100;
        const RIGHT  = 0b1000;
    }
}

/// The central compositor state.
pub struct State {
    pub config: Config,
    pub windows: HashMap<WindowId, Window>,
    pub workspaces: IndexMap<WorkspaceId, Workspace>,
    pub outputs: IndexMap<u64, Output>,
    pub containers: HashMap<ContainerId, Container>,
    pub focus: FocusState,
    pub scratchpad: Vec<WindowId>,
    pub marks: HashMap<String, WindowId>,
    pub running: bool,
    pub layout_dirty: bool,
    pub pointer_position: (f64, f64),
    pub grabbed_window: Option<GrabbedWindow>,
}

impl State {
    pub fn new(config: Config) -> Self {
        let mut state = Self {
            config,
            windows: HashMap::new(),
            workspaces: IndexMap::new(),
            outputs: IndexMap::new(),
            containers: HashMap::new(),
            focus: FocusState::default(),
            scratchpad: Vec::new(),
            marks: HashMap::new(),
            running: true,
            layout_dirty: false,
            pointer_position: (0.0, 0.0),
            grabbed_window: None,
        };

        // Create default workspaces
        for i in 1..=10 {
            let ws_id = WorkspaceId(i);
            let workspace = Workspace::new(ws_id, format!("{i}"));
            state.workspaces.insert(ws_id, workspace);
        }

        state
    }

    pub fn add_window(&mut self, mut window: Window) -> WindowId {
        let id = window.id;

        let workspace_id = self
            .focus
            .focused_workspace
            .or_else(|| self.workspaces.keys().next().copied())
            .expect("No workspaces available");

        window.workspace = Some(workspace_id);

        if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
            workspace.add_window(id, &self.config);
        }

        self.windows.insert(id, window);
        self.layout_dirty = true;

        id
    }

    pub fn remove_window(&mut self, window_id: WindowId) -> Option<Window> {
        let window = self.windows.remove(&window_id)?;

        if let Some(workspace_id) = window.workspace {
            if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
                workspace.remove_window(window_id);
            }
        }

        if self.focus.focused_window == Some(window_id) {
            self.focus.clear_focused();
            while let Some(next) = self.focus.focus_history.pop() {
                if self.windows.contains_key(&next) {
                    self.focus.focused_window = Some(next);
                    break;
                }
            }
        }
        self.focus.focus_history.retain(|&id| id != window_id);
        self.scratchpad.retain(|&id| id != window_id);
        self.marks.retain(|_, &mut id| id != window_id);
        self.layout_dirty = true;

        Some(window)
    }

    pub fn focus_window(&mut self, window_id: WindowId) {
        if !self.windows.contains_key(&window_id) {
            return;
        }

        if let Some(current) = self.focus.focused_window {
            if let Some(window) = self.windows.get_mut(&current) {
                window.state.remove(WindowState::FOCUSED);
            }
        }

        self.focus.set_focused(window_id);
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.state.insert(WindowState::FOCUSED);
            if let Some(ws_id) = window.workspace {
                self.focus.focused_workspace = Some(ws_id);
            }
        }
    }

    pub fn focused_window(&self) -> Option<&Window> {
        self.focus.focused_window.and_then(|id| self.windows.get(&id))
    }

    pub fn focused_workspace(&self) -> Option<&Workspace> {
        self.focus.focused_workspace.and_then(|id| self.workspaces.get(&id))
    }

    pub fn switch_workspace(&mut self, workspace_id: WorkspaceId) {
        if !self.workspaces.contains_key(&workspace_id) {
            return;
        }
        self.focus.focused_workspace = Some(workspace_id);

        if let Some(workspace) = self.workspaces.get(&workspace_id) {
            if let Some(&window_id) = workspace.focus_stack.last() {
                self.focus_window(window_id);
            }
        }
        self.layout_dirty = true;
    }

    pub fn move_window_to_workspace(&mut self, window_id: WindowId, target_workspace: WorkspaceId) {
        let window = match self.windows.get_mut(&window_id) {
            Some(w) => w,
            None => return,
        };

        let old_workspace = window.workspace;
        window.workspace = Some(target_workspace);

        if let Some(old_ws_id) = old_workspace {
            if let Some(old_ws) = self.workspaces.get_mut(&old_ws_id) {
                old_ws.remove_window(window_id);
            }
        }

        if let Some(new_ws) = self.workspaces.get_mut(&target_workspace) {
            new_ws.add_window(window_id, &self.config);
        }

        self.layout_dirty = true;
    }

    pub fn toggle_scratchpad(&mut self, window_id: WindowId) {
        if let Some(pos) = self.scratchpad.iter().position(|&id| id == window_id) {
            self.scratchpad.remove(pos);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.state.remove(WindowState::HIDDEN);
            }
        } else {
            self.scratchpad.push(window_id);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.state.insert(WindowState::HIDDEN);
            }
        }
        self.layout_dirty = true;
    }

    pub fn set_mark(&mut self, mark: String, window_id: WindowId) {
        self.marks.insert(mark, window_id);
    }

    pub fn goto_mark(&mut self, mark: &str) {
        if let Some(&window_id) = self.marks.get(mark) {
            self.focus_window(window_id);
            if let Some(window) = self.windows.get(&window_id) {
                if let Some(ws_id) = window.workspace {
                    if self.focus.focused_workspace != Some(ws_id) {
                        self.switch_workspace(ws_id);
                    }
                }
            }
        }
    }

    pub fn window_at(&self, x: f64, y: f64) -> Option<WindowId> {
        for (_, workspace) in self.workspaces.iter().rev() {
            if Some(workspace.id) != self.focus.focused_workspace {
                continue;
            }
            for &window_id in workspace.floating_windows.iter().rev() {
                if let Some(window) = self.windows.get(&window_id) {
                    if !window.state.contains(WindowState::HIDDEN)
                        && window.geometry.contains(x as i32, y as i32)
                    {
                        return Some(window_id);
                    }
                }
            }
            for &window_id in workspace.tiled_windows.iter().rev() {
                if let Some(window) = self.windows.get(&window_id) {
                    if !window.state.contains(WindowState::HIDDEN)
                        && window.geometry.contains(x as i32, y as i32)
                    {
                        return Some(window_id);
                    }
                }
            }
        }
        None
    }

    pub fn mark_layout_dirty(&mut self) {
        self.layout_dirty = true;
    }

    pub const fn needs_layout(&self) -> bool {
        self.layout_dirty
    }

    /// Validate core invariants. See `invariants` module.
    pub fn validate_invariants(&self) -> Result<(), crate::invariants::InvariantError> {
        crate::invariants::validate(self)
    }
}
