//! Git log filtering, formatting, and graph rendering.
//!
//! Provides types for filtering commit logs, formatting output in
//! various styles, and rendering branch graph visualizations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Filter criteria for querying the commit log.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    /// Filter commits by author name or email.
    pub author: Option<String>,
    /// Filter commits by committer name or email.
    pub committer: Option<String>,
    /// Only include commits after this date.
    pub since: Option<DateTime<Utc>>,
    /// Only include commits before this date.
    pub until: Option<DateTime<Utc>>,
    /// Grep commit messages for this pattern.
    pub message_grep: Option<String>,
    /// Only include commits touching these paths.
    pub paths: Vec<String>,
    /// Maximum number of commits to return.
    pub max_count: Option<usize>,
    /// Skip this many commits from the start.
    pub skip: Option<usize>,
    /// Only include merge commits.
    pub merges_only: bool,
    /// Exclude merge commits.
    pub no_merges: bool,
    /// Starting ref (e.g., branch name, commit hash).
    pub from_ref: Option<String>,
    /// Ending ref for a range (from_ref..to_ref).
    pub to_ref: Option<String>,
    /// Only first-parent commits (simplify history).
    pub first_parent: bool,
}

impl LogFilter {
    /// Create a filter for commits by a specific author.
    pub fn by_author(author: impl Into<String>) -> Self {
        Self {
            author: Some(author.into()),
            ..Default::default()
        }
    }

    /// Create a filter for the last N commits.
    pub fn last_n(n: usize) -> Self {
        Self {
            max_count: Some(n),
            ..Default::default()
        }
    }

    /// Create a filter for a commit range.
    pub fn range(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self {
            from_ref: Some(from.into()),
            to_ref: Some(to.into()),
            ..Default::default()
        }
    }

    /// Add a path filter.
    pub fn with_path(mut self, path: impl Into<String>) -> Self {
        self.paths.push(path.into());
        self
    }

    /// Set the message grep pattern.
    pub fn with_grep(mut self, pattern: impl Into<String>) -> Self {
        self.message_grep = Some(pattern.into());
        self
    }

    /// Limit results.
    pub fn with_limit(mut self, n: usize) -> Self {
        self.max_count = Some(n);
        self
    }

    /// Exclude merges.
    pub fn without_merges(mut self) -> Self {
        self.no_merges = true;
        self
    }

    /// Check if a commit matches this filter (client-side filtering).
    pub fn matches_commit(&self, commit: &LogEntry) -> bool {
        if let Some(ref author) = self.author {
            let author_lower = author.to_lowercase();
            if !commit.author.to_lowercase().contains(&author_lower)
                && !commit
                    .author_email
                    .as_ref()
                    .map_or(false, |e| e.to_lowercase().contains(&author_lower))
            {
                return false;
            }
        }
        if let Some(ref grep) = self.message_grep {
            let grep_lower = grep.to_lowercase();
            if !commit.message.to_lowercase().contains(&grep_lower) {
                return false;
            }
        }
        if let Some(since) = self.since {
            if commit.date < since {
                return false;
            }
        }
        if let Some(until) = self.until {
            if commit.date > until {
                return false;
            }
        }
        if self.no_merges && commit.is_merge {
            return false;
        }
        if self.merges_only && !commit.is_merge {
            return false;
        }
        true
    }
}

/// A commit entry in the log output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// The full commit hash.
    pub hash: String,
    /// The abbreviated commit hash.
    pub short_hash: String,
    /// The commit message (first line / subject).
    pub message: String,
    /// The full commit message body.
    pub body: Option<String>,
    /// Author name.
    pub author: String,
    /// Author email.
    pub author_email: Option<String>,
    /// Commit date.
    pub date: DateTime<Utc>,
    /// Parent commit hashes.
    pub parents: Vec<String>,
    /// Whether this is a merge commit (2+ parents).
    pub is_merge: bool,
    /// Refs pointing at this commit (branches, tags).
    pub refs: Vec<String>,
    /// Files changed in this commit.
    pub changed_files: Option<Vec<String>>,
    /// Insertions count.
    pub insertions: Option<usize>,
    /// Deletions count.
    pub deletions: Option<usize>,
}

