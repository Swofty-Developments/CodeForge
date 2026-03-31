//! Diff types for representing file and line-level changes.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A diff for a single file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileDiff {
    /// Path of the file (relative to repo root).
    pub path: String,
    /// Previous path if the file was renamed.
    pub old_path: Option<String>,
    /// The kind of change (added, modified, deleted, renamed).
    pub status: FileStatus,
    /// Individual diff hunks within this file.
    pub hunks: Vec<DiffHunk>,
    /// Aggregate statistics for this file.
    pub stats: DiffStats,
}

/// The status of a file in the diff.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FileStatus {
    /// File was newly added.
    Added,
    /// File was modified.
    Modified,
    /// File was deleted.
    Deleted,
    /// File was renamed (possibly with modifications).
    Renamed,
    /// File was copied.
    Copied,
}

impl fmt::Display for FileStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Added => write!(f, "A"),
            Self::Modified => write!(f, "M"),
            Self::Deleted => write!(f, "D"),
            Self::Renamed => write!(f, "R"),
            Self::Copied => write!(f, "C"),
        }
    }
}

/// A contiguous region of changes within a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffHunk {
    /// Starting line number in the old file.
    pub old_start: u32,
    /// Number of lines from the old file in this hunk.
    pub old_count: u32,
    /// Starting line number in the new file.
    pub new_start: u32,
    /// Number of lines from the new file in this hunk.
    pub new_count: u32,
    /// Optional header text (e.g., function name).
    pub header: Option<String>,
    /// Individual lines in this hunk.
    pub lines: Vec<DiffLine>,
}

impl fmt::Display for DiffHunk {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "@@ -{},{} +{},{} @@",
            self.old_start, self.old_count, self.new_start, self.new_count
        )?;
        if let Some(ref header) = self.header {
            write!(f, " {header}")?;
        }
        Ok(())
    }
}

/// A single line within a diff hunk.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiffLine {
    /// The kind of diff line.
    pub kind: DiffLineKind,
    /// The text content of the line (without the diff prefix character).
    pub content: String,
    /// Line number in the old file, if applicable.
    pub old_line_no: Option<u32>,
    /// Line number in the new file, if applicable.
    pub new_line_no: Option<u32>,
}

/// The kind of a diff line.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DiffLineKind {
    /// Unchanged context line.
    Context,
    /// Line was added.
    Addition,
    /// Line was removed.
    Deletion,
}

impl fmt::Display for DiffLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.kind {
            DiffLineKind::Context => ' ',
            DiffLineKind::Addition => '+',
            DiffLineKind::Deletion => '-',
        };
        write!(f, "{prefix}{}", self.content)
    }
}

/// Aggregate statistics for a diff.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiffStats {
    /// Number of files changed.
    pub files_changed: usize,
    /// Total lines added across all files.
    pub insertions: usize,
    /// Total lines removed across all files.
    pub deletions: usize,
}

impl DiffStats {
    /// Merge another `DiffStats` into this one.
    pub fn merge(&mut self, other: &DiffStats) {
        self.files_changed += other.files_changed;
        self.insertions += other.insertions;
        self.deletions += other.deletions;
    }

    /// Returns the net change in lines (insertions - deletions).
    pub fn net_change(&self) -> isize {
        self.insertions as isize - self.deletions as isize
    }
}

impl fmt::Display for DiffStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} file(s) changed, {} insertion(s)(+), {} deletion(s)(-)",
            self.files_changed, self.insertions, self.deletions
        )
    }
}
