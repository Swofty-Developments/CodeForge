//! Pane layout management for the application UI.
//!
//! Defines a tree-based layout model where panes can be split horizontally
//! or vertically with configurable resize percentages. The layout can be
//! serialized and restored across sessions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// A unique identifier for a pane in the layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PaneId(Uuid);

impl PaneId {
    /// Create a new random pane identifier.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Create the root pane identifier (deterministic nil UUID).
    pub fn root() -> Self {
        Self(Uuid::nil())
    }

    /// Check if this is the root pane.
    pub fn is_root(&self) -> bool {
        self.0.is_nil()
    }
}

impl Default for PaneId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PaneId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "pane-{}", &self.0.to_string()[..8])
    }
}

/// The direction of a split.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SplitDirection {
    /// Split into left and right halves.
    Horizontal,
    /// Split into top and bottom halves.
    Vertical,
}

impl fmt::Display for SplitDirection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SplitDirection::Horizontal => write!(f, "horizontal"),
            SplitDirection::Vertical => write!(f, "vertical"),
        }
    }
}

/// A node in the layout tree.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LayoutNode {
    /// A leaf pane displaying content.
    Leaf {
        /// The pane identifier.
        id: PaneId,
        /// The content type displayed in this pane.
        content: PaneContent,
    },
    /// A split containing two child nodes.
    Split {
        /// The split direction.
        direction: SplitDirection,
        /// Percentage of space given to the first child (0.0-1.0).
        ratio: f64,
        /// The first (left or top) child.
        first: Box<LayoutNode>,
        /// The second (right or bottom) child.
        second: Box<LayoutNode>,
    },
}

impl LayoutNode {
    /// Create a leaf node with the given content.
    pub fn leaf(content: PaneContent) -> Self {
        LayoutNode::Leaf {
            id: PaneId::new(),
            content,
        }
    }

    /// Create a horizontal split (side by side) with a 50/50 ratio.
    pub fn hsplit(first: LayoutNode, second: LayoutNode) -> Self {
        LayoutNode::Split {
            direction: SplitDirection::Horizontal,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Create a vertical split (stacked) with a 50/50 ratio.
    pub fn vsplit(first: LayoutNode, second: LayoutNode) -> Self {
        LayoutNode::Split {
            direction: SplitDirection::Vertical,
            ratio: 0.5,
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Create a split with a custom ratio.
    pub fn split_with_ratio(
        direction: SplitDirection,
        ratio: f64,
        first: LayoutNode,
        second: LayoutNode,
    ) -> Self {
        LayoutNode::Split {
            direction,
            ratio: ratio.clamp(0.1, 0.9),
            first: Box::new(first),
            second: Box::new(second),
        }
    }

    /// Count the total number of leaf panes in this subtree.
    pub fn pane_count(&self) -> usize {
        match self {
            LayoutNode::Leaf { .. } => 1,
            LayoutNode::Split { first, second, .. } => first.pane_count() + second.pane_count(),
        }
    }

    /// Collect all leaf pane IDs.
    pub fn pane_ids(&self) -> Vec<PaneId> {
        let mut ids = Vec::new();
        self.collect_ids(&mut ids);
        ids
    }

    fn collect_ids(&self, ids: &mut Vec<PaneId>) {
        match self {
            LayoutNode::Leaf { id, .. } => ids.push(*id),
            LayoutNode::Split { first, second, .. } => {
                first.collect_ids(ids);
                second.collect_ids(ids);
            }
        }
    }

    /// Find a pane by ID and return a reference to its content.
    pub fn find_pane(&self, target: PaneId) -> Option<&PaneContent> {
        match self {
            LayoutNode::Leaf { id, content } => {
                if *id == target {
                    Some(content)
                } else {
                    None
                }
            }
            LayoutNode::Split { first, second, .. } => {
                first.find_pane(target).or_else(|| second.find_pane(target))
            }
        }
    }

    /// Find a pane by ID and return a mutable reference to its content.
    pub fn find_pane_mut(&mut self, target: PaneId) -> Option<&mut PaneContent> {
        match self {
            LayoutNode::Leaf { id, content } => {
                if *id == target {
                    Some(content)
                } else {
                    None
                }
            }
            LayoutNode::Split { first, second, .. } => {
                if let Some(c) = first.find_pane_mut(target) {
                    Some(c)
                } else {
                    second.find_pane_mut(target)
                }
            }
        }
    }

    /// Calculate the depth of the layout tree.
    pub fn depth(&self) -> usize {
        match self {
            LayoutNode::Leaf { .. } => 1,
            LayoutNode::Split { first, second, .. } => {
                1 + first.depth().max(second.depth())
            }
        }
    }
}

/// The type of content displayed in a pane.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum PaneContent {
    /// A chat / conversation view.
    Chat {
        /// The thread ID being displayed.
        thread_id: Option<String>,
    },
    /// A file editor view.
    Editor {
        /// The file path being edited.
        file_path: Option<String>,
    },
    /// A diff viewer.
    Diff {
        /// The file being diffed.
        file_path: Option<String>,
    },
    /// A terminal / shell view.
    Terminal {
        /// Working directory for the terminal.
        cwd: Option<String>,
    },
    /// A file tree / explorer sidebar.
    FileTree,
    /// A settings panel.
    Settings,
    /// An empty pane.
    Empty,
}

impl Default for PaneContent {
    fn default() -> Self {
        PaneContent::Empty
    }
}

impl fmt::Display for PaneContent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PaneContent::Chat { thread_id } => {
                write!(f, "Chat")?;
                if let Some(tid) = thread_id {
                    write!(f, " ({tid})")?;
                }
                Ok(())
            }
            PaneContent::Editor { file_path } => {
                write!(f, "Editor")?;
                if let Some(fp) = file_path {
                    write!(f, " ({fp})")?;
                }
                Ok(())
            }
            PaneContent::Diff { file_path } => {
                write!(f, "Diff")?;
                if let Some(fp) = file_path {
                    write!(f, " ({fp})")?;
                }
                Ok(())
            }
            PaneContent::Terminal { cwd } => {
                write!(f, "Terminal")?;
                if let Some(c) = cwd {
                    write!(f, " ({c})")?;
                }
                Ok(())
            }
            PaneContent::FileTree => write!(f, "File Tree"),
            PaneContent::Settings => write!(f, "Settings"),
            PaneContent::Empty => write!(f, "Empty"),
        }
    }
}

