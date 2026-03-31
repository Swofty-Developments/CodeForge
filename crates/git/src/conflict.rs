//! Merge conflict detection and representation types.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// A file containing one or more merge conflicts.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictFile {
    /// Path to the conflicted file, relative to the repository root.
    pub path: PathBuf,
    /// The individual conflict regions within the file.
    pub regions: Vec<ConflictRegion>,
    /// The total number of conflict markers found.
    pub marker_count: usize,
}

impl ConflictFile {
    /// Create a new conflict file with no regions.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            regions: Vec::new(),
            marker_count: 0,
        }
    }

    /// Add a conflict region to this file.
    pub fn add_region(&mut self, region: ConflictRegion) {
        self.marker_count += 1;
        self.regions.push(region);
    }

    /// Returns `true` if all conflicts in this file have been resolved.
    pub fn is_resolved(&self) -> bool {
        self.regions.iter().all(|r| r.resolution.is_some())
    }
}

impl fmt::Display for ConflictFile {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} conflict(s)",
            self.path.display(),
            self.regions.len()
        )
    }
}

/// A single conflict region within a file, bounded by conflict markers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictRegion {
    /// The starting line number of the conflict (the `<<<<<<<` marker).
    pub start_line: u32,
    /// The ending line number of the conflict (the `>>>>>>>` marker).
    pub end_line: u32,
    /// Content from the current branch (ours).
    pub ours: String,
    /// Content from the incoming branch (theirs).
    pub theirs: String,
    /// Content from the common ancestor, if available (three-way merge).
    pub base: Option<String>,
    /// The label for our side (e.g., branch name).
    pub ours_label: String,
    /// The label for their side.
    pub theirs_label: String,
    /// How this conflict was resolved, if at all.
    pub resolution: Option<ConflictResolution>,
}

/// How a conflict region was resolved.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ConflictResolution {
    /// Accepted our side entirely.
    AcceptOurs,
    /// Accepted their side entirely.
    AcceptTheirs,
    /// Accepted the common base content.
    AcceptBase,
    /// Merged both sides with custom content.
    Custom(String),
}

impl fmt::Display for ConflictResolution {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptOurs => write!(f, "accept ours"),
            Self::AcceptTheirs => write!(f, "accept theirs"),
            Self::AcceptBase => write!(f, "accept base"),
            Self::Custom(_) => write!(f, "custom merge"),
        }
    }
}

/// A conflict marker found during parsing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConflictMarker {
    /// Start of our side: `<<<<<<< branch-name`
    Ours,
    /// Separator between sides: `=======`
    Separator,
    /// Start of base content (three-way): `||||||| base`
    Base,
    /// End of their side: `>>>>>>> branch-name`
    Theirs,
}

impl ConflictMarker {
    /// The string prefix that identifies this marker type.
    pub fn prefix(&self) -> &'static str {
        match self {
            Self::Ours => "<<<<<<<",
            Self::Separator => "=======",
            Self::Base => "|||||||",
            Self::Theirs => ">>>>>>>",
        }
    }

    /// Attempt to identify a conflict marker from a line of text.
    pub fn from_line(line: &str) -> Option<Self> {
        let trimmed = line.trim_start();
        if trimmed.starts_with("<<<<<<<") {
            Some(Self::Ours)
        } else if trimmed.starts_with("=======") {
            Some(Self::Separator)
        } else if trimmed.starts_with("|||||||") {
            Some(Self::Base)
        } else if trimmed.starts_with(">>>>>>>") {
            Some(Self::Theirs)
        } else {
            None
        }
    }
}
