//! Git submodule types and management.
//!
//! Provides structs for representing submodule state, configuration,
//! and operations like initialization, update, and synchronization.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::{Path, PathBuf};

/// The current status of a submodule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SubmoduleStatus {
    /// Submodule is checked out at the expected commit.
    Clean,
    /// Submodule has local modifications.
    Modified,
    /// Submodule is registered but not yet initialized.
    Uninitialized,
    /// Submodule is checked out at a different commit than expected.
    OutOfDate,
    /// Submodule has merge conflicts.
    Conflicted,
    /// Submodule directory is missing.
    Missing,
}

impl SubmoduleStatus {
    /// Return the status indicator character used by `git submodule status`.
    pub fn indicator(&self) -> char {
        match self {
            SubmoduleStatus::Clean => ' ',
            SubmoduleStatus::Modified => '+',
            SubmoduleStatus::Uninitialized => '-',
            SubmoduleStatus::OutOfDate => 'U',
            SubmoduleStatus::Conflicted => 'C',
            SubmoduleStatus::Missing => '!',
        }
    }

    /// Whether the submodule needs attention.
    pub fn needs_action(&self) -> bool {
        !matches!(self, SubmoduleStatus::Clean)
    }
}

impl fmt::Display for SubmoduleStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SubmoduleStatus::Clean => write!(f, "clean"),
            SubmoduleStatus::Modified => write!(f, "modified"),
            SubmoduleStatus::Uninitialized => write!(f, "uninitialized"),
            SubmoduleStatus::OutOfDate => write!(f, "out of date"),
            SubmoduleStatus::Conflicted => write!(f, "conflicted"),
            SubmoduleStatus::Missing => write!(f, "missing"),
        }
    }
}

/// Strategy for updating a submodule.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UpdateStrategy {
    /// Checkout the recorded commit (default).
    Checkout,
    /// Rebase local work on top of the recorded commit.
    Rebase,
    /// Merge the recorded commit into local work.
    Merge,
    /// Do not update automatically.
    None,
}

impl Default for UpdateStrategy {
    fn default() -> Self {
        UpdateStrategy::Checkout
    }
}

impl fmt::Display for UpdateStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            UpdateStrategy::Checkout => write!(f, "checkout"),
            UpdateStrategy::Rebase => write!(f, "rebase"),
            UpdateStrategy::Merge => write!(f, "merge"),
            UpdateStrategy::None => write!(f, "none"),
        }
    }
}

/// A git submodule within a repository.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Submodule {
    /// The logical name of the submodule.
    pub name: String,
    /// The path relative to the repository root.
    pub path: PathBuf,
    /// The remote URL of the submodule.
    pub url: String,
    /// The expected commit hash (from the superproject tree).
    pub expected_commit: Option<String>,
    /// The actual checked-out commit hash.
    pub actual_commit: Option<String>,
    /// Current status of the submodule.
    pub status: SubmoduleStatus,
    /// The branch to track (if configured).
    pub branch: Option<String>,
    /// The update strategy.
    pub update_strategy: UpdateStrategy,
    /// Whether shallow clone is enabled.
    pub shallow: bool,
    /// Whether recursive submodule init is enabled.
    pub recursive: bool,
}

impl Submodule {
    /// Create a new submodule descriptor.
    pub fn new(name: impl Into<String>, path: impl Into<PathBuf>, url: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            path: path.into(),
            url: url.into(),
            expected_commit: None,
            actual_commit: None,
            status: SubmoduleStatus::Uninitialized,
            branch: None,
            update_strategy: UpdateStrategy::default(),
            shallow: false,
            recursive: false,
        }
    }

    /// Set the expected commit.
    pub fn with_expected_commit(mut self, hash: impl Into<String>) -> Self {
        self.expected_commit = Some(hash.into());
        self
    }

    /// Set the actual commit and derive status.
    pub fn with_actual_commit(mut self, hash: impl Into<String>) -> Self {
        let hash = hash.into();
        self.status = match &self.expected_commit {
            Some(expected) if *expected == hash => SubmoduleStatus::Clean,
            Some(_) => SubmoduleStatus::OutOfDate,
            None => SubmoduleStatus::Modified,
        };
        self.actual_commit = Some(hash);
        self
    }

    /// Whether the submodule is at the expected commit.
    pub fn is_clean(&self) -> bool {
        self.status == SubmoduleStatus::Clean
    }

    /// Whether the submodule needs to be initialized.
    pub fn needs_init(&self) -> bool {
        self.status == SubmoduleStatus::Uninitialized
    }

    /// Whether the submodule needs to be updated.
    pub fn needs_update(&self) -> bool {
        matches!(
            self.status,
            SubmoduleStatus::OutOfDate | SubmoduleStatus::Uninitialized
        )
    }

    /// Return the short hash of the expected commit (first 8 chars).
    pub fn short_expected_hash(&self) -> Option<&str> {
        self.expected_commit
            .as_ref()
            .map(|h| &h[..h.len().min(8)])
    }

    /// Return the short hash of the actual commit (first 8 chars).
    pub fn short_actual_hash(&self) -> Option<&str> {
        self.actual_commit
            .as_ref()
            .map(|h| &h[..h.len().min(8)])
    }
}

