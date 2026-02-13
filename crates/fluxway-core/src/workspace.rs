//! Workspace management — virtual desktops.

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::layout::{LayoutTree, SplitDirection};
use crate::state::Geometry;
use crate::window::WindowId;

/// Unique identifier for workspaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkspaceId(pub u32);

impl std::fmt::Display for WorkspaceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "ws:{}", self.0)
    }
}

/// A virtual workspace/desktop.
#[derive(Debug)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub number: Option<u32>,
    pub output: Option<String>,
    pub layout: LayoutTree,
    pub tiled_windows: Vec<WindowId>,
    pub floating_windows: Vec<WindowId>,
    pub fullscreen_window: Option<WindowId>,
    pub focus_stack: Vec<WindowId>,
    pub visible: bool,
    pub urgent: bool,
    pub geometry: Geometry,
    pub work_area: Geometry,
}

impl Workspace {
    pub fn new(id: WorkspaceId, name: String) -> Self {
        let number = name.parse::<u32>().ok();
        Self {
            id,
            name,
            number,
            output: None,
            layout: LayoutTree::new(),
            tiled_windows: Vec::new(),
            floating_windows: Vec::new(),
            fullscreen_window: None,
            focus_stack: Vec::new(),
            visible: false,
            urgent: false,
            geometry: Geometry::default(),
            work_area: Geometry::default(),
        }
    }

    pub fn add_window(&mut self, window_id: WindowId, config: &Config) {
        self.tiled_windows.push(window_id);
        self.layout.add_window(window_id, config);
        self.focus_stack.push(window_id);
    }

    pub fn add_floating_window(&mut self, window_id: WindowId) {
        self.floating_windows.push(window_id);
        self.focus_stack.push(window_id);
    }

    pub fn remove_window(&mut self, window_id: WindowId) {
        self.tiled_windows.retain(|&id| id != window_id);
        self.layout.remove_window(window_id);
        self.floating_windows.retain(|&id| id != window_id);
        self.focus_stack.retain(|&id| id != window_id);
        if self.fullscreen_window == Some(window_id) {
            self.fullscreen_window = None;
        }
    }

    pub fn float_window(&mut self, window_id: WindowId) {
        if let Some(pos) = self.tiled_windows.iter().position(|&id| id == window_id) {
            self.tiled_windows.remove(pos);
            self.layout.remove_window(window_id);
            self.floating_windows.push(window_id);
        }
    }

    pub fn tile_window(&mut self, window_id: WindowId, config: &Config) {
        if let Some(pos) = self.floating_windows.iter().position(|&id| id == window_id) {
            self.floating_windows.remove(pos);
            self.tiled_windows.push(window_id);
            self.layout.add_window(window_id, config);
        }
    }

    pub fn windows(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.tiled_windows
            .iter()
            .chain(self.floating_windows.iter())
            .copied()
    }

    pub fn window_count(&self) -> usize {
        self.tiled_windows.len() + self.floating_windows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tiled_windows.is_empty() && self.floating_windows.is_empty()
    }

    pub fn contains(&self, window_id: WindowId) -> bool {
        self.tiled_windows.contains(&window_id) || self.floating_windows.contains(&window_id)
    }

    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus_stack.last().copied()
    }

    pub fn focus_window(&mut self, window_id: WindowId) {
        self.focus_stack.retain(|&id| id != window_id);
        self.focus_stack.push(window_id);
    }

    pub fn calculate_layout(&mut self, outer_gap: u32) {
        self.layout.calculate_layout(self.work_area, outer_gap);
    }

    pub fn window_geometry(&self, window_id: WindowId) -> Option<Geometry> {
        self.layout.window_geometries.get(&window_id).copied()
    }

    pub fn set_geometry(&mut self, geometry: Geometry) {
        self.geometry = geometry;
        self.work_area = geometry;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let ws = Workspace::new(WorkspaceId(1), "1".to_string());
        assert_eq!(ws.name, "1");
        assert_eq!(ws.number, Some(1));
        assert!(ws.is_empty());
    }

    #[test]
    fn test_workspace_windows() {
        let config = Config::default();
        let mut ws = Workspace::new(WorkspaceId(1), "1".to_string());
        let w1 = WindowId(100);
        let w2 = WindowId(101);

        ws.add_window(w1, &config);
        ws.add_window(w2, &config);

        assert_eq!(ws.window_count(), 2);
        assert!(ws.contains(w1));
        assert!(ws.contains(w2));

        ws.remove_window(w1);
        assert_eq!(ws.window_count(), 1);
        assert!(!ws.contains(w1));
    }

    #[test]
    fn test_focus_stack() {
        let config = Config::default();
        let mut ws = Workspace::new(WorkspaceId(1), "1".to_string());
        let w1 = WindowId(100);
        let w2 = WindowId(101);

        ws.add_window(w1, &config);
        ws.add_window(w2, &config);
        assert_eq!(ws.focused_window(), Some(w2));

        ws.focus_window(w1);
        assert_eq!(ws.focused_window(), Some(w1));
    }
}
