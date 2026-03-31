//! Git blame types for per-line commit attribution.
//!
//! Provides types for representing blame output, which maps each line
//! of a file to the commit that last modified it.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Attribution data for a single line in a blame result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameLine {
    /// The line number (1-based).
    pub line_number: usize,
    /// The commit hash that introduced this line.
    pub commit_hash: String,
    /// The abbreviated commit hash.
    pub short_hash: String,
    /// The author who wrote this line.
    pub author: String,
    /// The author's email.
    pub author_email: Option<String>,
    /// When this line was written.
    pub date: DateTime<Utc>,
    /// The original file path (if the file was renamed).
    pub original_path: Option<String>,
    /// The original line number in the source commit.
    pub original_line: usize,
    /// The actual content of the line.
    pub content: String,
}

impl BlameLine {
    /// Create a new blame line.
    pub fn new(
        line_number: usize,
        commit_hash: impl Into<String>,
        author: impl Into<String>,
        date: DateTime<Utc>,
        content: impl Into<String>,
    ) -> Self {
        let hash = commit_hash.into();
        let short = hash[..hash.len().min(8)].to_string();
        Self {
            line_number,
            commit_hash: hash,
            short_hash: short,
            author: author.into(),
            author_email: None,
            date,
            original_path: None,
            original_line: line_number,
            content: content.into(),
        }
    }

    /// Set the author email.
    pub fn with_email(mut self, email: impl Into<String>) -> Self {
        self.author_email = Some(email.into());
        self
    }

    /// Set the original file path for renamed files.
    pub fn with_original_path(mut self, path: impl Into<String>) -> Self {
        self.original_path = Some(path.into());
        self
    }

    /// Set the original line number.
    pub fn with_original_line(mut self, line: usize) -> Self {
        self.original_line = line;
        self
    }

    /// Check if this line was introduced by the same commit as another.
    pub fn same_commit(&self, other: &BlameLine) -> bool {
        self.commit_hash == other.commit_hash
    }
}

impl fmt::Display for BlameLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({} {} {:>4}) {}",
            self.short_hash,
            self.author,
            self.date.format("%Y-%m-%d"),
            self.line_number,
            self.content,
        )
    }
}

/// The complete blame result for a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameResult {
    /// The file path that was blamed.
    pub file_path: String,
    /// The ref or commit used for the blame.
    pub revision: String,
    /// Per-line blame data.
    pub lines: Vec<BlameLine>,
}

impl BlameResult {
    /// Create a new blame result.
    pub fn new(
        file_path: impl Into<String>,
        revision: impl Into<String>,
        lines: Vec<BlameLine>,
    ) -> Self {
        Self {
            file_path: file_path.into(),
            revision: revision.into(),
            lines,
        }
    }

    /// Return the total number of lines.
    pub fn line_count(&self) -> usize {
        self.lines.len()
    }

    /// Return blame for a specific line (1-based).
    pub fn line(&self, n: usize) -> Option<&BlameLine> {
        if n == 0 || n > self.lines.len() {
            return None;
        }
        Some(&self.lines[n - 1])
    }

    /// Return a range of blame lines (1-based, inclusive).
    pub fn line_range(&self, start: usize, end: usize) -> &[BlameLine] {
        let start = start.saturating_sub(1);
        let end = end.min(self.lines.len());
        &self.lines[start..end]
    }

    /// Return the unique commit hashes that contributed to this file.
    pub fn unique_commits(&self) -> Vec<String> {
        let mut commits: Vec<String> = self
            .lines
            .iter()
            .map(|l| l.commit_hash.clone())
            .collect();
        commits.sort();
        commits.dedup();
        commits
    }

    /// Return the unique authors that contributed to this file.
    pub fn unique_authors(&self) -> Vec<String> {
        let mut authors: Vec<String> = self
            .lines
            .iter()
            .map(|l| l.author.clone())
            .collect();
        authors.sort();
        authors.dedup();
        authors
    }