impl fmt::Display for Submodule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}{} {} ({})",
            self.status.indicator(),
            self.short_actual_hash().unwrap_or("--------"),
            self.path.display(),
            self.status
        )
    }
}

/// Summary of submodule states across a repository.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SubmoduleSummary {
    /// Total number of submodules.
    pub total: usize,
    /// Number of clean submodules.
    pub clean: usize,
    /// Number of modified submodules.
    pub modified: usize,
    /// Number of uninitialized submodules.
    pub uninitialized: usize,
    /// Number of out-of-date submodules.
    pub out_of_date: usize,
    /// Number of missing submodules.
    pub missing: usize,
}

impl SubmoduleSummary {
    /// Build a summary from a list of submodules.
    pub fn from_submodules(submodules: &[Submodule]) -> Self {
        let mut summary = Self {
            total: submodules.len(),
            ..Default::default()
        };
        for sub in submodules {
            match sub.status {
                SubmoduleStatus::Clean => summary.clean += 1,
                SubmoduleStatus::Modified => summary.modified += 1,
                SubmoduleStatus::Uninitialized => summary.uninitialized += 1,
                SubmoduleStatus::OutOfDate => summary.out_of_date += 1,
                SubmoduleStatus::Conflicted => summary.modified += 1,
                SubmoduleStatus::Missing => summary.missing += 1,
            }
        }
        summary
    }

    /// Whether all submodules are clean.
    pub fn all_clean(&self) -> bool {
        self.clean == self.total
    }
}

impl fmt::Display for SubmoduleSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} submodules ({} clean, {} modified, {} uninitialized)",
            self.total, self.clean, self.modified, self.uninitialized
        )
    }
}

/// Trait for managing submodules in a repository.
pub trait SubmoduleManager {
    /// The error type for submodule operations.
    type Error: std::error::Error;

    /// List all registered submodules.
    fn list(&self) -> Result<Vec<Submodule>, Self::Error>;

    /// Initialize a submodule (populate its working directory).
    fn init(&self, path: &Path) -> Result<(), Self::Error>;

    /// Update a submodule to the expected commit.
    fn update(&self, path: &Path, strategy: UpdateStrategy) -> Result<(), Self::Error>;

    /// Synchronize the remote URL from `.gitmodules` to `.git/config`.
    fn sync(&self, path: &Path) -> Result<(), Self::Error>;

    /// Deinitialize a submodule (remove its working directory).
    fn deinit(&self, path: &Path, force: bool) -> Result<(), Self::Error>;

    /// Add a new submodule.
    fn add(&self, url: &str, path: &Path, branch: Option<&str>) -> Result<Submodule, Self::Error>;
}

/// Configuration entry from `.gitmodules`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitmodulesEntry {
    /// The submodule name (section key).
    pub name: String,
    /// The relative path.
    pub path: String,
    /// The remote URL.
    pub url: String,
    /// Optional branch to track.
    pub branch: Option<String>,
    /// Whether to shallow clone.
    pub shallow: Option<bool>,
}

impl GitmodulesEntry {
    /// Convert to a Submodule struct.
    pub fn to_submodule(&self) -> Submodule {
        let mut sub = Submodule::new(&self.name, &self.path, &self.url);
        sub.branch = self.branch.clone();
        sub.shallow = self.shallow.unwrap_or(false);
        sub
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn submodule_status_display() {
        let sub = Submodule::new("lib", "vendor/lib", "https://github.com/example/lib.git")
            .with_expected_commit("abc123def456")
            .with_actual_commit("abc123def456");
        assert!(sub.is_clean());
        assert!(!sub.needs_update());
    }

    #[test]
    fn out_of_date() {
        let sub = Submodule::new("lib", "vendor/lib", "https://github.com/example/lib.git")
            .with_expected_commit("abc123")
            .with_actual_commit("def456");
        assert_eq!(sub.status, SubmoduleStatus::OutOfDate);
        assert!(sub.needs_update());
    }

    #[test]
    fn summary() {
        let subs = vec![
            Submodule::new("a", "a", "u").with_expected_commit("1").with_actual_commit("1"),
            Submodule::new("b", "b", "u"),
        ];
        let summary = SubmoduleSummary::from_submodules(&subs);
        assert_eq!(summary.total, 2);
        assert_eq!(summary.clean, 1);
        assert_eq!(summary.uninitialized, 1);
    }
}
