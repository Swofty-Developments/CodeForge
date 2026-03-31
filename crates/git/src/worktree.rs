//! Git worktree types and management traits.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Represents a git worktree (either the main working tree or a linked one).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worktree {
    /// Absolute path to the worktree directory.
    pub path: PathBuf,
    /// The branch checked out in this worktree.
    pub branch: Option<String>,
    /// The HEAD commit hash of this worktree.
    pub head: String,
    /// Whether this is the main worktree.
    pub is_main: bool,
    /// Whether this worktree is in a detached HEAD state.
    pub is_detached: bool,
    /// Whether the worktree directory is locked.
    pub is_locked: bool,
    /// Optional lock reason.
    pub lock_reason: Option<String>,
}

impl Worktree {
    /// Create a representation of the main worktree.
    pub fn main(path: impl Into<PathBuf>, head: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            branch: None,
            head: head.into(),
            is_main: true,
            is_detached: false,
            is_locked: false,
            lock_reason: None,
        }
    }

    /// Create a representation of a linked worktree.
    pub fn linked(
        path: impl Into<PathBuf>,
        branch: impl Into<String>,
        head: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            branch: Some(branch.into()),
            head: head.into(),
            is_main: false,
            is_detached: false,
            is_locked: false,
            lock_reason: None,
        }
    }

    /// Returns the worktree path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the branch name, if any.
    pub fn branch_name(&self) -> Option<&str> {
        self.branch.as_deref()
    }

    /// Returns the abbreviated HEAD hash (first 7 characters).
    pub fn short_head(&self) -> &str {
        if self.head.len() >= 7 {
            &self.head[..7]
        } else {
            &self.head
        }
    }
}

impl std::fmt::Display for Worktree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.display())?;
        if let Some(ref branch) = self.branch {
            write!(f, " [{branch}]")?;
        } else {
            write!(f, " (detached {})", self.short_head())?;
        }
        if self.is_locked {
            write!(f, " (locked)")?;
        }
        Ok(())
    }
}

/// Trait for managing git worktrees.
pub trait WorktreeManager {
    /// The error type for worktree operations.
    type Error: std::error::Error;

    /// List all worktrees for the repository.
    fn list(&self) -> Result<Vec<Worktree>, Self::Error>;

    /// Add a new linked worktree at the given path for the given branch.
    fn add(&self, path: &Path, branch: &str) -> Result<Worktree, Self::Error>;

    /// Remove a linked worktree.
    fn remove(&self, path: &Path, force: bool) -> Result<(), Self::Error>;

    /// Lock a worktree with an optional reason.
    fn lock(&self, path: &Path, reason: Option<&str>) -> Result<(), Self::Error>;

    /// Unlock a previously locked worktree.
    fn unlock(&self, path: &Path) -> Result<(), Self::Error>;

    /// Prune worktree metadata for worktrees that no longer exist on disk.
    fn prune(&self) -> Result<usize, Self::Error>;
}
