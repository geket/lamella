//! Layout engine for window tiling
//!
//! Implements a tree-based layout system inspired by i3, with additional
//! features like tabbed containers from Fluxbox.
//!
//! The layout tree consists of:
//! - Split containers: divide space horizontally or vertically
//! - Tabbed containers: stack windows with tabs (Fluxbox-style)
//! - Stacked containers: stack windows with title bars
//! - Windows: leaf nodes containing actual windows

use std::collections::HashMap;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::config::Config;
use crate::state::{next_id, Geometry};
use crate::window::WindowId;

/// Unique identifier for a container
pub type ContainerId = Uuid;

/// Direction for splits
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SplitDirection {
    #[default]
    Horizontal,
    Vertical,
}

impl SplitDirection {
    pub fn toggle(&self) -> Self {
        match self {
            Self::Horizontal => Self::Vertical,
            Self::Vertical => Self::Horizontal,
        }
    }
}

/// Layout mode for containers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayoutMode {
    /// Split container (divides space)
    #[default]
    Split,
    /// Tabbed container (Fluxbox-style tabs)
    Tabbed,
    /// Stacked container (vertical tabs/list)
    Stacked,
}

/// A node in the layout tree
#[derive(Debug, Clone)]
pub enum LayoutNode {
    /// A container that holds other nodes
    Container(ContainerId),
    /// A window (leaf node)
    Window(WindowId),
}

/// A container in the layout tree
#[derive(Debug, Clone)]
pub struct Container {
    /// Unique identifier
    pub id: ContainerId,
    /// Parent container (None for root)
    pub parent: Option<ContainerId>,
    /// Children (containers or windows)
    pub children: Vec<LayoutNode>,
    /// Layout mode
    pub layout: LayoutMode,
    /// Split direction (only relevant for Split layout)
    pub split_direction: SplitDirection,
    /// Child sizes as ratios (sum to 1.0)
    pub ratios: Vec<f64>,
    /// Currently focused child index
    pub focused_child: usize,
    /// Calculated geometry
    pub geometry: Geometry,
    /// Gap between children (pixels)
    pub gap: u32,
}

impl Container {
    pub fn new(layout: LayoutMode, split_direction: SplitDirection) -> Self {
        Self {
            id: Uuid::new_v4(),
            parent: None,
            children: Vec::new(),
            layout,
            split_direction,
            ratios: Vec::new(),
            focused_child: 0,
            geometry: Geometry::default(),
            gap: 4,
        }
    }

    pub fn new_split(direction: SplitDirection) -> Self {
        Self::new(LayoutMode::Split, direction)
    }

    pub fn new_tabbed() -> Self {
        Self::new(LayoutMode::Tabbed, SplitDirection::Horizontal)
    }

    pub fn new_stacked() -> Self {
        Self::new(LayoutMode::Stacked, SplitDirection::Vertical)
    }

    /// Add a child node
    pub fn add_child(&mut self, node: LayoutNode) {
        self.children.push(node);
        self.recalculate_ratios();
    }

    /// Insert a child at a specific position
    pub fn insert_child(&mut self, index: usize, node: LayoutNode) {
        let index = index.min(self.children.len());
        self.children.insert(index, node);
        self.recalculate_ratios();
    }

    /// Remove a child node
    pub fn remove_child(&mut self, index: usize) -> Option<LayoutNode> {
        if index < self.children.len() {
            let node = self.children.remove(index);
            self.recalculate_ratios();
            if self.focused_child >= self.children.len() && !self.children.is_empty() {
                self.focused_child = self.children.len() - 1;
            }
            Some(node)
        } else {
            None
        }
    }

    /// Remove a specific child
    pub fn remove_node(&mut self, node: &LayoutNode) -> bool {
        if let Some(pos) = self.children.iter().position(|n| match (n, node) {
            (LayoutNode::Container(a), LayoutNode::Container(b)) => a == b,
            (LayoutNode::Window(a), LayoutNode::Window(b)) => a == b,
            _ => false,
        }) {
            self.remove_child(pos);
            true
        } else {
            false
        }
    }

    /// Recalculate child ratios to be equal
    fn recalculate_ratios(&mut self) {
        let n = self.children.len();
        if n > 0 {
            let ratio = 1.0 / n as f64;
            self.ratios = vec![ratio; n];
        } else {
            self.ratios.clear();
        }
    }

