//! forge-git — diff extraction + diff-by-feature grouping.
//!
//! Shells out to the `git` CLI via `tokio::process::Command` (always with
//! `.current_dir(repo_root)`, errors surfaced from stderr). No libgit2.

use std::path::{Path, PathBuf};

use forge_core::DiffByFeature;
use forge_index::FeatureIndex;

/// Git errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git failed: {0}")]
    Git(String),
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The full pending diff (`git status --porcelain=v2` + `git diff HEAD`, with a
/// `git diff --cached` fallback for repos without a HEAD), parsed into hunks and
/// grouped by feature via `index.classify_paths`. Files in N features appear in
/// each group (`shared: true`); unmatched files land in the "unmapped" group.
pub async fn diff_by_feature(repo_root: &Path, index: &FeatureIndex) -> Result<DiffByFeature> {
    let _ = (repo_root, index);
    // IMPLEMENT(agent): run git, parse unified diff (hand-rolled parser per
    // docs/ARCHITECTURE.md §Diff review), group via index.classify_paths.
    todo!("forge_git::diff_by_feature")
}

/// `git rev-parse HEAD` — the current HEAD sha.
pub async fn current_head(repo_root: &Path) -> Result<String> {
    let _ = repo_root;
    // IMPLEMENT(agent): tokio::process::Command("git").args(["rev-parse", "HEAD"]).
    todo!("forge_git::current_head")
}

/// Repo-relative paths of all changed (staged + unstaged + untracked) files,
/// from `git status --porcelain=v2`.
pub async fn changed_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let _ = repo_root;
    // IMPLEMENT(agent): parse porcelain v2 output.
    todo!("forge_git::changed_files")
}
