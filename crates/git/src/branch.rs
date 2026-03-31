//! Branch types and management traits.

use serde::{Deserialize, Serialize};

/// Represents a git branch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Branch {
    /// The branch name (e.g., `main`, `feature/foo`).
    pub name: String,
    /// Whether this is a local or remote-tracking branch.
    pub kind: BranchKind,
    /// The upstream tracking branch, if configured.
    pub upstream: Option<String>,
    /// Number of commits ahead of upstream.
    pub ahead: u32,
    /// Number of commits behind upstream.
    pub behind: u32,
    /// Whether this is the currently checked-out branch.
    pub is_head: bool,
}

/// Distinguishes local branches from remote-tracking branches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BranchKind {
    /// A local branch under `refs/heads/`.
    Local,
    /// A remote-tracking branch under `refs/remotes/`.
    Remote,
}

impl Branch {
    /// Create a new local branch with the given name.
    pub fn local(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BranchKind::Local,
            upstream: None,
            ahead: 0,
            behind: 0,
            is_head: false,
        }
    }

    /// Create a new remote-tracking branch.
    pub fn remote(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: BranchKind::Remote,
            upstream: None,
            ahead: 0,
            behind: 0,
            is_head: false,
        }
    }

    /// Mark this branch as the current HEAD.
    pub fn as_head(mut self) -> Self {
        self.is_head = true;
        self
    }

    /// Set upstream tracking information.
    pub fn with_upstream(mut self, upstream: impl Into<String>, ahead: u32, behind: u32) -> Self {
        self.upstream = Some(upstream.into());
        self.ahead = ahead;
        self.behind = behind;
        self
    }

    /// Returns `true` if this branch has diverged from its upstream.
    pub fn has_diverged(&self) -> bool {
        self.ahead > 0 && self.behind > 0
    }

    /// Returns `true` if this branch is up to date with its upstream.
    pub fn is_up_to_date(&self) -> bool {
        self.upstream.is_some() && self.ahead == 0 && self.behind == 0
    }
}

impl std::fmt::Display for Branch {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)?;
        if let Some(ref upstream) = self.upstream {
            write!(f, " -> {upstream}")?;
            if self.ahead > 0 || self.behind > 0 {
                write!(f, " [ahead {}, behind {}]", self.ahead, self.behind)?;
            }
        }
        Ok(())
    }
}

/// Trait for managing branches within a repository.
pub trait BranchManager {
    /// The error type for branch operations.
    type Error: std::error::Error;

    /// List all branches matching the given filter.
    fn list(&self, filter: BranchFilter) -> Result<Vec<Branch>, Self::Error>;

    /// Create a new branch at the given start point.
    fn create(&self, name: &str, start_point: Option<&str>) -> Result<Branch, Self::Error>;

    /// Delete a branch by name.
    fn delete(&self, name: &str, force: bool) -> Result<(), Self::Error>;

    /// Rename a branch.
    fn rename(&self, old_name: &str, new_name: &str) -> Result<Branch, Self::Error>;

    /// Checkout (switch to) a branch.
    fn checkout(&self, name: &str) -> Result<(), Self::Error>;
}

/// Filter options for listing branches.
#[derive(Debug, Clone, Default)]
pub struct BranchFilter {
    /// Only include branches of this kind.
    pub kind: Option<BranchKind>,
    /// Only include branches whose name contains this substring.
    pub name_contains: Option<String>,
    /// Only include merged or unmerged branches relative to HEAD.
    pub merged: Option<bool>,
}
