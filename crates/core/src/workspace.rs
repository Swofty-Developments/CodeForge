//! Workspace representation and management traits.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a project workspace on disk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workspace {
    /// Absolute path to the workspace root.
    pub root: PathBuf,
    /// Human-readable name (typically the directory name).
    pub name: String,
    /// Whether this workspace is inside a git repository.
    pub is_git_repo: bool,
    /// The current git branch, if applicable.
    pub current_branch: Option<String>,
    /// Remote URL (e.g., GitHub origin), if configured.
    pub remote_url: Option<String>,
    /// Whether the workspace has uncommitted changes.
    pub has_uncommitted_changes: bool,
}

impl Workspace {
    /// Create a new workspace from a directory path.
    ///
    /// This does not perform any filesystem checks; use [`WorkspaceManager::scan`]
    /// to populate git status fields.
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let root = root.into();
        let name = root
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        Self {
            root,
            name,
            is_git_repo: false,
            current_branch: None,
            remote_url: None,
            has_uncommitted_changes: false,
        }
    }

    /// Returns the workspace root path.
    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Returns `true` if the workspace has a remote configured.
    pub fn has_remote(&self) -> bool {
        self.remote_url.is_some()
    }
}

/// Trait for discovering and managing project workspaces.
pub trait WorkspaceManager {
    /// The error type returned by workspace operations.
    type Error: std::error::Error;

    /// Scan a directory to build a [`Workspace`] with full git status.
    fn scan(&self, path: &Path) -> Result<Workspace, Self::Error>;

    /// Validate that a path is a usable workspace (exists, readable, etc.).
    fn validate(&self, path: &Path) -> Result<bool, Self::Error>;

    /// Watch a workspace for filesystem changes, invoking the callback on each change.
    fn watch(
        &self,
        workspace: &Workspace,
        callback: Box<dyn Fn(WorkspaceEvent) + Send>,
    ) -> Result<WatchHandle, Self::Error>;
}

/// An event emitted when a watched workspace changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceEvent {
    /// Path that changed, relative to the workspace root.
    pub relative_path: PathBuf,
    /// The kind of change observed.
    pub kind: WorkspaceEventKind,
}

/// The kind of workspace filesystem event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum WorkspaceEventKind {
    /// A file was created.
    Created,
    /// A file was modified.
    Modified,
    /// A file was deleted.
    Deleted,
    /// A file was renamed.
    Renamed,
}

/// Handle returned by [`WorkspaceManager::watch`] to control the watcher lifetime.
///
/// Dropping this handle should stop the watcher.
pub struct WatchHandle {
    _cancel: Box<dyn FnOnce() + Send>,
}

impl WatchHandle {
    /// Create a new watch handle with the given cancellation function.
    pub fn new(cancel: impl FnOnce() + Send + 'static) -> Self {
        Self {
            _cancel: Box::new(cancel),
        }
    }

    /// Stop watching and release resources.
    pub fn cancel(self) {
        (self._cancel)();
    }
}