/// The complete layout state for the application window.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LayoutState {
    /// The root of the layout tree.
    pub root: LayoutNode,
    /// The currently focused pane.
    pub focused_pane: Option<PaneId>,
    /// Per-pane scroll positions for restoration.
    pub scroll_positions: HashMap<PaneId, ScrollPosition>,
    /// Whether the sidebar is visible.
    pub sidebar_visible: bool,
    /// Width of the sidebar in pixels.
    pub sidebar_width: u32,
    /// Whether the bottom panel is visible.
    pub bottom_panel_visible: bool,
    /// Height of the bottom panel in pixels.
    pub bottom_panel_height: u32,
}

/// Saved scroll position for a pane.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ScrollPosition {
    /// Horizontal scroll offset.
    pub x: f64,
    /// Vertical scroll offset.
    pub y: f64,
}

impl Default for ScrollPosition {
    fn default() -> Self {
        Self { x: 0.0, y: 0.0 }
    }
}

impl LayoutState {
    /// Create a default single-pane layout.
    pub fn single_pane(content: PaneContent) -> Self {
        let node = LayoutNode::leaf(content);
        let pane_id = match &node {
            LayoutNode::Leaf { id, .. } => *id,
            _ => unreachable!(),
        };
        Self {
            root: node,
            focused_pane: Some(pane_id),
            scroll_positions: HashMap::new(),
            sidebar_visible: true,
            sidebar_width: 260,
            bottom_panel_visible: false,
            bottom_panel_height: 200,
        }
    }

    /// Create a two-pane editor + chat layout.
    pub fn editor_and_chat() -> Self {
        let editor = LayoutNode::leaf(PaneContent::Editor { file_path: None });
        let chat = LayoutNode::leaf(PaneContent::Chat { thread_id: None });
        Self {
            root: LayoutNode::split_with_ratio(SplitDirection::Horizontal, 0.6, editor, chat),
            focused_pane: None,
            scroll_positions: HashMap::new(),
            sidebar_visible: true,
            sidebar_width: 260,
            bottom_panel_visible: false,
            bottom_panel_height: 200,
        }
    }

    /// Return the total number of visible panes.
    pub fn pane_count(&self) -> usize {
        self.root.pane_count()
    }

    /// Toggle the sidebar visibility.
    pub fn toggle_sidebar(&mut self) {
        self.sidebar_visible = !self.sidebar_visible;
    }

    /// Toggle the bottom panel visibility.
    pub fn toggle_bottom_panel(&mut self) {
        self.bottom_panel_visible = !self.bottom_panel_visible;
    }

    /// Set the focused pane.
    pub fn focus(&mut self, pane: PaneId) {
        self.focused_pane = Some(pane);
    }

    /// Save a scroll position for a pane.
    pub fn save_scroll(&mut self, pane: PaneId, position: ScrollPosition) {
        self.scroll_positions.insert(pane, position);
    }

    /// Get the saved scroll position for a pane.
    pub fn get_scroll(&self, pane: &PaneId) -> ScrollPosition {
        self.scroll_positions
            .get(pane)
            .copied()
            .unwrap_or_default()
    }

    /// Serialize the layout to JSON for persistence.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a layout from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for LayoutState {
    fn default() -> Self {
        Self::single_pane(PaneContent::Chat { thread_id: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_pane_layout() {
        let layout = LayoutState::single_pane(PaneContent::Chat { thread_id: None });
        assert_eq!(layout.pane_count(), 1);
        assert!(layout.focused_pane.is_some());
    }

    #[test]
    fn split_layout_count() {
        let layout = LayoutState::editor_and_chat();
        assert_eq!(layout.pane_count(), 2);
    }

    #[test]
    fn find_pane_in_tree() {
        let node = LayoutNode::hsplit(
            LayoutNode::leaf(PaneContent::Editor { file_path: None }),
            LayoutNode::leaf(PaneContent::Chat { thread_id: None }),
        );
        let ids = node.pane_ids();
        assert_eq!(ids.len(), 2);
        let content = node.find_pane(ids[0]);
        assert!(content.is_some());
    }

    #[test]
    fn layout_json_roundtrip() {
        let layout = LayoutState::editor_and_chat();
        let json = layout.to_json().unwrap();
        let restored = LayoutState::from_json(&json).unwrap();
        assert_eq!(restored.pane_count(), 2);
    }
}
