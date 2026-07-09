//! Pure helpers for worktree ops: porcelain parsing, ref selection, slugify.
//! No IO — unit-tested directly.

use std::path::{Path, PathBuf};

/// One entry of `git worktree list --porcelain`, before enrichment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct RawWorktree {
    pub path: PathBuf,
    pub head: Option<String>,
    /// Short branch name (`refs/heads/` stripped); `None` when detached/bare.
    pub branch: Option<String>,
    pub bare: bool,
    pub detached: bool,
}

/// Parse `git worktree list --porcelain`. Entries are blank-line separated; the
/// main worktree is always the first entry.
pub(crate) fn parse_worktree_porcelain(raw: &str) -> Vec<RawWorktree> {
    let mut out = Vec::new();
    let mut cur: Option<RawWorktree> = None;
    for line in raw.lines() {
        if line.is_empty() {
            out.extend(cur.take());
            continue;
        }
        if let Some(path) = line.strip_prefix("worktree ") {
            out.extend(cur.take());
            cur = Some(RawWorktree {
                path: PathBuf::from(path),
                head: None,
                branch: None,
                bare: false,
                detached: false,
            });
        } else if let Some(w) = cur.as_mut() {
            if let Some(h) = line.strip_prefix("HEAD ") {
                w.head = Some(h.to_string());
            } else if let Some(b) = line.strip_prefix("branch ") {
                w.branch = Some(b.strip_prefix("refs/heads/").unwrap_or(b).to_string());
            } else if line == "detached" {
                w.detached = true;
            } else if line == "bare" {
                w.bare = true;
            }
            // "locked"/"prunable" carry no info we surface.
        }
    }
    out.extend(cur.take());
    out
}

/// The ref used to place a worktree in ahead/behind space: its branch (as a full
/// `refs/heads/` ref) or, when detached, its HEAD sha. `None` for a bare/unborn
/// entry with neither — those compare as 0/0.
pub(crate) fn compare_ref(w: &RawWorktree) -> Option<String> {
    match &w.branch {
        Some(b) => Some(format!("refs/heads/{b}")),
        None => w.head.clone(),
    }
}

pub(crate) fn dir_name(path: &Path) -> String {
    path.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

/// Lowercase alphanumeric slug; runs of other chars collapse to a single `-`.
pub(crate) fn slugify(name: &str) -> String {
    let mut s = String::with_capacity(name.len());
    for c in name.chars() {
        if c.is_ascii_alphanumeric() {
            s.extend(c.to_lowercase());
        } else if !s.is_empty() && !s.ends_with('-') {
            s.push('-');
        }
    }
    s.trim_matches('-').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_branch_detached_and_bare_entries() {
        let raw = "\
worktree /repo/main
HEAD aaaa1111
branch refs/heads/main

worktree /repo/.featureforge-worktrees/feat
HEAD bbbb2222
branch refs/heads/feat

worktree /repo/.featureforge-worktrees/loose
HEAD cccc3333
detached
";
        let got = parse_worktree_porcelain(raw);
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].path, PathBuf::from("/repo/main"));
        assert_eq!(got[0].branch.as_deref(), Some("main"));
        assert!(!got[0].detached);
        assert_eq!(got[1].branch.as_deref(), Some("feat"));
        assert_eq!(got[2].branch, None);
        assert!(got[2].detached);
        assert_eq!(got[2].head.as_deref(), Some("cccc3333"));
    }

    #[test]
    fn bare_entry_has_no_branch_or_head() {
        let raw = "worktree /repo/bare\nbare\n";
        let got = parse_worktree_porcelain(raw);
        assert_eq!(got.len(), 1);
        assert!(got[0].bare);
        assert_eq!(got[0].branch, None);
        assert_eq!(got[0].head, None);
    }

    #[test]
    fn slugify_collapses_and_trims() {
        assert_eq!(slugify("Fix Auth Bug!"), "fix-auth-bug");
        assert_eq!(slugify("  --Weird__Name-- "), "weird-name");
        assert_eq!(slugify("已经"), "");
    }
}
