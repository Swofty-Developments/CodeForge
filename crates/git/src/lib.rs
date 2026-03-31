//! Git abstraction layer for CodeForge.
//!
//! Provides types and traits for interacting with git repositories,
//! branches, commits, diffs, worktrees, remotes, and merge conflicts.

pub mod blame;
pub mod branch;
pub mod commit;
pub mod config;
pub mod conflict;
pub mod diff;
pub mod error;
pub mod hook;
pub mod ignore;
pub mod log;
pub mod remote;
pub mod repository;
pub mod stash;
pub mod submodule;
pub mod tag;
pub mod worktree;

pub use branch::{Branch, BranchManager};
pub use commit::{Commit, CommitBuilder};
pub use conflict::{ConflictFile, ConflictMarker, ConflictRegion};
pub use diff::{DiffHunk, DiffLine, DiffStats, FileDiff};
pub use error::GitError;
pub use remote::Remote;
pub use repository::Repository;
pub use worktree::{Worktree, WorktreeManager};
