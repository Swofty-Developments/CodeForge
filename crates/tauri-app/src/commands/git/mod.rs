//! Git command handlers, split by concern:
//! - `local`: Branch, checkout, commit, status, stash, merge, log
//! - `remote`: Fetch, push, pull, PR creation, remote checks

mod local;
mod remote;

pub use local::*;
pub use remote::*;

use serde::Serialize;

// ── Shared types ──

#[derive(Debug, Clone, Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitDiffStat {
    pub file: String,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize)]
pub struct RemoteUpdate {
    pub branch: String,
    pub behind: u32,
    pub latest_message: String,
}

/// Describes the git state of a project directory.
#[derive(Debug, Clone, Serialize)]
pub struct RepoStatus {
    /// "none" | "git" | "github"
    pub status: String,
    /// Current branch name if git, None otherwise
    pub branch: Option<String>,
    /// Whether a remote named "origin" exists
    pub has_remote: bool,
}
