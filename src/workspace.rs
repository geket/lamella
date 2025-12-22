//! Workspace management
//!
//! Implements virtual desktops/workspaces with per-workspace layouts.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::layout::{LayoutMode, LayoutTree, SplitDirection};
use crate::state::Geometry;
use crate::window::WindowId;

/// Unique identifier for workspaces
pub type WorkspaceId = Uuid;

/// Workspace configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// Workspace name
    pub name: String,
    /// Output to assign this workspace to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Default layout mode
    #[serde(default)]
    pub layout: LayoutMode,
    /// Custom gaps for this workspace
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gaps: Option<u32>,
}

/// A virtual workspace/desktop
#[derive(Debug)]
pub struct Workspace {
    /// Unique identifier
    pub id: WorkspaceId,
    
    /// Workspace name (can be number or string)
    pub name: String,
    
    /// Display number (for numbered workspaces)
    pub number: Option<u32>,
    
    /// Output this workspace is on
    pub output: Option<String>,
    
    /// Layout tree for tiled windows
    pub layout: LayoutTree,
    
    /// Tiled windows in order
    pub tiled_windows: Vec<WindowId>,
    
    /// Floating windows in stacking order
    pub floating_windows: Vec<WindowId>,
    
    /// Fullscreen window (if any)
    pub fullscreen_window: Option<WindowId>,
    
    /// Focus history for this workspace
    pub focus_stack: Vec<WindowId>,
    
    /// Is this workspace visible?
    pub visible: bool,
    
    /// Is this workspace urgent?
    pub urgent: bool,
    
    /// Workspace geometry (from output)
    pub geometry: Geometry,
    
    /// Available work area (excluding panels, etc.)
    pub work_area: Geometry,
}

impl Workspace {
    /// Create a new workspace
    pub fn new(name: String) -> Self {
        let number = name.parse::<u32>().ok();
        
        Self {
            id: Uuid::new_v4(),
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

    /// Create a workspace with specific configuration
    pub fn with_config(config: WorkspaceConfig) -> Self {
        let mut ws = Self::new(config.name);
        ws.output = config.output;
        ws.layout.default_direction = SplitDirection::Horizontal;
        ws
    }

    /// Add a window to this workspace
    pub fn add_window(&mut self, window_id: WindowId, config: &Config) {
        // Add to tiled windows by default
        self.tiled_windows.push(window_id);
        self.layout.add_window(window_id, config);
        self.focus_stack.push(window_id);
    }

    /// Add a floating window
    pub fn add_floating_window(&mut self, window_id: WindowId) {
        self.floating_windows.push(window_id);
        self.focus_stack.push(window_id);
    }

    /// Remove a window from this workspace
    pub fn remove_window(&mut self, window_id: WindowId) {
        // Remove from tiled
        self.tiled_windows.retain(|&id| id != window_id);
        self.layout.remove_window(window_id);
        
        // Remove from floating
        self.floating_windows.retain(|&id| id != window_id);
        
        // Remove from focus stack
        self.focus_stack.retain(|&id| id != window_id);
        
        // Clear fullscreen if needed
        if self.fullscreen_window == Some(window_id) {
            self.fullscreen_window = None;
        }
    }

    /// Move a window from tiled to floating
    pub fn float_window(&mut self, window_id: WindowId) {
        if let Some(pos) = self.tiled_windows.iter().position(|&id| id == window_id) {
            self.tiled_windows.remove(pos);
            self.layout.remove_window(window_id);
            self.floating_windows.push(window_id);
        }
    }

    /// Move a window from floating to tiled
    pub fn tile_window(&mut self, window_id: WindowId, config: &Config) {
        if let Some(pos) = self.floating_windows.iter().position(|&id| id == window_id) {
            self.floating_windows.remove(pos);
            self.tiled_windows.push(window_id);
            self.layout.add_window(window_id, config);
        }
    }

    /// Get all windows on this workspace
    pub fn windows(&self) -> impl Iterator<Item = WindowId> + '_ {
        self.tiled_windows.iter().chain(self.floating_windows.iter()).copied()
    }

    /// Get the number of windows
    pub fn window_count(&self) -> usize {
        self.tiled_windows.len() + self.floating_windows.len()
    }

    /// Check if workspace is empty
    pub fn is_empty(&self) -> bool {
        self.tiled_windows.is_empty() && self.floating_windows.is_empty()
    }

    /// Check if a window is on this workspace
    pub fn contains(&self, window_id: WindowId) -> bool {
        self.tiled_windows.contains(&window_id) || self.floating_windows.contains(&window_id)
    }

    /// Get the focused window on this workspace
    pub fn focused_window(&self) -> Option<WindowId> {
        self.focus_stack.last().copied()
    }

    /// Focus a window on this workspace
    pub fn focus_window(&mut self, window_id: WindowId) {
        // Remove from current position in focus stack
        self.focus_stack.retain(|&id| id != window_id);
        // Add to top of focus stack
        self.focus_stack.push(window_id);
    }

    /// Raise a floating window to the top
    pub fn raise_window(&mut self, window_id: WindowId) {
        if let Some(pos) = self.floating_windows.iter().position(|&id| id == window_id) {
            self.floating_windows.remove(pos);
            self.floating_windows.push(window_id);
        }
    }

    /// Lower a floating window to the bottom
    pub fn lower_window(&mut self, window_id: WindowId) {
        if let Some(pos) = self.floating_windows.iter().position(|&id| id == window_id) {
            self.floating_windows.remove(pos);
            self.floating_windows.insert(0, window_id);
        }
    }

    /// Set the fullscreen window
    pub fn set_fullscreen(&mut self, window_id: Option<WindowId>) {
        self.fullscreen_window = window_id;
    }

    /// Calculate layout for all tiled windows
    pub fn calculate_layout(&mut self, outer_gap: u32) {
        self.layout.calculate_layout(self.work_area, outer_gap);
    }

    /// Get calculated geometry for a window
    pub fn window_geometry(&self, window_id: WindowId) -> Option<Geometry> {
        self.layout.window_geometries.get(&window_id).copied()
    }

    /// Set workspace geometry
    pub fn set_geometry(&mut self, geometry: Geometry) {
        self.geometry = geometry;
        self.work_area = geometry; // Will be adjusted for panels later
    }

    /// Set work area (available space after panels)
    pub fn set_work_area(&mut self, work_area: Geometry) {
        self.work_area = work_area;
    }
}

/// Manages all workspaces
#[derive(Debug, Default)]
pub struct WorkspaceManager {
    /// Currently focused workspace
    pub focused: Option<WorkspaceId>,
    /// Previous workspace (for back-and-forth)
    pub previous: Option<WorkspaceId>,
    /// Workspace assignment to outputs
    pub output_workspaces: Vec<(String, Vec<WorkspaceId>)>,
}

impl WorkspaceManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Switch to a workspace
    pub fn focus(&mut self, workspace_id: WorkspaceId) {
        if self.focused != Some(workspace_id) {
            self.previous = self.focused;
            self.focused = Some(workspace_id);
        }
    }