impl LogEntry {
    /// Create a minimal log entry.
    pub fn new(
        hash: impl Into<String>,
        message: impl Into<String>,
        author: impl Into<String>,
        date: DateTime<Utc>,
    ) -> Self {
        let hash = hash.into();
        let short_hash = hash[..hash.len().min(8)].to_string();
        Self {
            hash,
            short_hash,
            message: message.into(),
            body: None,
            author: author.into(),
            author_email: None,
            date,
            parents: Vec::new(),
            is_merge: false,
            refs: Vec::new(),
            changed_files: None,
            insertions: None,
            deletions: None,
        }
    }

    /// Return the subject (first line of message).
    pub fn subject(&self) -> &str {
        self.message.lines().next().unwrap_or(&self.message)
    }

    /// Return a stat summary like "+42 -15".
    pub fn stat_summary(&self) -> Option<String> {
        match (self.insertions, self.deletions) {
            (Some(ins), Some(del)) => Some(format!("+{ins} -{del}")),
            _ => None,
        }
    }

    /// Return the ref decorations as a formatted string.
    pub fn decoration(&self) -> Option<String> {
        if self.refs.is_empty() {
            None
        } else {
            Some(format!("({})", self.refs.join(", ")))
        }
    }
}

impl fmt::Display for LogEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {}", self.short_hash, self.subject())
    }
}

/// Output format for log rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    /// One commit per line: hash + subject.
    Oneline,
    /// Short format: hash, author, subject.
    Short,
    /// Medium format: hash, author, date, subject, body.
    Medium,
    /// Full format: all fields.
    Full,
    /// Raw format: hash only.
    Raw,
}

impl Default for LogFormat {
    fn default() -> Self {
        LogFormat::Medium
    }
}

/// Format a log entry according to the given format.
pub fn format_log_entry(entry: &LogEntry, format: LogFormat) -> String {
    match format {
        LogFormat::Oneline => {
            let decoration = entry
                .decoration()
                .map(|d| format!(" {d}"))
                .unwrap_or_default();
            format!("{}{} {}", entry.short_hash, decoration, entry.subject())
        }
        LogFormat::Short => {
            format!(
                "commit {}\nAuthor: {}\n\n    {}\n",
                entry.short_hash,
                entry.author,
                entry.subject()
            )
        }
        LogFormat::Medium => {
            let mut out = format!(
                "commit {}\nAuthor: {}\nDate:   {}\n\n    {}\n",
                entry.hash,
                entry.author,
                entry.date.format("%a %b %d %H:%M:%S %Y %z"),
                entry.subject()
            );
            if let Some(ref body) = entry.body {
                for line in body.lines() {
                    out.push_str(&format!("    {line}\n"));
                }
            }
            out
        }
        LogFormat::Full => {
            let mut out = format!(
                "commit {}\nAuthor: {} <{}>\nDate:   {}\nParents: {}\n",
                entry.hash,
                entry.author,
                entry.author_email.as_deref().unwrap_or(""),
                entry.date.format("%a %b %d %H:%M:%S %Y %z"),
                entry.parents.join(", "),
            );
            if !entry.refs.is_empty() {
                out.push_str(&format!("Refs: {}\n", entry.refs.join(", ")));
            }
            out.push_str(&format!("\n    {}\n", entry.subject()));
            if let Some(ref body) = entry.body {
                for line in body.lines() {
                    out.push_str(&format!("    {line}\n"));
                }
            }
            if let Some(summary) = entry.stat_summary() {
                out.push_str(&format!("\n {summary}\n"));
            }
            out
        }
        LogFormat::Raw => {
            entry.hash.clone()
        }
    }
}

/// A character used in graph rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphChar {
    /// A commit node.
    Commit,
    /// A vertical continuation line.
    Pipe,
    /// A merge point.
    Merge,
    /// A branch fork going right.
    ForkRight,
    /// A branch fork going left.
    ForkLeft,
    /// A horizontal line.
    Horizontal,
    /// Empty space.
    Space,
}

impl GraphChar {
    /// Return the Unicode character for rendering.
    pub fn to_char(self) -> char {
        match self {
            GraphChar::Commit => '*',
            GraphChar::Pipe => '|',
            GraphChar::Merge => '*',
            GraphChar::ForkRight => '\\',
            GraphChar::ForkLeft => '/',
            GraphChar::Horizontal => '-',
            GraphChar::Space => ' ',
        }
    }
}

