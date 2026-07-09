//! Small pure helpers for repo paths and [`RepoState`] assembly.

use std::path::{Path, PathBuf};

use forge_core::RepoState;

/// Canonicalize an incoming repo path (also validates existence). Used as the
/// `state.repos` map key so every command resolves the same key.
pub fn canonical<P: AsRef<Path>>(path: P) -> Result<PathBuf, String> {
    let path = path.as_ref();
    std::fs::canonicalize(path).map_err(|e| format!("invalid repo path {}: {e}", path.display()))
}

/// A directory is treated as a git repo when it has a `.git` entry (dir for a
/// normal clone, file for a worktree/submodule).
pub fn is_git_repo(root: &Path) -> bool {
    root.join(".git").exists()
}

/// Display name for a repo = its directory name.
pub fn repo_name(root: &Path) -> String {
    root.file_name()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| root.display().to_string())
}

/// Assemble the [`RepoState`] returned by `open_repo` / emitted on `repo:changed`.
///
/// `indexed_at` is left `None` here: CONTRACT-3 makes `repos.indexed_at` (DB)
/// the single source of truth, and this pure helper cannot reach the DB. A
/// DB-aware caller must set `state.indexed_at` from `repos.indexed_at` after
/// building the state (`open_repo` does; the reindex emitter must too).
pub fn repo_state(root: &Path, features_count: u32, daemon_port: Option<u16>) -> RepoState {
    RepoState {
        path: root.to_path_buf(),
        name: repo_name(root),
        features_count,
        indexed_at: None,
        daemon_port,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ff-repoutil-{label}-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn repo_name_is_dir_name() {
        let dir = tmp("myrepo");
        assert!(repo_name(&dir).starts_with("ff-repoutil-myrepo-"));
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn repo_state_reflects_features_and_port() {
        let dir = tmp("state");
        let st = repo_state(&dir, 5, Some(49000));
        assert_eq!(st.features_count, 5);
        assert_eq!(st.daemon_port, Some(49000));
        // indexed_at is DB-authoritative (CONTRACT-3): repo_state never derives it.
        assert!(st.indexed_at.is_none());
        assert_eq!(st.path, dir);
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn is_git_repo_detects_dot_git() {
        let dir = tmp("git");
        assert!(!is_git_repo(&dir));
        std::fs::create_dir_all(dir.join(".git")).unwrap();
        assert!(is_git_repo(&dir));
        std::fs::remove_dir_all(&dir).ok();
    }
}