    /// Return a per-author line count breakdown.
    pub fn author_line_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for line in &self.lines {
            *counts.entry(line.author.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Return a per-commit line count breakdown.
    pub fn commit_line_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for line in &self.lines {
            *counts.entry(line.commit_hash.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Find contiguous groups of lines attributed to the same commit.
    pub fn commit_groups(&self) -> Vec<BlameGroup> {
        let mut groups = Vec::new();
        if self.lines.is_empty() {
            return groups;
        }

        let mut current_hash = self.lines[0].commit_hash.clone();
        let mut start_line = 1;
        let mut count = 1;

        for (i, line) in self.lines.iter().enumerate().skip(1) {
            if line.commit_hash == current_hash {
                count += 1;
            } else {
                groups.push(BlameGroup {
                    commit_hash: current_hash.clone(),
                    author: self.lines[i - 1].author.clone(),
                    date: self.lines[i - 1].date,
                    start_line,
                    line_count: count,
                });
                current_hash = line.commit_hash.clone();
                start_line = line.line_number;
                count = 1;
            }
        }

        // Push the last group.
        if let Some(last) = self.lines.last() {
            groups.push(BlameGroup {
                commit_hash: current_hash,
                author: last.author.clone(),
                date: last.date,
                start_line,
                line_count: count,
            });
        }

        groups
    }

    /// Find the most recent change in the file.
    pub fn most_recent_change(&self) -> Option<&BlameLine> {
        self.lines.iter().max_by_key(|l| l.date)
    }

    /// Find the oldest line in the file.
    pub fn oldest_line(&self) -> Option<&BlameLine> {
        self.lines.iter().min_by_key(|l| l.date)
    }
}

/// A contiguous group of lines attributed to the same commit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameGroup {
    /// The commit hash for this group.
    pub commit_hash: String,
    /// The author.
    pub author: String,
    /// The commit date.
    pub date: DateTime<Utc>,
    /// The starting line number (1-based).
    pub start_line: usize,
    /// Number of contiguous lines.
    pub line_count: usize,
}

impl BlameGroup {
    /// Return the ending line number (inclusive).
    pub fn end_line(&self) -> usize {
        self.start_line + self.line_count - 1
    }

    /// Return the short commit hash.
    pub fn short_hash(&self) -> &str {
        &self.commit_hash[..self.commit_hash.len().min(8)]
    }
}

impl fmt::Display for BlameGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {}) lines {}-{}",
            self.short_hash(),
            self.author,
            self.date.format("%Y-%m-%d"),
            self.start_line,
            self.end_line()
        )
    }
}

/// Summary statistics for a blame result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlameSummary {
    /// The file path.
    pub file_path: String,
    /// Total lines in the file.
    pub total_lines: usize,
    /// Number of unique commits.
    pub unique_commits: usize,
    /// Number of unique authors.
    pub unique_authors: usize,
    /// Most prolific author (by line count).
    pub primary_author: Option<String>,
    /// Primary author's line count.
    pub primary_author_lines: usize,
    /// Most recent modification date.
    pub last_modified: Option<DateTime<Utc>>,
}

impl BlameSummary {
    /// Build a summary from a blame result.
    pub fn from_result(result: &BlameResult) -> Self {
        let author_counts = result.author_line_counts();
        let (primary_author, primary_lines) = author_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(author, count)| (Some(author.clone()), *count))
            .unwrap_or((None, 0));

        Self {
            file_path: result.file_path.clone(),
            total_lines: result.line_count(),
            unique_commits: result.unique_commits().len(),
            unique_authors: result.unique_authors().len(),
            primary_author,
            primary_author_lines: primary_lines,
            last_modified: result.most_recent_change().map(|l| l.date),
        }
    }
}

impl fmt::Display for BlameSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} lines, {} commits, {} authors",
            self.file_path, self.total_lines, self.unique_commits, self.unique_authors
        )?;
        if let Some(ref author) = self.primary_author {
            write!(f, " (primary: {} with {} lines)", author, self.primary_author_lines)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_blame() -> BlameResult {
        BlameResult::new(
            "src/main.rs",
            "HEAD",
            vec![
                BlameLine::new(1, "aaa11111", "Alice", Utc::now(), "fn main() {"),
                BlameLine::new(2, "aaa11111", "Alice", Utc::now(), "    println!(\"hello\");"),
                BlameLine::new(3, "bbb22222", "Bob", Utc::now(), "    do_stuff();"),
                BlameLine::new(4, "aaa11111", "Alice", Utc::now(), "}"),
            ],
        )
    }

    #[test]
    fn unique_authors() {
        let blame = sample_blame();
        let authors = blame.unique_authors();
        assert_eq!(authors.len(), 2);
    }

    #[test]
    fn author_line_counts() {
        let blame = sample_blame();
        let counts = blame.author_line_counts();
        assert_eq!(counts.get("Alice"), Some(&3));
        assert_eq!(counts.get("Bob"), Some(&1));
    }

    #[test]
    fn commit_groups() {
        let blame = sample_blame();
        let groups = blame.commit_groups();
        assert_eq!(groups.len(), 3); // aaa(2), bbb(1), aaa(1)
        assert_eq!(groups[0].line_count, 2);
        assert_eq!(groups[1].line_count, 1);
    }

    #[test]
    fn summary() {
        let blame = sample_blame();
        let summary = BlameSummary::from_result(&blame);
        assert_eq!(summary.total_lines, 4);
        assert_eq!(summary.primary_author.as_deref(), Some("Alice"));
    }
}
