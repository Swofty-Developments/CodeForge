use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// A git worktree belonging to a repo. The base checkout is itself reported as a
/// worktree with `is_base: true`. Each worktree is opened as its own repo context
/// (its own daemon / feature index / timeline) and gets a tab in the UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Worktree {
    pub path: PathBuf,
    /// Display name — the branch name, or the directory name on a detached HEAD.
    pub name: String,
    /// `None` on a detached HEAD.
    pub branch: Option<String>,
    /// True for the primary checkout the worktrees were spun off of.
    pub is_base: bool,
    /// Commits ahead of / behind the base branch (0 for the base itself, or when
    /// no merge-base is computable).
    pub ahead: u32,
    pub behind: u32,
    /// Uncommitted changes present in the worktree.
    pub dirty: bool,
}

/// Outcome of merging a worktree branch back into the base branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MergeResult {
    /// True when the merge committed cleanly.
    pub merged: bool,
    /// Repo-relative paths with merge conflicts (empty on a clean merge).
    pub conflicts: Vec<String>,
    /// Whether the working tree was left mid-merge (conflicts to resolve).
    pub aborted: bool,
    /// Human-readable git output / summary.
    pub message: String,
    /// The branch that was merged, and the branch it was merged into.
    pub source_branch: String,
    pub target_branch: String,
}
