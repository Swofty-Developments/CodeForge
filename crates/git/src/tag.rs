//! Git tag types and management.
//!
//! Provides types for lightweight and annotated tags, version tag parsing,
//! tag signing status, and a trait for tag operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// The type of a git tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TagKind {
    /// A lightweight tag (just a pointer to a commit).
    Lightweight,
    /// An annotated tag (has its own object with message, author, date).
    Annotated,
}

impl fmt::Display for TagKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagKind::Lightweight => write!(f, "lightweight"),
            TagKind::Annotated => write!(f, "annotated"),
        }
    }
}

/// The signing status of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SigningStatus {
    /// Tag is not signed.
    Unsigned,
    /// Tag is signed and signature is valid.
    ValidSignature,
    /// Tag is signed but signature verification failed.
    InvalidSignature,
    /// Tag is signed but the signing key is unknown.
    UnknownKey,
}

impl fmt::Display for SigningStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SigningStatus::Unsigned => write!(f, "unsigned"),
            SigningStatus::ValidSignature => write!(f, "valid signature"),
            SigningStatus::InvalidSignature => write!(f, "invalid signature"),
            SigningStatus::UnknownKey => write!(f, "unknown key"),
        }
    }
}

/// A git tag.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// The tag name (e.g., "v1.0.0").
    pub name: String,
    /// The type of tag.
    pub kind: TagKind,
    /// The commit hash this tag points to.
    pub target_commit: String,
    /// The tag object hash (for annotated tags).
    pub tag_object: Option<String>,
    /// The tag message (for annotated tags).
    pub message: Option<String>,
    /// The tagger name and email (for annotated tags).
    pub tagger: Option<String>,
    /// When the tag was created.
    pub date: Option<DateTime<Utc>>,
    /// Signing status.
    pub signing: SigningStatus,
}

impl Tag {
    /// Create a new lightweight tag.
    pub fn lightweight(name: impl Into<String>, commit: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            kind: TagKind::Lightweight,
            target_commit: commit.into(),
            tag_object: None,
            message: None,
            tagger: None,
            date: None,
            signing: SigningStatus::Unsigned,
        }
    }

    /// Create a new annotated tag.
    pub fn annotated(
        name: impl Into<String>,
        commit: impl Into<String>,
        message: impl Into<String>,
        tagger: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            kind: TagKind::Annotated,
            target_commit: commit.into(),
            tag_object: None,
            message: Some(message.into()),
            tagger: Some(tagger.into()),
            date: Some(Utc::now()),
            signing: SigningStatus::Unsigned,
        }
    }

    /// Return the short hash of the target commit.
    pub fn short_hash(&self) -> &str {
        &self.target_commit[..self.target_commit.len().min(8)]
    }

    /// Check if this tag looks like a version tag (starts with 'v' followed by a digit).
    pub fn is_version_tag(&self) -> bool {
        self.name.starts_with('v')
            && self
                .name
                .chars()
                .nth(1)
                .map_or(false, |c| c.is_ascii_digit())
    }

    /// Parse the version from a version tag (strips 'v' prefix).
    pub fn parse_version(&self) -> Option<VersionTag> {
        if !self.is_version_tag() {
            return None;
        }
        let version_str = &self.name[1..];
        let parts: Vec<&str> = version_str.split('.').collect();
        if parts.len() < 2 {
            return None;
        }
        let major = parts[0].parse::<u64>().ok()?;
        let minor = parts.get(1).and_then(|s| s.parse::<u64>().ok())?;
        // Patch may contain pre-release suffix.
        let patch_str = parts.get(2).unwrap_or(&"0");
        let (patch_num, pre_release) = if let Some(idx) = patch_str.find('-') {
            let p = patch_str[..idx].parse::<u64>().ok()?;
            Some((p, Some(patch_str[idx + 1..].to_string())))
        } else {
            Some((patch_str.parse::<u64>().ok()?, None))
        }?;

        Some(VersionTag {
            original: self.name.clone(),
            major,
            minor,
            patch: patch_num,
            pre_release,
        })
    }

    /// Whether the tag is signed.
    pub fn is_signed(&self) -> bool {
        !matches!(self.signing, SigningStatus::Unsigned)
    }

    /// Whether the tag has a valid signature.
    pub fn has_valid_signature(&self) -> bool {
        matches!(self.signing, SigningStatus::ValidSignature)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} -> {} ({})", self.name, self.short_hash(), self.kind)?;
        if let Some(ref msg) = self.message {
            let first_line = msg.lines().next().unwrap_or("");
            write!(f, " {first_line}")?;
        }
        Ok(())
    }
}

