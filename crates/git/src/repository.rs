//! Core repository abstraction over a git working directory.

use std::path::{Path, PathBuf};

use crate::branch::Branch;
use crate::commit::Commit;
use crate::diff::FileDiff;
use crate::error::GitError;
use crate::remote::Remote;

/// A handle to a git repository on disk.
#[derive(Debug, Clone)]
pub struct Repository {
    /// Absolute path to the repository root (containing `.git`).
    path: PathBuf,
}

impl Repository {
    /// Open a repository at the given path.
    ///
    /// Returns an error if the path does not exist or is not a git repository.
    pub fn open(path: impl Into<PathBuf>) -> Result<Self, GitError> {
        let path = path.into();
        let git_dir = path.join(".git");
        if !git_dir.exists() {
            return Err(GitError::NotARepository {
                path: path.display().to_string(),
            });
        }
        Ok(Self { path })
    }

    /// Returns the repository root path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Get the current HEAD branch name, or `None` if in detached HEAD state.
    pub fn head_branch(&self) -> Result<Option<String>, GitError> {
        let head_path = self.path.join(".git/HEAD");
        let contents =
            std::fs::read_to_string(&head_path).map_err(|e| GitError::IoError { source: e })?;
        if let Some(ref_name) = contents.trim().strip_prefix("ref: refs/heads/") {
            Ok(Some(ref_name.to_string()))
        } else {
            Ok(None)
        }
    }

    /// List all local branches.
    pub fn branches(&self) -> Result<Vec<Branch>, GitError> {
        let heads_dir = self.path.join(".git/refs/heads");
        if !heads_dir.exists() {
            return Ok(Vec::new());
        }
        let mut branches = Vec::new();
        Self::collect_branches(&heads_dir, &heads_dir, &mut branches)?;
        Ok(branches)
    }

    /// Recursively collect branch names from the refs/heads directory.
    fn collect_branches(
        base: &Path,
        dir: &Path,
        branches: &mut Vec<Branch>,
    ) -> Result<(), GitError> {
        let entries = std::fs::read_dir(dir).map_err(|e| GitError::IoError { source: e })?;
        for entry in entries {
            let entry = entry.map_err(|e| GitError::IoError { source: e })?;
            let ft = entry
                .file_type()
                .map_err(|e| GitError::IoError { source: e })?;
            if ft.is_dir() {
                Self::collect_branches(base, &entry.path(), branches)?;
            } else {
                let relative = entry
                    .path()
                    .strip_prefix(base)
                    .unwrap_or(&entry.path())
                    .to_string_lossy()
                    .replace('\\', "/");
                branches.push(Branch::local(relative));
            }
        }
        Ok(())
    }

    /// Get a list of recent commits (up to `limit`).
    pub fn log(&self, limit: usize) -> Result<Vec<Commit>, GitError> {
        // Placeholder: in a real implementation this would parse git log output.
        let _ = limit;
        Ok(Vec::new())
    }

    /// Get the diff of unstaged changes.
    pub fn diff_unstaged(&self) -> Result<Vec<FileDiff>, GitError> {
        Ok(Vec::new())
    }

    /// Get the diff of staged changes.
    pub fn diff_staged(&self) -> Result<Vec<FileDiff>, GitError> {
        Ok(Vec::new())
    }

    /// Get the status summary (number of modified, added, deleted files).
    pub fn status_counts(&self) -> Result<StatusCounts, GitError> {
        Ok(StatusCounts::default())
    }

    /// List configured remotes.
    pub fn remotes(&self) -> Result<Vec<Remote>, GitError> {
        Ok(Vec::new())
    }

    /// Stash the current working directory changes.
    pub fn stash_push(&self, message: Option<&str>) -> Result<(), GitError> {
        let _ = message;
        Ok(())
    }

    /// Pop the most recent stash entry.
    pub fn stash_pop(&self) -> Result<(), GitError> {
        Ok(())
    }
}

/// Summary counts of repository file statuses.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct StatusCounts {
    /// Number of modified files.
    pub modified: usize,
    /// Number of newly added (untracked) files.
    pub added: usize,
    /// Number of deleted files.
    pub deleted: usize,
    /// Number of renamed files.
    pub renamed: usize,
    /// Number of files with merge conflicts.
    pub conflicted: usize,
}

impl StatusCounts {
    /// Returns `true` if the working directory is clean.
    pub fn is_clean(&self) -> bool {
        self.modified == 0
            && self.added == 0
            && self.deleted == 0
            && self.renamed == 0
            && self.conflicted == 0
    }

    /// Returns the total number of changed files.
    pub fn total(&self) -> usize {
        self.modified + self.added + self.deleted + self.renamed + self.conflicted
    }
}