    /// Resize a child by adjusting ratios
    pub fn resize_child(&mut self, index: usize, delta: f64) {
        if self.children.len() < 2 || index >= self.children.len() - 1 {
            return;
        }

        let min_ratio = 0.05; // Minimum 5% size
        let max_delta =
            (self.ratios[index + 1] - min_ratio).min(1.0 - min_ratio - self.ratios[index]);
        let min_delta = -(self.ratios[index] - min_ratio);
        let clamped_delta = delta.clamp(min_delta, max_delta);

        self.ratios[index] += clamped_delta;
        self.ratios[index + 1] -= clamped_delta;
    }

    /// Focus next child
    pub fn focus_next(&mut self) {
        if !self.children.is_empty() {
            self.focused_child = (self.focused_child + 1) % self.children.len();
        }
    }

    /// Focus previous child
    pub fn focus_prev(&mut self) {
        if !self.children.is_empty() {
            self.focused_child = if self.focused_child == 0 {
                self.children.len() - 1
            } else {
                self.focused_child - 1
            };
        }
    }

    /// Get the focused child
    pub fn focused(&self) -> Option<&LayoutNode> {
        self.children.get(self.focused_child)
    }

    /// Check if this container is empty
    pub fn is_empty(&self) -> bool {
        self.children.is_empty()
    }

    /// Check if this container contains a window
    pub fn contains_window(&self, window_id: WindowId) -> bool {
        self.children
            .iter()
            .any(|node| matches!(node, LayoutNode::Window(id) if *id == window_id))
    }
}

/// The layout tree for a workspace
#[derive(Debug)]
pub struct LayoutTree {
    /// All containers in the tree
    pub containers: HashMap<ContainerId, Container>,
    /// Root container ID
    pub root: Option<ContainerId>,
    /// Currently focused container
    pub focused_container: Option<ContainerId>,
    /// Default split direction for new containers
    pub default_direction: SplitDirection,
    /// Calculated window geometries
    pub window_geometries: HashMap<WindowId, Geometry>,
}

impl Default for LayoutTree {
    fn default() -> Self {
        Self::new()
    }
}

impl LayoutTree {
    pub fn new() -> Self {
        Self {
            containers: HashMap::new(),
            root: None,
            focused_container: None,
            default_direction: SplitDirection::Horizontal,
            window_geometries: HashMap::new(),
        }
    }

    /// Add a new window to the layout
    pub fn add_window(&mut self, window_id: WindowId, config: &Config) {
        let node = LayoutNode::Window(window_id);

        // If no root, create one
        if self.root.is_none() {
            let mut root = Container::new_split(self.default_direction);
            root.gap = config.gaps.inner;
            root.add_child(node);
            let root_id = root.id;
            self.containers.insert(root_id, root);
            self.root = Some(root_id);
            self.focused_container = Some(root_id);
            return;
        }

        // Add to focused container or root
        let target_id = self.focused_container.or(self.root).unwrap();
        if let Some(container) = self.containers.get_mut(&target_id) {
            let insert_pos = container.focused_child + 1;
            container.insert_child(insert_pos, node);
            container.focused_child = insert_pos;
        }
    }

    /// Remove a window from the layout
    pub fn remove_window(&mut self, window_id: WindowId) -> bool {
        let node = LayoutNode::Window(window_id);

        // Find and remove from container
        let mut container_to_remove = None;
        for (container_id, container) in self.containers.iter_mut() {
            if container.remove_node(&node) {
                if container.is_empty() {
                    container_to_remove = Some(*container_id);
                }
                break;
            }
        }

        // Clean up empty containers
        if let Some(empty_id) = container_to_remove {
            self.remove_empty_container(empty_id);
        }

        self.window_geometries.remove(&window_id);
        true
    }

    /// Remove an empty container and update parent
    fn remove_empty_container(&mut self, container_id: ContainerId) {
        let container = match self.containers.remove(&container_id) {
            Some(c) => c,
            None => return,
        };

        if self.root == Some(container_id) {
            self.root = None;
            self.focused_container = None;
            return;
        }

        // Remove from parent
        if let Some(parent_id) = container.parent {
            if let Some(parent) = self.containers.get_mut(&parent_id) {
                parent.remove_node(&LayoutNode::Container(container_id));

                // If parent now has only one child, flatten
                if parent.children.len() == 1 {
                    // This is simplified - full implementation would flatten
                }
            }
        }
    }

    /// Split the focused container
    pub fn split(&mut self, direction: SplitDirection, config: &Config) {
        if let Some(container_id) = self.focused_container {
            if let Some(container) = self.containers.get_mut(&container_id) {
                container.split_direction = direction;
                container.layout = LayoutMode::Split;
            }
        }
    }