/// A parsed version from a version tag.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct VersionTag {
    /// The original tag name.
    pub original: String,
    /// Major version number.
    pub major: u64,
    /// Minor version number.
    pub minor: u64,
    /// Patch version number.
    pub patch: u64,
    /// Pre-release label (e.g., "beta.1").
    pub pre_release: Option<String>,
}

impl VersionTag {
    /// Format as a version string without the 'v' prefix.
    pub fn version_string(&self) -> String {
        let base = format!("{}.{}.{}", self.major, self.minor, self.patch);
        match &self.pre_release {
            Some(pre) => format!("{}-{}", base, pre),
            None => base,
        }
    }

    /// Check if this is a pre-release version.
    pub fn is_pre_release(&self) -> bool {
        self.pre_release.is_some()
    }

    /// Check if this is a major release (minor and patch are 0).
    pub fn is_major_release(&self) -> bool {
        self.minor == 0 && self.patch == 0 && self.pre_release.is_none()
    }
}

impl fmt::Display for VersionTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "v{}", self.version_string())
    }
}

/// Options for creating a tag.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreateTagOptions {
    /// The tag name.
    pub name: String,
    /// The target commit (defaults to HEAD).
    pub target: Option<String>,
    /// Message for annotated tags.
    pub message: Option<String>,
    /// Whether to sign the tag.
    pub sign: bool,
    /// Whether to force-replace an existing tag.
    pub force: bool,
}

impl CreateTagOptions {
    /// Create options for a lightweight tag.
    pub fn lightweight(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            ..Default::default()
        }
    }

    /// Create options for an annotated tag.
    pub fn annotated(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            message: Some(message.into()),
            ..Default::default()
        }
    }

    /// Set the target commit.
    pub fn at_commit(mut self, commit: impl Into<String>) -> Self {
        self.target = Some(commit.into());
        self
    }

    /// Enable GPG signing.
    pub fn signed(mut self) -> Self {
        self.sign = true;
        self
    }
}

/// Filter for listing tags.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TagFilter {
    /// Filter by name pattern (glob-style).
    pub pattern: Option<String>,
    /// Only include tags pointing at commits reachable from this ref.
    pub contains: Option<String>,
    /// Only include version tags.
    pub version_only: bool,
    /// Sort order.
    pub sort: TagSort,
}

/// Sort order for tags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum TagSort {
    /// Sort alphabetically by name.
    #[default]
    Name,
    /// Sort by version (requires version tags).
    Version,
    /// Sort by creation date.
    Date,
}

/// Trait for managing tags in a repository.
pub trait TagManager {
    /// The error type for tag operations.
    type Error: std::error::Error;

    /// List tags matching the filter.
    fn list(&self, filter: &TagFilter) -> Result<Vec<Tag>, Self::Error>;

    /// Create a new tag.
    fn create(&self, options: &CreateTagOptions) -> Result<Tag, Self::Error>;

    /// Delete a tag by name.
    fn delete(&self, name: &str) -> Result<(), Self::Error>;

    /// Verify the signature of a tag.
    fn verify(&self, name: &str) -> Result<SigningStatus, Self::Error>;

    /// Push a tag to a remote.
    fn push(&self, name: &str, remote: &str) -> Result<(), Self::Error>;

    /// Push all tags to a remote.
    fn push_all(&self, remote: &str) -> Result<(), Self::Error>;
}

/// Find the latest version tag from a list of tags.
pub fn latest_version_tag(tags: &[Tag]) -> Option<VersionTag> {
    let mut versions: Vec<VersionTag> = tags
        .iter()
        .filter_map(|t| t.parse_version())
        .collect();
    versions.sort();
    versions.into_iter().last()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_tag_parsing() {
        let tag = Tag::lightweight("v1.2.3", "abc123");
        let version = tag.parse_version().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
        assert_eq!(version.patch, 3);
        assert!(!version.is_pre_release());
    }

    #[test]
    fn pre_release_version() {
        let tag = Tag::lightweight("v2.0.0-beta.1", "abc123");
        let version = tag.parse_version().unwrap();
        assert!(version.is_pre_release());
        assert_eq!(version.pre_release.as_deref(), Some("beta.1"));
    }

    #[test]
    fn non_version_tag() {
        let tag = Tag::lightweight("release-candidate", "abc123");
        assert!(!tag.is_version_tag());
        assert!(tag.parse_version().is_none());
    }

    #[test]
    fn latest_version() {
        let tags = vec![
            Tag::lightweight("v1.0.0", "a"),
            Tag::lightweight("v2.1.0", "b"),
            Tag::lightweight("v1.5.0", "c"),
        ];
        let latest = latest_version_tag(&tags).unwrap();
        assert_eq!(latest.major, 2);
        assert_eq!(latest.minor, 1);
    }
}
