//! Remote repository types and operations.

use serde::{Deserialize, Serialize};

/// A configured git remote.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Remote {
    /// The remote name (e.g., `origin`, `upstream`).
    pub name: String,
    /// The fetch URL.
    pub fetch_url: String,
    /// The push URL (may differ from fetch URL).
    pub push_url: String,
}

impl Remote {
    /// Create a new remote with the same URL for fetch and push.
    pub fn new(name: impl Into<String>, url: impl Into<String>) -> Self {
        let url = url.into();
        Self {
            name: name.into(),
            fetch_url: url.clone(),
            push_url: url,
        }
    }

    /// Returns `true` if this remote points to a GitHub repository.
    pub fn is_github(&self) -> bool {
        self.fetch_url.contains("github.com")
    }

    /// Extract the `owner/repo` slug from a GitHub remote URL.
    ///
    /// Returns `None` if the URL is not a recognized GitHub format.
    pub fn github_slug(&self) -> Option<String> {
        let url = &self.fetch_url;
        // Handle SSH: git@github.com:owner/repo.git
        if let Some(rest) = url.strip_prefix("git@github.com:") {
            let slug = rest.strip_suffix(".git").unwrap_or(rest);
            return Some(slug.to_string());
        }
        // Handle HTTPS: https://github.com/owner/repo.git
        if let Some(rest) = url
            .strip_prefix("https://github.com/")
            .or_else(|| url.strip_prefix("http://github.com/"))
        {
            let slug = rest.strip_suffix(".git").unwrap_or(rest);
            return Some(slug.to_string());
        }
        None
    }
}

impl std::fmt::Display for Remote {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.name, self.fetch_url)
    }
}

/// The result of a fetch operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResult {
    /// The remote that was fetched from.
    pub remote: String,
    /// Branches that were updated.
    pub updated_refs: Vec<UpdatedRef>,
}

/// A single ref that was updated during a fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdatedRef {
    /// The full ref name (e.g., `refs/remotes/origin/main`).
    pub ref_name: String,
    /// The old commit hash (before update).
    pub old_hash: Option<String>,
    /// The new commit hash (after update).
    pub new_hash: String,
    /// Whether this is a new ref (not previously tracked).
    pub is_new: bool,
}

/// The result of a push operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushResult {
    /// The remote that was pushed to.
    pub remote: String,
    /// Refs that were pushed.
    pub pushed_refs: Vec<PushedRef>,
}

/// A single ref that was pushed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushedRef {
    /// The local ref that was pushed.
    pub local_ref: String,
    /// The remote ref that was updated.
    pub remote_ref: String,
    /// Whether the push was a fast-forward.
    pub fast_forward: bool,
}

/// The result of a pull operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PullResult {
    /// The remote that was pulled from.
    pub remote: String,
    /// The branch that was pulled.
    pub branch: String,
    /// The merge strategy used.
    pub strategy: PullStrategy,
    /// Number of new commits pulled.
    pub commits_pulled: usize,
    /// Whether there were merge conflicts.
    pub had_conflicts: bool,
}

/// The merge strategy used during a pull.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PullStrategy {
    /// Standard merge.
    Merge,
    /// Rebase on top of remote.
    Rebase,
    /// Fast-forward only (fails if not possible).
    FastForwardOnly,
}
