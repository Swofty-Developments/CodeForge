//! Git worktree operations: list / create / remove / merge.
//!
//! All state-mutating git runs against the USER's target repos (this is the
//! feature). Every distinct state is named honestly — detached HEAD, no
//! merge-base, merge conflict — never papered over with a silent fallback.

use std::path::{Path, PathBuf};

use forge_core::{MergeResult, Worktree};
use tokio::process::Command;

use crate::worktree_parse::{compare_ref, dir_name, parse_worktree_porcelain, slugify, RawWorktree};
use crate::{run_git, Error, Result};

pub(crate) async fn raw_worktrees(repo_root: &Path) -> Result<Vec<RawWorktree>> {
    let raw = run_git(repo_root, &["worktree", "list", "--porcelain"]).await?;
    Ok(parse_worktree_porcelain(&raw))
}

/// The main worktree of the repo `repo_root` belongs to — the first entry of
/// `git worktree list --porcelain`. Worktree ops key off this path.
pub async fn base_repo_root(repo_root: &Path) -> Result<PathBuf> {
    raw_worktrees(repo_root)
        .await?
        .into_iter()
        .next()
        .map(|w| w.path)
        .ok_or_else(|| Error::NotARepo(repo_root.to_path_buf()))
}

/// `git rev-list --left-right --count base...branch` → (ahead, behind), where
/// ahead = commits on `branch` not on `base`, behind = the reverse. Returns
/// (0,0) when the two share no merge-base (a named, honest state).
async fn ahead_behind(base_repo: &Path, base_ref: &str, branch_ref: &str) -> Result<(u32, u32)> {
    let mb = run_git_raw(base_repo, &["merge-base", base_ref, branch_ref]).await?;
    if !mb.status.success() {
        return Ok((0, 0));
    }
    let range = format!("{base_ref}...{branch_ref}");
    let out = run_git(base_repo, &["rev-list", "--left-right", "--count", &range]).await?;
    let nums: Vec<&str> = out.split_whitespace().collect();
    if nums.len() != 2 {
        return Err(Error::Git(format!("unexpected rev-list output: {out:?}")));
    }
    let behind = nums[0]
        .parse()
        .map_err(|_| Error::Git(format!("bad behind count: {:?}", nums[0])))?;
    let ahead = nums[1]
        .parse()
        .map_err(|_| Error::Git(format!("bad ahead count: {:?}", nums[1])))?;
    Ok((ahead, behind))
}

async fn is_dirty(w: &RawWorktree) -> Result<bool> {
    if w.bare {
        return Ok(false);
    }
    let out = run_git(&w.path, &["status", "--porcelain"]).await?;
    Ok(!out.trim().is_empty())
}

/// All worktrees of the repo `repo_root` belongs to, base first. ahead/behind is
/// measured against the base worktree's branch (its HEAD sha when detached).
pub async fn list_worktrees(repo_root: &Path) -> Result<Vec<Worktree>> {
    let entries = raw_worktrees(repo_root).await?;
    let base = entries
        .first()
        .ok_or_else(|| Error::NotARepo(repo_root.to_path_buf()))?;
    let base_path = base.path.clone();
    let base_ref = compare_ref(base);

    let mut out = Vec::with_capacity(entries.len());
    for e in &entries {
        let is_base = e.path == base_path;
        let (ahead, behind) = match (is_base, &base_ref, compare_ref(e)) {
            (false, Some(br), Some(er)) => ahead_behind(&base_path, br, &er).await?,
            _ => (0, 0),
        };
        let name = e.branch.clone().unwrap_or_else(|| dir_name(&e.path));
        out.push(Worktree {
            path: e.path.clone(),
            name,
            branch: e.branch.clone(),
            is_base,
            ahead,
            behind,
            dirty: is_dirty(e).await?,
        });
    }
    Ok(out)
}

/// Create a new worktree on branch `<slug of name>` at
/// `<base>/.codeforge-worktrees/<slug>`, off `base_ref` (default: the repo's
/// current branch, or its HEAD sha when detached). An existing branch or path is
/// a named error — never silently reused.
pub async fn create_worktree(
    repo_root: &Path,
    name: &str,
    base_ref: Option<&str>,
) -> Result<Worktree> {
    let base = base_repo_root(repo_root).await?;
    let slug = slugify(name);
    if slug.is_empty() {
        return Err(Error::InvalidName(name.to_string()));
    }
    let path = base.join(".codeforge-worktrees").join(&slug);
    if path.exists() {
        return Err(Error::WorktreePathExists(path));
    }
    let head_ref = format!("refs/heads/{slug}");
    if run_git_raw(&base, &["show-ref", "--verify", "--quiet", &head_ref])
        .await?
        .status
        .success()
    {
        return Err(Error::BranchExists(slug));
    }

    let resolved_ref = match base_ref {
        Some(r) => r.to_string(),
        None => match crate::current_branch(repo_root).await? {
            Some(b) => b,
            None => crate::current_head(repo_root).await?.ok_or(Error::NoCommits)?,
        },
    };

    let path_str = path.to_string_lossy().into_owned();
    let add = run_git_raw(
        &base,
        &["worktree", "add", "-b", &slug, &path_str, &resolved_ref],
    )
    .await?;
    if !add.status.success() {
        return Err(Error::Git(
            String::from_utf8_lossy(&add.stderr).trim().to_string(),
        ));
    }

    enriched_worktree(&base, &path).await
}

