//! Git stash management types and traits.
//!
//! Provides types for representing stash entries and a trait for
//! stash operations (list, push, pop, apply, drop).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A single entry in the stash stack.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashEntry {
    /// The stash index (0 = most recent).
    pub index: usize,
    /// The stash reference (e.g., "stash@{0}").
    pub reference: String,
    /// The stash message / description.
    pub message: String,
    /// When the stash was created.
    pub date: DateTime<Utc>,
    /// The branch that was active when the stash was created.
    pub branch: Option<String>,
    /// The commit hash of the stash commit.
    pub commit_hash: String,
    /// Number of files modified in the stash.
    pub files_changed: Option<usize>,
    /// Whether the stash includes untracked files.
    pub includes_untracked: bool,
    /// Whether the stash includes ignored files.
    pub includes_ignored: bool,
}

impl StashEntry {
    /// Create a new stash entry.
    pub fn new(
        index: usize,
        message: impl Into<String>,
        commit_hash: impl Into<String>,
    ) -> Self {
        Self {
            index,
            reference: format!("stash@{{{index}}}"),
            message: message.into(),
            date: Utc::now(),
            branch: None,
            commit_hash: commit_hash.into(),
            files_changed: None,
            includes_untracked: false,
            includes_ignored: false,
        }
    }

    /// Set the branch.
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = Some(branch.into());
        self
    }

    /// Set the date.
    pub fn with_date(mut self, date: DateTime<Utc>) -> Self {
        self.date = date;
        self
    }

    /// Return the short commit hash (first 7 characters).
    pub fn short_hash(&self) -> &str {
        &self.commit_hash[..self.commit_hash.len().min(7)]
    }

    /// Return a descriptive label combining branch and message.
    pub fn label(&self) -> String {
        match &self.branch {
            Some(branch) => format!("On {}: {}", branch, self.message),
            None => self.message.clone(),
        }
    }
}

impl fmt::Display for StashEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.reference, self.label())
    }
}

/// Options for creating a new stash.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StashPushOptions {
    /// Custom message for the stash entry.
    pub message: Option<String>,
    /// Whether to include untracked files.
    pub include_untracked: bool,
    /// Whether to include ignored files.
    pub include_ignored: bool,
    /// Whether to keep the index staged changes.
    pub keep_index: bool,
    /// Specific paths to stash (empty = all changes).
    pub paths: Vec<String>,
}

impl StashPushOptions {
    /// Create default push options.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set a custom message.
    pub fn with_message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Include untracked files.
    pub fn with_untracked(mut self) -> Self {
        self.include_untracked = true;
        self
    }

    /// Keep staged changes in the index.
    pub fn with_keep_index(mut self) -> Self {
        self.keep_index = true;
        self
    }

    /// Stash only specific paths.
    pub fn with_paths(mut self, paths: Vec<String>) -> Self {
        self.paths = paths;
        self
    }
}

/// Options for applying or popping a stash.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StashApplyOptions {
    /// The stash index to apply (default 0).
    pub index: usize,
    /// Whether to try to reinstate the index (staged changes).
    pub reinstate_index: bool,
}

/// The result of a stash apply or pop operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StashApplyResult {
    /// Whether the apply succeeded cleanly.
    pub success: bool,
    /// Whether there were conflicts during the apply.
    pub has_conflicts: bool,
    /// List of files that conflicted.
    pub conflicted_files: Vec<String>,
    /// List of files that were successfully applied.
    pub applied_files: Vec<String>,
}

impl StashApplyResult {
    /// Create a successful result.
    pub fn success(applied_files: Vec<String>) -> Self {
        Self {
            success: true,
            has_conflicts: false,
            conflicted_files: Vec::new(),
            applied_files,
        }
    }

    /// Create a result with conflicts.
    pub fn with_conflicts(applied: Vec<String>, conflicted: Vec<String>) -> Self {
        Self {
            success: false,
            has_conflicts: true,
            conflicted_files: conflicted,
            applied_files: applied,
        }
    }
}

impl fmt::Display for StashApplyResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success {
            write!(f, "Applied {} files", self.applied_files.len())
        } else {
            write!(
                f,
                "Applied with {} conflicts in: {}",
                self.conflicted_files.len(),
                self.conflicted_files.join(", ")
            )
        }
    }
}

/// Trait for managing the stash stack.
pub trait StashManager {
    /// The error type for stash operations.
    type Error: std::error::Error;

    /// List all stash entries.
    fn list(&self) -> Result<Vec<StashEntry>, Self::Error>;

    /// Push (create) a new stash entry.
    fn push(&self, options: &StashPushOptions) -> Result<StashEntry, Self::Error>;

    /// Pop the most recent (or specified) stash entry, removing it from the stack.
    fn pop(&self, options: &StashApplyOptions) -> Result<StashApplyResult, Self::Error>;

    /// Apply a stash entry without removing it from the stack.
    fn apply(&self, options: &StashApplyOptions) -> Result<StashApplyResult, Self::Error>;

    /// Drop (delete) a stash entry.
    fn drop(&self, index: usize) -> Result<(), Self::Error>;

    /// Drop all stash entries.
    fn clear(&self) -> Result<(), Self::Error>;

    /// Show the diff of a stash entry.
    fn show(&self, index: usize) -> Result<String, Self::Error>;

    /// Create a branch from a stash entry.
    fn branch(
        &self,
        branch_name: &str,
        index: usize,
    ) -> Result<(), Self::Error>;
}

/// Summary of the stash stack.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StashSummary {
    /// Total number of stash entries.
    pub count: usize,
    /// Date of the most recent stash.
    pub most_recent: Option<DateTime<Utc>>,
    /// Date of the oldest stash.
    pub oldest: Option<DateTime<Utc>>,
    /// Distinct branches represented in the stash.
    pub branches: Vec<String>,
}

impl StashSummary {
    /// Build a summary from a list of stash entries.
    pub fn from_entries(entries: &[StashEntry]) -> Self {
        let mut branches: Vec<String> = entries
            .iter()
            .filter_map(|e| e.branch.clone())
            .collect();
        branches.sort();
        branches.dedup();

        Self {
            count: entries.len(),
            most_recent: entries.first().map(|e| e.date),
            oldest: entries.last().map(|e| e.date),
            branches,
        }
    }
}

impl fmt::Display for StashSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} stash entries", self.count)?;
        if !self.branches.is_empty() {
            write!(f, " across {} branches", self.branches.len())?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stash_entry_display() {
        let entry = StashEntry::new(0, "WIP on main", "abc1234def5678")
            .with_branch("main");
        assert_eq!(entry.to_string(), "stash@{0}: On main: WIP on main");
        assert_eq!(entry.short_hash(), "abc1234");
    }

    #[test]
    fn push_options() {
        let opts = StashPushOptions::new()
            .with_message("save progress")
            .with_untracked()
            .with_keep_index();
        assert_eq!(opts.message.as_deref(), Some("save progress"));
        assert!(opts.include_untracked);
        assert!(opts.keep_index);
    }

    #[test]
    fn apply_result_display() {
        let result = StashApplyResult::success(vec!["a.rs".to_string(), "b.rs".to_string()]);
        assert!(result.success);
        assert_eq!(result.to_string(), "Applied 2 files");
    }
}