/// A single row in a graph rendering.
#[derive(Debug, Clone)]
pub struct GraphRow {
    /// The graph characters for this row.
    pub columns: Vec<GraphChar>,
    /// The log entry for this row (if it is a commit row).
    pub entry: Option<LogEntry>,
}

impl GraphRow {
    /// Render the graph prefix as a string.
    pub fn graph_prefix(&self) -> String {
        self.columns.iter().map(|c| c.to_char()).collect()
    }
}

impl fmt::Display for GraphRow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = self.graph_prefix();
        if let Some(ref entry) = self.entry {
            write!(f, "{prefix} {}", entry.subject())
        } else {
            write!(f, "{prefix}")
        }
    }
}

/// Build a simple graph rendering for a linear commit history.
pub fn render_linear_graph(entries: &[LogEntry]) -> Vec<GraphRow> {
    let mut rows = Vec::new();
    for (i, entry) in entries.iter().enumerate() {
        let is_last = i == entries.len() - 1;
        rows.push(GraphRow {
            columns: vec![if entry.is_merge {
                GraphChar::Merge
            } else {
                GraphChar::Commit
            }],
            entry: Some(entry.clone()),
        });
        if !is_last {
            rows.push(GraphRow {
                columns: vec![GraphChar::Pipe],
                entry: None,
            });
        }
    }
    rows
}

/// Statistics about a range of log entries.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogStats {
    /// Total number of commits.
    pub total_commits: usize,
    /// Number of merge commits.
    pub merge_commits: usize,
    /// Unique authors.
    pub authors: Vec<String>,
    /// Total insertions.
    pub total_insertions: usize,
    /// Total deletions.
    pub total_deletions: usize,
    /// Earliest commit date.
    pub first_date: Option<DateTime<Utc>>,
    /// Latest commit date.
    pub last_date: Option<DateTime<Utc>>,
}

impl LogStats {
    /// Compute statistics from a list of log entries.
    pub fn from_entries(entries: &[LogEntry]) -> Self {
        let mut authors = Vec::new();
        let mut total_ins = 0;
        let mut total_del = 0;
        let mut merges = 0;

        for entry in entries {
            if !authors.contains(&entry.author) {
                authors.push(entry.author.clone());
            }
            if entry.is_merge {
                merges += 1;
            }
            total_ins += entry.insertions.unwrap_or(0);
            total_del += entry.deletions.unwrap_or(0);
        }

        authors.sort();

        Self {
            total_commits: entries.len(),
            merge_commits: merges,
            authors,
            total_insertions: total_ins,
            total_deletions: total_del,
            first_date: entries.last().map(|e| e.date),
            last_date: entries.first().map(|e| e.date),
        }
    }
}

impl fmt::Display for LogStats {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} commits by {} authors, +{} -{}",
            self.total_commits,
            self.authors.len(),
            self.total_insertions,
            self.total_deletions
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_by_author() {
        let filter = LogFilter::by_author("alice");
        let entry = LogEntry::new("abc123", "fix bug", "Alice Smith", Utc::now());
        assert!(filter.matches_commit(&entry));

        let other = LogEntry::new("def456", "add feature", "Bob Jones", Utc::now());
        assert!(!filter.matches_commit(&other));
    }

    #[test]
    fn oneline_format() {
        let entry = LogEntry::new("abc12345", "Fix the thing", "Alice", Utc::now());
        let output = format_log_entry(&entry, LogFormat::Oneline);
        assert!(output.contains("abc12345"));
        assert!(output.contains("Fix the thing"));
    }

    #[test]
    fn graph_rendering() {
        let entries = vec![
            LogEntry::new("abc", "first", "A", Utc::now()),
            LogEntry::new("def", "second", "B", Utc::now()),
        ];
        let rows = render_linear_graph(&entries);
        assert_eq!(rows.len(), 3); // 2 commits + 1 pipe
        assert_eq!(rows[0].graph_prefix(), "*");
        assert_eq!(rows[1].graph_prefix(), "|");
    }

    #[test]
    fn log_stats() {
        let entries = vec![
            LogEntry::new("a", "msg1", "Alice", Utc::now()),
            LogEntry::new("b", "msg2", "Bob", Utc::now()),
        ];
        let stats = LogStats::from_entries(&entries);
        assert_eq!(stats.total_commits, 2);
        assert_eq!(stats.authors.len(), 2);
    }
}
