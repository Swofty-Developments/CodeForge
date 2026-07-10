//! Branch enumeration + checking EXISTING branches out into worktrees.
//!
//! Complements `worktree.rs` (which spins a NEW branch off the base): here the
//! branch already exists — locally, or as a remote-tracking ref — and gets its
//! own worktree under `.codeforge-worktrees/`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use forge_core::Worktree;
use serde::{Deserialize, Serialize};

use crate::worktree::{base_repo_root, enriched_worktree, raw_worktrees, run_git_raw};
use crate::worktree_parse::slugify;
use crate::{run_git, Error, Result};

/// One branch of the repo family: a local head or a remote-tracking ref.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchInfo {
    /// Short name, e.g. `feat/x` (no `refs/heads/` or `<remote>/` prefix).
    pub name: String,
    /// `None` for a local branch, else the remote name, e.g. `origin`.
    pub remote: Option<String>,
    /// Checked out at the base worktree's HEAD.
    pub is_head: bool,
    /// Absolute worktree path where this branch is checked out, else `None`.
    pub checked_out_at: Option<PathBuf>,
}

/// A parsed `for-each-ref` line: (short name, remote, is_head).
type RefLine = (String, Option<String>, bool);

/// Parse `for-each-ref refs/heads refs/remotes` output in the
/// `%(refname)%00%(HEAD)%00%(symref)` format. Symbolic refs (`origin/HEAD`) are
/// skipped — they alias a branch, they aren't one; any other unrecognized line
/// is a named parse error, never silently dropped.
fn parse_ref_lines(raw: &str) -> Result<Vec<RefLine>> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let mut parts = line.splitn(3, '\0');
        let (Some(refname), Some(head), Some(symref)) = (parts.next(), parts.next(), parts.next())
        else {
            return Err(Error::Git(format!("unexpected for-each-ref line: {line:?}")));
        };
        if !symref.is_empty() {
            continue;
        }
        if let Some(name) = refname.strip_prefix("refs/heads/") {
            out.push((name.to_string(), None, head == "*"));
        } else if let Some(rest) = refname.strip_prefix("refs/remotes/") {
            let Some((remote, name)) = rest.split_once('/') else {
                return Err(Error::Git(format!("unexpected remote-tracking ref: {refname:?}")));
            };
            out.push((name.to_string(), Some(remote.to_string()), false));
        } else {
            return Err(Error::Git(format!("ref outside heads/remotes: {refname:?}")));
        }
    }
    Ok(out)
}

/// All branches of the repo family `repo_root` belongs to: local heads first,
/// then remote-tracking branches, each alphabetical. Local branches are
/// cross-referenced against `git worktree list` to fill `checked_out_at` (the
/// base checkout included); `is_head` marks the base HEAD's branch.
pub async fn list_branches(repo_root: &Path) -> Result<Vec<BranchInfo>> {
    let base = base_repo_root(repo_root).await?;
    let raw = run_git(
        &base,
        &[
            "for-each-ref",
            "refs/heads",
            "refs/remotes",
            "--format=%(refname)%00%(HEAD)%00%(symref)",
        ],
    )
    .await?;
    let refs = parse_ref_lines(&raw)?;

    let checkouts: HashMap<String, PathBuf> = raw_worktrees(&base)
        .await?
        .into_iter()
        .filter_map(|w| w.branch.clone().map(|b| (b, w.path)))
        .collect();

    let mut locals = Vec::new();
    let mut remotes = Vec::new();
    for (name, remote, is_head) in refs {
        match remote {
            None => locals.push(BranchInfo {
                checked_out_at: checkouts.get(&name).cloned(),
                is_head,
                remote: None,
                name,
            }),
            Some(r) => remotes.push(BranchInfo {
                name,
                remote: Some(r),
                is_head: false,
                checked_out_at: None,
            }),
        }
    }
    locals.sort_by(|a, b| a.name.cmp(&b.name));
    remotes.sort_by(|a, b| (&a.remote, &a.name).cmp(&(&b.remote, &b.name)));
    locals.extend(remotes);
    Ok(locals)
}

/// `git fetch --all --prune` in the base. A repo with no remotes is a real,
/// fine state — nothing to fetch, logged and reported as `Ok`. A failing fetch
/// surfaces git's stderr verbatim.
pub async fn fetch_remotes(repo_root: &Path) -> Result<()> {
    let base = base_repo_root(repo_root).await?;
    if run_git(&base, &["remote"]).await?.trim().is_empty() {
        tracing::info!(repo = %base.display(), "no remotes configured; nothing to fetch");
        return Ok(());
    }
    run_git(&base, &["fetch", "--all", "--prune"]).await?;
    Ok(())
}

/// Check an EXISTING branch out into a new worktree at
/// `<base>/.codeforge-worktrees/<slug(branch)>`.
///
/// * `remote: None` — check out the local branch (`git worktree add`, no `-b`).
///   Already checked out somewhere (the base included) is the named
///   [`Error::BranchCheckedOut`], carrying that worktree's path so the caller
///   can offer to open it instead.
/// * `remote: Some(r)` — create local `branch` tracking `r/branch`
///   (`git worktree add --track -b`). An existing local branch of that name is
///   the named [`Error::BranchExists`]: pick the local branch instead.
pub async fn worktree_for_branch(
    repo_root: &Path,
    branch: &str,
    remote: Option<&str>,
) -> Result<Worktree> {
    let base = base_repo_root(repo_root).await?;
    let slug = slugify(branch);
    if slug.is_empty() {
        return Err(Error::InvalidName(branch.to_string()));
    }
    let path = base.join(".codeforge-worktrees").join(&slug);
    if path.exists() {
        return Err(Error::WorktreePathExists(path));
    }
    let path_str = path.to_string_lossy().into_owned();

    match remote {
        None => {
            if let Some(w) = raw_worktrees(&base)
                .await?
                .into_iter()
                .find(|w| w.branch.as_deref() == Some(branch))
            {
                return Err(Error::BranchCheckedOut { branch: branch.to_string(), path: w.path });
            }
            run_git(&base, &["worktree", "add", &path_str, branch]).await?;
        }
        Some(r) => {
            let head_ref = format!("refs/heads/{branch}");
            if run_git_raw(&base, &["show-ref", "--verify", "--quiet", &head_ref])
                .await?
                .status
                .success()
            {
                return Err(Error::BranchExists(format!(
                    "{branch}; pick the local branch instead of {r}/{branch}"
                )));
            }
            let start = format!("{r}/{branch}");
            run_git(&base, &["worktree", "add", "--track", "-b", branch, &path_str, &start])
                .await?;
        }
    }
    enriched_worktree(&base, &path).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_locals_remotes_and_skips_symrefs() {
        let raw = "refs/heads/main\0*\0\n\
                   refs/heads/feat/x\0 \0\n\
                   refs/remotes/origin/HEAD\0 \0refs/remotes/origin/main\n\
                   refs/remotes/origin/feat/x\0 \0\n";
        let got = parse_ref_lines(raw).unwrap();
        assert_eq!(
            got,
            vec![
                ("main".to_string(), None, true),
                ("feat/x".to_string(), None, false),
                ("feat/x".to_string(), Some("origin".to_string()), false),
            ]
        );
    }

    #[test]
    fn malformed_lines_are_named_errors() {
        assert!(parse_ref_lines("refs/heads/x").is_err(), "missing fields");
        assert!(parse_ref_lines("refs/tags/v1\0 \0\n").is_err(), "ref outside heads/remotes");
        assert!(parse_ref_lines("refs/remotes/lonely\0 \0\n").is_err(), "remote ref w/o branch");
    }
}