    /// Switch to previous workspace (back-and-forth)
    pub fn back_and_forth(&mut self) -> Option<WorkspaceId> {
        if let Some(prev) = self.previous {
            self.previous = self.focused;
            self.focused = Some(prev);
            Some(prev)
        } else {
            None
        }
    }

    /// Assign a workspace to an output
    pub fn assign_to_output(&mut self, workspace_id: WorkspaceId, output: &str) {
        // Find or create output entry
        if let Some((_, workspaces)) = self.output_workspaces.iter_mut()
            .find(|(name, _)| name == output) 
        {
            if !workspaces.contains(&workspace_id) {
                workspaces.push(workspace_id);
            }
        } else {
            self.output_workspaces.push((output.to_string(), vec![workspace_id]));
        }
    }

    /// Get workspaces for an output
    pub fn workspaces_for_output(&self, output: &str) -> &[WorkspaceId] {
        self.output_workspaces.iter()
            .find(|(name, _)| name == output)
            .map(|(_, ws)| ws.as_slice())
            .unwrap_or(&[])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_creation() {
        let ws = Workspace::new("1".to_string());
        assert_eq!(ws.name, "1");
        assert_eq!(ws.number, Some(1));
        assert!(ws.is_empty());
    }

    #[test]
    fn test_workspace_windows() {
        let config = Config::default();
        let mut ws = Workspace::new("1".to_string());
        let window1 = Uuid::new_v4();
        let window2 = Uuid::new_v4();

        ws.add_window(window1, &config);
        ws.add_window(window2, &config);

        assert_eq!(ws.window_count(), 2);
        assert!(ws.contains(window1));
        assert!(ws.contains(window2));

        ws.remove_window(window1);
        assert_eq!(ws.window_count(), 1);
        assert!(!ws.contains(window1));
    }

    #[test]
    fn test_focus_stack() {
        let config = Config::default();
        let mut ws = Workspace::new("1".to_string());
        let window1 = Uuid::new_v4();
        let window2 = Uuid::new_v4();

        ws.add_window(window1, &config);
        ws.add_window(window2, &config);

        assert_eq!(ws.focused_window(), Some(window2));

        ws.focus_window(window1);
        assert_eq!(ws.focused_window(), Some(window1));
    }
}
