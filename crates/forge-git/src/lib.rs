//! forge-git — diff extraction + diff-by-feature grouping.
//!
//! Shells out to the `git` CLI via `tokio::process::Command` (always with
//! `.current_dir(repo_root)`, errors surfaced from stderr). No libgit2.

mod branches;
#[cfg(test)]
mod branches_tests;
mod group;
mod parse;
mod status;
#[cfg(test)]
mod tests;
mod truncate;
mod untracked;
mod worktree;
mod worktree_parse;
#[cfg(test)]
mod worktree_tests;

use std::path::{Path, PathBuf};

use forge_core::{DiffByFeature, FileDiff};
use forge_index::FeatureIndex;
use tokio::process::Command;

pub use branches::{fetch_remotes, list_branches, worktree_for_branch, BranchInfo};
pub use worktree::{
    base_repo_root, create_worktree, list_worktrees, merge_worktree, remove_worktree,
};

/// Git errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git failed: {0}")]
    Git(String),
    #[error("not a git repository: {0}")]
    NotARepo(PathBuf),
    #[error("worktree path already exists: {0}")]
    WorktreePathExists(PathBuf),
    #[error("branch already exists: {0}")]
    BranchExists(String),
    #[error("branch '{branch}' is already checked out at {}", path.display())]
    BranchCheckedOut { branch: String, path: PathBuf },
    #[error("refusing to remove the base worktree: {0}")]
    CannotRemoveBase(PathBuf),
    #[error("cannot merge a detached-HEAD worktree (no branch to merge): {0}")]
    WorktreeDetached(PathBuf),
    #[error("cannot merge into a detached-HEAD base checkout")]
    BaseDetached,
    #[error("worktree name is empty after slugify: {0:?}")]
    InvalidName(String),
    #[error("repository has no commits yet; cannot create a worktree")]
    NoCommits,
    #[error("worktree not found under this repo: {0}")]
    WorktreeNotFound(PathBuf),
}

pub type Result<T> = std::result::Result<T, Error>;

const QUOTEPATH_OFF: [&str; 2] = ["-c", "core.quotepath=false"];

/// Run `git <args>` in `repo_root` and return stdout (lossy UTF-8).
async fn run_git(repo_root: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("not a git repository") {
            return Err(Error::NotARepo(repo_root.to_path_buf()));
        }
        return Err(Error::Git(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

/// The full pending diff (`git status --porcelain=v2` + `git diff HEAD`, falling
/// back to a diff against the empty tree when HEAD is unborn), parsed into hunks
/// and grouped by feature via `index.classify_paths`. Files in N features appear
/// in each group (`shared: true`); unmatched files land in the "unmapped" group.
pub async fn diff_by_feature(repo_root: &Path, index: &FeatureIndex) -> Result<DiffByFeature> {
    let files = collect_file_diffs(repo_root).await?;
    Ok(group::group_by_feature(
        files,
        |path| {
            let paths = [PathBuf::from(path)];
            index.classify_paths(&paths)
        },
        |slug| index.get(slug).map(|f| f.name.clone()),
    ))
}

/// `git rev-parse HEAD` — the current HEAD sha. `Ok(None)` on an unborn branch
/// (fresh repo with no commits yet).
pub async fn current_head(repo_root: &Path) -> Result<Option<String>> {
    let out = Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", "HEAD"])
        .current_dir(repo_root)
        .output()
        .await?;
    if out.status.success() {
        return Ok(Some(String::from_utf8_lossy(&out.stdout).trim().to_string()));
    }
    let stderr = String::from_utf8_lossy(&out.stderr);
    if stderr.contains("not a git repository") {
        return Err(Error::NotARepo(repo_root.to_path_buf()));
    }
    // --verify --quiet exits 1 with no stderr when HEAD is unverifiable (unborn).
    if out.status.code() == Some(1) {
        return Ok(None);
    }
    Err(Error::Git(stderr.trim().to_string()))
}

/// The current branch name (`git branch --show-current`). `Ok(None)` on a
/// detached HEAD or unborn branch — a real, distinct state, shown as such.
pub async fn current_branch(repo_root: &Path) -> Result<Option<String>> {
    let out = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(repo_root)
        .output()
        .await?;
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("not a git repository") {
            return Err(Error::NotARepo(repo_root.to_path_buf()));
        }
        return Err(Error::Git(stderr.trim().to_string()));
    }
    let name = String::from_utf8_lossy(&out.stdout).trim().to_string();
    Ok(if name.is_empty() { None } else { Some(name) })
}

/// Repo-relative paths of all changed (staged + unstaged + untracked) files,
/// from `git status --porcelain=v2`. Renames report the new path.
pub async fn changed_files(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let raw = run_git(repo_root, &status_args()).await?;
    let mut seen = std::collections::HashSet::new();
    Ok(status::parse_porcelain_v2(&raw)
        .into_iter()
        .map(|e| e.path)
        .filter(|p| seen.insert(p.clone()))
        .collect())
}

fn status_args() -> Vec<&'static str> {
    let mut args = QUOTEPATH_OFF.to_vec();
    args.extend(["status", "--porcelain=v2", "--untracked-files=all"]);
    args
}

/// All pending [`FileDiff`]s: parsed `git diff` output plus synthesized
/// all-added diffs for untracked text files.
pub(crate) async fn collect_file_diffs(repo_root: &Path) -> Result<Vec<FileDiff>> {
    let entries = status::parse_porcelain_v2(&run_git(repo_root, &status_args()).await?);

    let base = match current_head(repo_root).await? {
        Some(_) => "HEAD".to_string(),
        // Unborn HEAD: diff against the empty tree so staged files parse as added.
        None => run_git(repo_root, &["hash-object", "-t", "tree", "/dev/null"])
            .await?
            .trim()
            .to_string(),
    };
    let mut diff_args = QUOTEPATH_OFF.to_vec();
    diff_args.extend(["diff", "--no-color", "--unified=3", "--find-renames", &base]);
    let mut files = parse::parse_unified_diff(&run_git(repo_root, &diff_args).await?);

    for entry in entries.iter().filter(|e| e.status == "untracked") {
        if let Some(fd) = untracked::synthesize(repo_root, &entry.path) {
            files.push(fd);
        }
    }
    // One authoritative truncation over tracked + untracked diffs alike.
    truncate::cap_file_diffs(&mut files);
    tracing::debug!(files = files.len(), "collected pending diff");
    Ok(files)
}