    /// Change layout mode of focused container
    pub fn set_layout(&mut self, mode: LayoutMode) {
        if let Some(container_id) = self.focused_container {
            if let Some(container) = self.containers.get_mut(&container_id) {
                container.layout = mode;
            }
        }
    }

    /// Toggle split direction
    pub fn toggle_split(&mut self) {
        if let Some(container_id) = self.focused_container {
            if let Some(container) = self.containers.get_mut(&container_id) {
                container.split_direction = container.split_direction.toggle();
            }
        }
    }

    /// Calculate layouts for all windows
    pub fn calculate_layout(&mut self, available: Geometry, outer_gap: u32) {
        self.window_geometries.clear();

        if let Some(root_id) = self.root {
            // Apply outer gaps
            let inner = Geometry::new(
                available.x + outer_gap as i32,
                available.y + outer_gap as i32,
                available.width.saturating_sub(outer_gap * 2),
                available.height.saturating_sub(outer_gap * 2),
            );

            self.layout_container(root_id, inner);
        }
    }

    /// Recursively layout a container
    fn layout_container(&mut self, container_id: ContainerId, geometry: Geometry) {
        let container = match self.containers.get(&container_id).cloned() {
            Some(c) => c,
            None => return,
        };

        // Update container geometry
        if let Some(c) = self.containers.get_mut(&container_id) {
            c.geometry = geometry;
        }

        if container.children.is_empty() {
            return;
        }

        match container.layout {
            LayoutMode::Split => self.layout_split(&container, geometry),
            LayoutMode::Tabbed | LayoutMode::Stacked => self.layout_tabbed(&container, geometry),
        }
    }

    /// Layout children in split mode
    fn layout_split(&mut self, container: &Container, geometry: Geometry) {
        let n = container.children.len();
        if n == 0 {
            return;
        }

        let gap = container.gap;
        let total_gap = gap * (n as u32 - 1);

        match container.split_direction {
            SplitDirection::Horizontal => {
                let available_width = geometry.width.saturating_sub(total_gap);
                let mut x = geometry.x;

                for (i, child) in container.children.iter().enumerate() {
                    let ratio = container.ratios.get(i).copied().unwrap_or(1.0 / n as f64);
                    let width = if i == n - 1 {
                        // Last child gets remaining space to avoid rounding errors
                        (geometry.x + geometry.width as i32 - x) as u32
                    } else {
                        (available_width as f64 * ratio) as u32
                    };

                    let child_geo = Geometry::new(x, geometry.y, width, geometry.height);
                    self.layout_child(child, child_geo);

                    x += width as i32 + gap as i32;
                }
            },
            SplitDirection::Vertical => {
                let available_height = geometry.height.saturating_sub(total_gap);
                let mut y = geometry.y;

                for (i, child) in container.children.iter().enumerate() {
                    let ratio = container.ratios.get(i).copied().unwrap_or(1.0 / n as f64);
                    let height = if i == n - 1 {
                        (geometry.y + geometry.height as i32 - y) as u32
                    } else {
                        (available_height as f64 * ratio) as u32
                    };

                    let child_geo = Geometry::new(geometry.x, y, geometry.width, height);
                    self.layout_child(child, child_geo);

                    y += height as i32 + gap as i32;
                }
            },
        }
    }

    /// Layout children in tabbed/stacked mode
    fn layout_tabbed(&mut self, container: &Container, geometry: Geometry) {
        // In tabbed/stacked mode, only the focused child is visible at full size
        // (In a real implementation, we'd reserve space for tabs/titles)
        let tab_height = 24u32; // Height for tab bar

        let content_geo = match container.layout {
            LayoutMode::Tabbed => Geometry::new(
                geometry.x,
                geometry.y + tab_height as i32,
                geometry.width,
                geometry.height.saturating_sub(tab_height),
            ),
            LayoutMode::Stacked => {
                let header_height = tab_height * container.children.len() as u32;
                Geometry::new(
                    geometry.x,
                    geometry.y + header_height as i32,
                    geometry.width,
                    geometry.height.saturating_sub(header_height),
                )
            },
            _ => geometry,
        };

        // Layout all children at the same position (only focused is visible)
        for (i, child) in container.children.iter().enumerate() {
            if i == container.focused_child {
                self.layout_child(child, content_geo);
            }
        }
    }

