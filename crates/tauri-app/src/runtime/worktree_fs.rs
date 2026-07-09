//! CodeForge-specific filesystem setup applied to a freshly created worktree
//! (contract W4): copy the base repo's `.codeforge/features.json` into the
//! new worktree so it inherits the feature model immediately, and keep
//! `.codeforge-worktrees/` ignored in the base repo. Pure fs; git lives in
//! git worktree ops (see `forge_git`).

use std::path::Path;

/// The directory (under the base repo) that holds all CodeForge worktrees.
const WORKTREES_DIR_IGNORE: &str = ".codeforge-worktrees/";
const FEATURES_REL: &str = ".codeforge/features.json";

/// Copy `<base>/.codeforge/features.json` into `<worktree>/.codeforge/`.
///
/// Returns `Ok(true)` when the file was copied, `Ok(false)` when the base has no
/// `features.json` yet — a real "base not indexed" state (the worktree simply
/// starts empty and gets its own cold-start on open), NOT an error to hide.
pub fn inherit_features(base: &Path, worktree: &Path) -> Result<bool, String> {
    let src = base.join(FEATURES_REL);
    if !src.exists() {
        tracing::info!(base = %base.display(), "base repo has no features.json yet; worktree starts unindexed");
        return Ok(false);
    }
    let dst = worktree.join(FEATURES_REL);
    if let Some(parent) = dst.parent() {
        std::fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    std::fs::copy(&src, &dst).map_err(|e| format!("copy features.json into worktree: {e}"))?;
    Ok(true)
}

/// Ensure `.codeforge-worktrees/` is ignored in the base repo's `.gitignore`
/// (idempotent read-modify-write). A missing `.gitignore` is created.
pub fn ensure_worktrees_ignored(base: &Path) -> Result<(), String> {
    let path = base.join(".gitignore");
    let existing = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(format!("read {}: {e}", path.display())),
    };

    if existing.lines().any(|l| l.trim() == WORKTREES_DIR_IGNORE) {
        return Ok(()); // already ignored — nothing to do
    }

    let mut next = existing;
    if !next.is_empty() && !next.ends_with('\n') {
        next.push('\n');
    }
    next.push_str(WORKTREES_DIR_IGNORE);
    next.push('\n');
    std::fs::write(&path, next).map_err(|e| format!("write {}: {e}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ff-wtfs-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ensure_worktrees_ignored_is_idempotent() {
        let base = tmp("ignore");
        // Create from scratch.
        ensure_worktrees_ignored(&base).unwrap();
        let first = std::fs::read_to_string(base.join(".gitignore")).unwrap();
        assert_eq!(first, ".codeforge-worktrees/\n");
        // Second call must not duplicate the entry.
        ensure_worktrees_ignored(&base).unwrap();
        let second = std::fs::read_to_string(base.join(".gitignore")).unwrap();
        assert_eq!(second, first);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn ensure_worktrees_ignored_preserves_existing_lines() {
        let base = tmp("preserve");
        std::fs::write(base.join(".gitignore"), "target/\nnode_modules/").unwrap(); // no trailing newline
        ensure_worktrees_ignored(&base).unwrap();
        let text = std::fs::read_to_string(base.join(".gitignore")).unwrap();
        assert_eq!(text, "target/\nnode_modules/\n.codeforge-worktrees/\n");
        // Idempotent even when the entry sits at the end.
        ensure_worktrees_ignored(&base).unwrap();
        let again = std::fs::read_to_string(base.join(".gitignore")).unwrap();
        assert_eq!(again, text);
        std::fs::remove_dir_all(&base).ok();
    }

    #[test]
    fn inherit_features_copies_when_present_and_skips_when_absent() {
        let base = tmp("inherit-base");
        let worktree = tmp("inherit-wt");

        // Base not indexed yet → skip, reported as false (not an error).
        assert!(!inherit_features(&base, &worktree).unwrap());
        assert!(!worktree.join(FEATURES_REL).exists());

        // Now the base has a features.json → it is copied verbatim.
        std::fs::create_dir_all(base.join(".codeforge")).unwrap();
        std::fs::write(base.join(FEATURES_REL), "[{\"slug\":\"x\"}]\n").unwrap();
        assert!(inherit_features(&base, &worktree).unwrap());
        let copied = std::fs::read_to_string(worktree.join(FEATURES_REL)).unwrap();
        assert_eq!(copied, "[{\"slug\":\"x\"}]\n");

        std::fs::remove_dir_all(&base).ok();
        std::fs::remove_dir_all(&worktree).ok();
    }
}
