//! Core compositor state management
//!
//! This module contains the central state of the compositor, including
//! workspaces, windows, and configuration.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};

use indexmap::IndexMap;
use uuid::Uuid;

use crate::config::Config;
use crate::layout::{Container, ContainerId, LayoutMode, SplitDirection};
use crate::window::{Window, WindowId, WindowState};
use crate::workspace::{Workspace, WorkspaceId};

/// Generates unique IDs for various entities
static NEXT_ID: AtomicU64 = AtomicU64::new(1);

pub fn next_id() -> u64 {
    NEXT_ID.fetch_add(1, Ordering::SeqCst)
}

/// Geometry of a rectangular region
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Geometry {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

impl Geometry {
    pub fn new(x: i32, y: i32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.x
            && x < self.x + self.width as i32
            && y >= self.y
            && y < self.y + self.height as i32
    }

    pub fn intersects(&self, other: &Geometry) -> bool {
        self.x < other.x + other.width as i32
            && self.x + self.width as i32 > other.x
            && self.y < other.y + other.height as i32
            && self.y + self.height as i32 > other.y
    }

    /// Split this geometry horizontally at the given ratio (0.0 to 1.0)
    pub fn split_horizontal(&self, ratio: f64) -> (Geometry, Geometry) {
        let left_width = (self.width as f64 * ratio) as u32;
        let right_width = self.width - left_width;

        let left = Geometry::new(self.x, self.y, left_width, self.height);
        let right = Geometry::new(self.x + left_width as i32, self.y, right_width, self.height);

        (left, right)
    }

    /// Split this geometry vertically at the given ratio (0.0 to 1.0)
    pub fn split_vertical(&self, ratio: f64) -> (Geometry, Geometry) {
        let top_height = (self.height as f64 * ratio) as u32;
        let bottom_height = self.height - top_height;

        let top = Geometry::new(self.x, self.y, self.width, top_height);
        let bottom = Geometry::new(
            self.x,
            self.y + top_height as i32,
            self.width,
            bottom_height,
        );

        (top, bottom)
    }
}

/// Output (monitor) information
#[derive(Debug, Clone)]
pub struct Output {
    pub id: u64,
    pub name: String,
    pub geometry: Geometry,
    pub scale: f64,
    pub refresh_rate: u32, // in mHz
    pub workspaces: Vec<WorkspaceId>,
    pub active_workspace: Option<WorkspaceId>,
}

impl Output {
    pub fn new(name: String, geometry: Geometry) -> Self {
        Self {
            id: next_id(),
            name,
            geometry,
            scale: 1.0,
            refresh_rate: 60000,
            workspaces: Vec::new(),
            active_workspace: None,
        }
    }
}

/// Focus tracking
#[derive(Debug, Clone, Default)]
pub struct FocusState {
    /// Currently focused window
    pub focused_window: Option<WindowId>,
    /// Previously focused window (for focus history)
    pub previous_window: Option<WindowId>,
    /// Currently focused workspace
    pub focused_workspace: Option<WorkspaceId>,
    /// Focus history stack for focus-follows-history
    pub focus_history: Vec<WindowId>,
}

impl FocusState {
    pub fn set_focused(&mut self, window_id: WindowId) {
        if self.focused_window != Some(window_id) {
            self.previous_window = self.focused_window;
            if let Some(prev) = self.focused_window {
                // Add to history, keeping it bounded
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

/// The central compositor state
pub struct State {
    /// Configuration
    pub config: Config,

    /// All managed windows
    pub windows: HashMap<WindowId, Window>,

    /// All workspaces
    pub workspaces: IndexMap<WorkspaceId, Workspace>,

    /// All outputs (monitors)
    pub outputs: IndexMap<u64, Output>,

    /// All containers in the layout tree
    pub containers: HashMap<ContainerId, Container>,

    /// Focus tracking
    pub focus: FocusState,

    /// Scratchpad windows (hidden floating windows)
    pub scratchpad: Vec<WindowId>,

    /// Marks (named references to windows, like vim marks)
    pub marks: HashMap<String, WindowId>,

    /// Running flag
    pub running: bool,

    /// Pending layout recalculation
    pub layout_dirty: bool,

    /// Current pointer position
    pub pointer_position: (f64, f64),

    /// Currently grabbed window (for move/resize)
    pub grabbed_window: Option<GrabbedWindow>,
}

/// State for window move/resize operations
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
        const TOP = 0b0001;
        const BOTTOM = 0b0010;
        const LEFT = 0b0100;
        const RIGHT = 0b1000;
    }
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
            let workspace = Workspace::new(format!("{}", i));
            state.workspaces.insert(workspace.id, workspace);
        }

        state
    }

    /// Add a new window to the compositor
    pub fn add_window(&mut self, mut window: Window) -> WindowId {
        let id = window.id;

        // Determine which workspace to add to
        let workspace_id = self
            .focus
            .focused_workspace
            .or_else(|| self.workspaces.keys().next().copied())
            .expect("No workspaces available");

        window.workspace = Some(workspace_id);

        // Add window to workspace
        if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
            workspace.add_window(id, &self.config);
        }

        self.windows.insert(id, window);
        self.layout_dirty = true;

        id
    }

    /// Remove a window from the compositor
    pub fn remove_window(&mut self, window_id: WindowId) -> Option<Window> {
        let window = self.windows.remove(&window_id)?;

        // Remove from workspace
        if let Some(workspace_id) = window.workspace {
            if let Some(workspace) = self.workspaces.get_mut(&workspace_id) {
                workspace.remove_window(window_id);
            }
        }

        // Remove from focus tracking
        if self.focus.focused_window == Some(window_id) {
            self.focus.clear_focused();
            // Try to focus next window in history
            while let Some(next) = self.focus.focus_history.pop() {
                if self.windows.contains_key(&next) {
                    self.focus.focused_window = Some(next);
                    break;
                }
            }
        }
        self.focus.focus_history.retain(|&id| id != window_id);

        // Remove from scratchpad
        self.scratchpad.retain(|&id| id != window_id);

        // Remove marks pointing to this window
        self.marks.retain(|_, &mut id| id != window_id);

        self.layout_dirty = true;

        Some(window)
    }

    /// Focus a window
    pub fn focus_window(&mut self, window_id: WindowId) {
        if !self.windows.contains_key(&window_id) {
            return;
        }

        // Unfocus current window
        if let Some(current) = self.focus.focused_window {
            if let Some(window) = self.windows.get_mut(&current) {
                window.state.remove(WindowState::FOCUSED);
            }
        }

        // Focus new window
        self.focus.set_focused(window_id);
        if let Some(window) = self.windows.get_mut(&window_id) {
            window.state.insert(WindowState::FOCUSED);
            // Ensure workspace is focused
            if let Some(ws_id) = window.workspace {
                self.focus.focused_workspace = Some(ws_id);
            }
        }
    }

    /// Get the focused window
    pub fn focused_window(&self) -> Option<&Window> {
        self.focus
            .focused_window
            .and_then(|id| self.windows.get(&id))
    }

    /// Get the focused window mutably
    pub fn focused_window_mut(&mut self) -> Option<&mut Window> {
        self.focus
            .focused_window
            .and_then(|id| self.windows.get_mut(&id))
    }

    /// Get the focused workspace
    pub fn focused_workspace(&self) -> Option<&Workspace> {
        self.focus
            .focused_workspace
            .and_then(|id| self.workspaces.get(&id))
    }

    /// Get the focused workspace mutably
    pub fn focused_workspace_mut(&mut self) -> Option<&mut Workspace> {
        self.focus
            .focused_workspace
            .and_then(|id| self.workspaces.get_mut(&id))
    }

    /// Switch to a workspace
    pub fn switch_workspace(&mut self, workspace_id: WorkspaceId) {
        if !self.workspaces.contains_key(&workspace_id) {
            return;
        }

        self.focus.focused_workspace = Some(workspace_id);

        // Focus the most recently focused window on that workspace
        if let Some(workspace) = self.workspaces.get(&workspace_id) {
            if let Some(&window_id) = workspace.focus_stack.last() {
                self.focus_window(window_id);
            }
        }

        self.layout_dirty = true;
    }

    /// Move focused window to a workspace
    pub fn move_window_to_workspace(&mut self, window_id: WindowId, target_workspace: WorkspaceId) {
        let window = match self.windows.get_mut(&window_id) {
            Some(w) => w,
            None => return,
        };

        let old_workspace = window.workspace;
        window.workspace = Some(target_workspace);

        // Remove from old workspace
        if let Some(old_ws_id) = old_workspace {
            if let Some(old_ws) = self.workspaces.get_mut(&old_ws_id) {
                old_ws.remove_window(window_id);
            }
        }

        // Add to new workspace
        if let Some(new_ws) = self.workspaces.get_mut(&target_workspace) {
            new_ws.add_window(window_id, &self.config);
        }

        self.layout_dirty = true;
    }

    /// Toggle window to/from scratchpad
    pub fn toggle_scratchpad(&mut self, window_id: WindowId) {
        if let Some(pos) = self.scratchpad.iter().position(|&id| id == window_id) {
            // Remove from scratchpad and show
            self.scratchpad.remove(pos);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.state.remove(WindowState::HIDDEN);
            }
        } else {
            // Add to scratchpad and hide
            self.scratchpad.push(window_id);
            if let Some(window) = self.windows.get_mut(&window_id) {
                window.state.insert(WindowState::HIDDEN);
            }
        }
        self.layout_dirty = true;
    }

    /// Set a mark on a window
    pub fn set_mark(&mut self, mark: String, window_id: WindowId) {
        self.marks.insert(mark, window_id);
    }

    /// Go to a marked window
    pub fn goto_mark(&mut self, mark: &str) {
        if let Some(&window_id) = self.marks.get(mark) {
            self.focus_window(window_id);
            // Switch to the window's workspace if needed
            if let Some(window) = self.windows.get(&window_id) {
                if let Some(ws_id) = window.workspace {
                    if self.focus.focused_workspace != Some(ws_id) {
                        self.switch_workspace(ws_id);
                    }
                }
            }
        }
    }

    /// Find window at a given position
    pub fn window_at(&self, x: f64, y: f64) -> Option<WindowId> {
        // Iterate in reverse stacking order (top to bottom)
        for (_, workspace) in self.workspaces.iter().rev() {
            if Some(workspace.id) != self.focus.focused_workspace {
                continue;
            }

            // Check floating windows first (they're on top)
            for &window_id in workspace.floating_windows.iter().rev() {
                if let Some(window) = self.windows.get(&window_id) {
                    if !window.state.contains(WindowState::HIDDEN)
                        && window.geometry.contains(x as i32, y as i32)
                    {
                        return Some(window_id);
                    }
                }
            }

            // Then check tiled windows
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

    /// Get output at position
    pub fn output_at(&self, x: f64, y: f64) -> Option<&Output> {
        self.outputs
            .values()
            .find(|output| output.geometry.contains(x as i32, y as i32))
    }

    /// Mark layout as needing recalculation
    pub fn mark_layout_dirty(&mut self) {
        self.layout_dirty = true;
    }

    /// Check if layout needs recalculation
    pub fn needs_layout(&self) -> bool {
        self.layout_dirty
    }
}