/// Canonicalize a freshly added worktree path and return its enriched
/// [`Worktree`] entry from [`list_worktrees`].
pub(crate) async fn enriched_worktree(base: &Path, path: &Path) -> Result<Worktree> {
    let canonical = std::fs::canonicalize(path)?;
    list_worktrees(base)
        .await?
        .into_iter()
        .find(|w| w.path == canonical)
        .ok_or_else(|| Error::WorktreeNotFound(path.to_path_buf()))
}

/// Whether two paths resolve to the same location (symlinks resolved when the
/// path exists — macOS `/var` vs `/private/var`).
fn same_path(a: &Path, b: &Path) -> bool {
    let ca = std::fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let cb = std::fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    ca == cb
}

/// `git worktree remove [--force] <path>`. Refuses to remove the base worktree.
/// Git's own error (dirty worktree without `force`, etc.) is surfaced verbatim.
pub async fn remove_worktree(repo_root: &Path, worktree_path: &Path, force: bool) -> Result<()> {
    let base = base_repo_root(repo_root).await?;
    if same_path(&base, worktree_path) {
        return Err(Error::CannotRemoveBase(base));
    }
    let path_str = worktree_path.to_string_lossy().into_owned();
    let mut args = vec!["worktree", "remove"];
    if force {
        args.push("--force");
    }
    args.push(&path_str);
    let out = run_git_raw(&base, &args).await?;
    if !out.status.success() {
        return Err(Error::Git(
            String::from_utf8_lossy(&out.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// Merge the worktree's branch into the base repo's current branch. A conflict
/// is reported (conflicted paths) AND aborted so the base is never left
/// mid-merge; a merge that can't even start (dirty base, bad ref) surfaces as an
/// error with the base untouched.
pub async fn merge_worktree(base_repo_root_arg: &Path, worktree_path: &Path) -> Result<MergeResult> {
    let base = base_repo_root(base_repo_root_arg).await?;
    let entries = raw_worktrees(&base).await?;
    let entry = entries
        .iter()
        .find(|w| same_path(&w.path, worktree_path))
        .ok_or_else(|| Error::WorktreeNotFound(worktree_path.to_path_buf()))?;
    let source_branch = entry
        .branch
        .clone()
        .ok_or_else(|| Error::WorktreeDetached(worktree_path.to_path_buf()))?;
    let target_branch = crate::current_branch(&base).await?.ok_or(Error::BaseDetached)?;

    let merge = run_git_raw(&base, &["merge", "--no-edit", &source_branch]).await?;
    if merge.status.success() {
        return Ok(MergeResult {
            merged: true,
            conflicts: Vec::new(),
            aborted: false,
            message: String::from_utf8_lossy(&merge.stdout).trim().to_string(),
            source_branch,
            target_branch,
        });
    }

    let message = {
        let mut m = String::from_utf8_lossy(&merge.stdout).into_owned();
        m.push_str(&String::from_utf8_lossy(&merge.stderr));
        m.trim().to_string()
    };
    // No MERGE_HEAD → the merge never started (dirty base / bad ref); base is
    // untouched. Surface it rather than calling a spurious `merge --abort`.
    let in_progress = run_git_raw(&base, &["rev-parse", "-q", "--verify", "MERGE_HEAD"])
        .await?
        .status
        .success();
    if !in_progress {
        return Err(Error::Git(message));
    }

    let conflicts_raw = run_git(&base, &["diff", "--name-only", "--diff-filter=U"]).await?;
    let conflicts: Vec<String> = conflicts_raw
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();

    let abort = run_git_raw(&base, &["merge", "--abort"]).await?;
    if !abort.status.success() {
        return Err(Error::Git(format!(
            "merge conflicted and `git merge --abort` failed: {}",
            String::from_utf8_lossy(&abort.stderr).trim()
        )));
    }
    Ok(MergeResult {
        merged: false,
        conflicts,
        aborted: true,
        message,
        source_branch,
        target_branch,
    })
}

/// Run `git <args>` in `cwd`, returning the raw `Output` (status + streams) so
/// the caller can branch on a non-zero exit (expected for conflicts / probes).
pub(crate) async fn run_git_raw(cwd: &Path, args: &[&str]) -> Result<std::process::Output> {
    Ok(Command::new("git").args(args).current_dir(cwd).output().await?)
}