    /// Layout a single child (container or window)
    fn layout_child(&mut self, child: &LayoutNode, geometry: Geometry) {
        match child {
            LayoutNode::Container(id) => {
                self.layout_container(*id, geometry);
            },
            LayoutNode::Window(id) => {
                self.window_geometries.insert(*id, geometry);
            },
        }
    }

    /// Move focus in a direction
    pub fn focus_direction(&mut self, direction: Direction) -> Option<WindowId> {
        let container_id = self.focused_container?;

        // First, update focus in container
        {
            let container = self.containers.get_mut(&container_id)?;
            match direction {
                Direction::Left | Direction::Up => container.focus_prev(),
                Direction::Right | Direction::Down => container.focus_next(),
            }
        }

        // Then get the focused node (immutable borrow)
        let container = self.containers.get(&container_id)?;
        match container.focused() {
            Some(LayoutNode::Window(id)) => Some(*id),
            Some(LayoutNode::Container(id)) => self.first_window_in_container(*id),
            None => None,
        }
    }

    /// Get the first window in a container (for focus)
    fn first_window_in_container(&self, container_id: ContainerId) -> Option<WindowId> {
        let container = self.containers.get(&container_id)?;
        match container.children.first()? {
            LayoutNode::Window(id) => Some(*id),
            LayoutNode::Container(id) => self.first_window_in_container(*id),
        }
    }

    /// Swap the focused window with adjacent window in direction
    pub fn swap_direction(&mut self, direction: Direction) {
        if let Some(container_id) = self.focused_container {
            if let Some(container) = self.containers.get_mut(&container_id) {
                let current = container.focused_child;
                let target = match direction {
                    Direction::Left | Direction::Up => {
                        if current > 0 {
                            current - 1
                        } else {
                            return;
                        }
                    },
                    Direction::Right | Direction::Down => {
                        if current < container.children.len() - 1 {
                            current + 1
                        } else {
                            return;
                        }
                    },
                };

                container.children.swap(current, target);
                container.ratios.swap(current, target);
                container.focused_child = target;
            }
        }
    }

    /// Resize focused container
    pub fn resize(&mut self, direction: Direction, amount: i32) {
        if let Some(container_id) = self.focused_container {
            if let Some(container) = self.containers.get_mut(&container_id) {
                let delta = amount as f64 / 1000.0; // Convert pixels to ratio
                match direction {
                    Direction::Left => {
                        container.resize_child(container.focused_child.saturating_sub(1), -delta)
                    },
                    Direction::Right => container.resize_child(container.focused_child, delta),
                    Direction::Up => {
                        container.resize_child(container.focused_child.saturating_sub(1), -delta)
                    },
                    Direction::Down => container.resize_child(container.focused_child, delta),
                }
            }
        }
    }
}

/// Direction for focus/move operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Left,
    Right,
    Up,
    Down,
}

impl Direction {
    pub fn opposite(&self) -> Self {
        match self {
            Self::Left => Self::Right,
            Self::Right => Self::Left,
            Self::Up => Self::Down,
            Self::Down => Self::Up,
        }
    }

    pub fn is_horizontal(&self) -> bool {
        matches!(self, Self::Left | Self::Right)
    }

    pub fn is_vertical(&self) -> bool {
        matches!(self, Self::Up | Self::Down)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_container_add_child() {
        let mut container = Container::new_split(SplitDirection::Horizontal);
        container.add_child(LayoutNode::Window(Uuid::new_v4()));
        container.add_child(LayoutNode::Window(Uuid::new_v4()));

        assert_eq!(container.children.len(), 2);
        assert_eq!(container.ratios.len(), 2);
        assert!((container.ratios[0] - 0.5).abs() < 0.001);
    }

    #[test]
    fn test_layout_tree_add_window() {
        let mut tree = LayoutTree::new();
        let config = Config::default();

        let window1 = Uuid::new_v4();
        let window2 = Uuid::new_v4();

        tree.add_window(window1, &config);
        tree.add_window(window2, &config);

        assert!(tree.root.is_some());
        let root = tree.containers.get(&tree.root.unwrap()).unwrap();
        assert_eq!(root.children.len(), 2);
    }

    #[test]
    fn test_geometry_split() {
        let geo = Geometry::new(0, 0, 1000, 500);

        let (left, right) = geo.split_horizontal(0.5);
        assert_eq!(left.width, 500);
        assert_eq!(right.width, 500);
        assert_eq!(right.x, 500);

        let (top, bottom) = geo.split_vertical(0.3);
        assert_eq!(top.height, 150);
        assert_eq!(bottom.height, 350);
    }
}
