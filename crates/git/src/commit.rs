//! Commit types and builder for staging and committing changes.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// A git commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Commit {
    /// The full commit hash (40 hex characters).
    pub hash: String,
    /// The abbreviated commit hash.
    pub short_hash: String,
    /// The commit message (first line / subject).
    pub subject: String,
    /// The full commit message body (may be empty).
    pub body: String,
    /// The commit author.
    pub author: Author,
    /// The commit timestamp.
    pub date: DateTime<Utc>,
    /// Parent commit hashes.
    pub parents: Vec<String>,
}

impl Commit {
    /// Returns the full commit message (subject + body).
    pub fn full_message(&self) -> String {
        if self.body.is_empty() {
            self.subject.clone()
        } else {
            format!("{}\n\n{}", self.subject, self.body)
        }
    }

    /// Returns `true` if this is a merge commit (has more than one parent).
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }
}

impl fmt::Display for Commit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.short_hash, self.subject)
    }
}

/// A commit author or committer identity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Author {
    /// The author's name.
    pub name: String,
    /// The author's email address.
    pub email: String,
}

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

/// Builder for constructing and executing a git commit.
///
/// Allows incrementally staging files and setting commit metadata
/// before performing the actual commit operation.
#[derive(Debug, Clone)]
pub struct CommitBuilder {
    /// Files to stage before committing.
    staged_files: Vec<PathBuf>,
    /// The commit message.
    message: Option<String>,
    /// Override author (uses git config default if `None`).
    author: Option<Author>,
    /// Whether to amend the previous commit.
    amend: bool,
    /// Whether to allow an empty commit.
    allow_empty: bool,
}

impl CommitBuilder {
    /// Create a new commit builder.
    pub fn new() -> Self {
        Self {
            staged_files: Vec::new(),
            message: None,
            author: None,
            amend: false,
            allow_empty: false,
        }
    }

    /// Stage a file for inclusion in the commit.
    pub fn stage(mut self, path: impl Into<PathBuf>) -> Self {
        self.staged_files.push(path.into());
        self
    }

    /// Stage multiple files for inclusion in the commit.
    pub fn stage_all(mut self, paths: impl IntoIterator<Item = PathBuf>) -> Self {
        self.staged_files.extend(paths);
        self
    }

    /// Set the commit message.
    pub fn message(mut self, msg: impl Into<String>) -> Self {
        self.message = Some(msg.into());
        self
    }

    /// Override the commit author.
    pub fn author(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }

    /// Set whether to amend the previous commit.
    pub fn amend(mut self, amend: bool) -> Self {
        self.amend = amend;
        self
    }

    /// Set whether to allow creating an empty commit.
    pub fn allow_empty(mut self, allow: bool) -> Self {
        self.allow_empty = allow;
        self
    }

    /// Returns the list of files that will be staged.
    pub fn staged_files(&self) -> &[PathBuf] {
        &self.staged_files
    }

    /// Returns the commit message, if set.
    pub fn get_message(&self) -> Option<&str> {
        self.message.as_deref()
    }

    /// Returns `true` if this builder is configured to amend.
    pub fn is_amend(&self) -> bool {
        self.amend
    }

    /// Validate that the builder has enough information to create a commit.
    pub fn validate(&self) -> Result<(), crate::error::GitError> {
        if self.message.is_none() && !self.amend {
            return Err(crate::error::GitError::InvalidOperation {
                message: "commit message is required for non-amend commits".to_string(),
            });
        }
        if self.staged_files.is_empty() && !self.amend && !self.allow_empty {
            return Err(crate::error::GitError::InvalidOperation {
                message: "no files staged and allow_empty is false".to_string(),
            });
        }
        Ok(())
    }
}

impl Default for CommitBuilder {
    fn default() -> Self {
        Self::new()
    }
}
