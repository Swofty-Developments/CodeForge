//! Gitignore pattern matching and file filtering.
//!
//! Provides pattern parsing for `.gitignore` files and checking whether
//! a given file path should be ignored based on the accumulated patterns.

use serde::{Deserialize, Serialize};
use std::fmt;

/// A single pattern from a `.gitignore` file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IgnorePattern {
    /// The raw pattern string from the gitignore file.
    pub raw: String,
    /// Whether this is a negation pattern (starts with `!`).
    pub negated: bool,
    /// Whether this pattern only matches directories (ends with `/`).
    pub directory_only: bool,
    /// Whether this pattern is anchored to the repo root (contains `/`).
    pub anchored: bool,
    /// The normalized pattern for matching.
    pattern: String,
    /// The source file and line number.
    pub source: Option<PatternSource>,
}

/// Where a pattern was defined.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternSource {
    /// Path to the `.gitignore` file.
    pub file: String,
    /// Line number (1-based).
    pub line: usize,
}

impl IgnorePattern {
    /// Parse a single gitignore pattern line.
    pub fn parse(line: &str) -> Option<Self> {
        let line = line.trim_end();
        // Blank lines are skipped.
        if line.is_empty() {
            return None;
        }
        // Comments start with #.
        if line.starts_with('#') {
            return None;
        }

        let mut pattern = line.to_string();
        let negated = pattern.starts_with('!');
        if negated {
            pattern = pattern[1..].to_string();
        }

        // Trailing spaces can be escaped with backslash.
        let pattern = if pattern.ends_with("\\ ") {
            pattern.replace("\\ ", " ")
        } else {
            pattern.trim_end().to_string()
        };

        let directory_only = pattern.ends_with('/');
        let pattern_trimmed = if directory_only {
            pattern[..pattern.len() - 1].to_string()
        } else {
            pattern.clone()
        };

        // A pattern is anchored if it contains a slash (other than trailing).
        let anchored = pattern_trimmed.contains('/');

        Some(Self {
            raw: line.to_string(),
            negated,
            directory_only,
            anchored,
            pattern: pattern_trimmed,
            source: None,
        })
    }

    /// Attach source information.
    pub fn with_source(mut self, file: impl Into<String>, line: usize) -> Self {
        self.source = Some(PatternSource {
            file: file.into(),
            line,
        });
        self
    }

    /// Check if this pattern matches the given path.
    ///
    /// The path should be relative to the repository root, using forward slashes.
    pub fn matches(&self, path: &str, is_dir: bool) -> bool {
        if self.directory_only && !is_dir {
            return false;
        }
        if self.anchored {
            fnmatch(&self.pattern, path)
        } else {
            // Unanchored patterns match against the filename or any path suffix.
            let filename = path.rsplit('/').next().unwrap_or(path);
            if fnmatch(&self.pattern, filename) {
                return true;
            }
            // Also try matching against each suffix of the path.
            let mut remaining = path;
            while let Some(pos) = remaining.find('/') {
                remaining = &remaining[pos + 1..];
                if fnmatch(&self.pattern, remaining) {
                    return true;
                }
            }
            false
        }
    }
}

impl fmt::Display for IgnorePattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.raw)
    }
}

/// A collection of ignore patterns from one or more gitignore files.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IgnoreRules {
    /// Patterns in order of declaration (later patterns override earlier ones).
    patterns: Vec<IgnorePattern>,
}

impl IgnoreRules {
    /// Create an empty rule set.
    pub fn new() -> Self {
        Self {
            patterns: Vec::new(),
        }
    }

    /// Parse a gitignore file and add its patterns.
    pub fn add_file(&mut self, content: &str, source_path: &str) {
        for (line_num, line) in content.lines().enumerate() {
            if let Some(pattern) = IgnorePattern::parse(line) {
                self.patterns
                    .push(pattern.with_source(source_path, line_num + 1));
            }
        }
    }

    /// Add a single pattern.
    pub fn add_pattern(&mut self, pattern: IgnorePattern) {
        self.patterns.push(pattern);
    }

    /// Check whether the given path should be ignored.
    ///
    /// Returns `true` if the path is ignored, `false` if not.
    /// Negation patterns (`!`) can un-ignore previously ignored paths.
    pub fn is_ignored(&self, path: &str, is_dir: bool) -> bool {
        let path = path.trim_start_matches('/');
        let mut ignored = false;
        for pattern in &self.patterns {
            if pattern.matches(path, is_dir) {
                ignored = !pattern.negated;
            }
        }
        ignored
    }

    /// Return all patterns.
    pub fn patterns(&self) -> &[IgnorePattern] {
        &self.patterns
    }

    /// Return the number of patterns.
    pub fn len(&self) -> usize {
        self.patterns.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.patterns.is_empty()
    }

    /// Create rules from common default ignores.
    pub fn with_defaults() -> Self {
        let mut rules = Self::new();
        let defaults = ".git\n.DS_Store\nThumbs.db\n*.swp\n*.swo\n*~\n";
        rules.add_file(defaults, "(default)");
        rules
    }
}

/// Simple fnmatch-style glob matching.
///
/// Supports `*` (matches any sequence except `/`), `?` (matches one char except `/`),
/// and `**` (matches any sequence including `/`).
fn fnmatch(pattern: &str, text: &str) -> bool {
    fnmatch_inner(pattern.as_bytes(), text.as_bytes())
}

fn fnmatch_inner(pattern: &[u8], text: &[u8]) -> bool {
    let mut pi = 0;
    let mut ti = 0;
    let mut star_pi = None;
    let mut star_ti = None;

    while ti < text.len() {
        if pi < pattern.len() && pattern[pi] == b'*' {
            if pi + 1 < pattern.len() && pattern[pi + 1] == b'*' {
                // "**" matches everything including slashes.
                // Skip all consecutive stars.
                while pi < pattern.len() && pattern[pi] == b'*' {
                    pi += 1;
                }
                // Skip optional slash after **.
                if pi < pattern.len() && pattern[pi] == b'/' {
                    pi += 1;
                }
                if pi >= pattern.len() {
                    return true;
                }
                // Try matching from every position.
                for start in ti..=text.len() {
                    if fnmatch_inner(&pattern[pi..], &text[start..]) {
                        return true;
                    }
                }
                return false;
            }
            star_pi = Some(pi);
            star_ti = Some(ti);
            pi += 1;
        } else if pi < pattern.len() && (pattern[pi] == b'?' && text[ti] != b'/')
            || (pattern[pi] != b'*' && pattern[pi] != b'?' && pattern[pi] == text[ti])
        {
            pi += 1;
            ti += 1;
        } else if let (Some(sp), Some(st)) = (star_pi, star_ti) {
            pi = sp + 1;
            let new_st = st + 1;
            if text[st] == b'/' {
                // Single * does not match /.
                return false;
            }
            star_ti = Some(new_st);
            ti = new_st;
        } else {
            return false;
        }
    }

    while pi < pattern.len() && pattern[pi] == b'*' {
        pi += 1;
    }

    pi >= pattern.len()
}

/// Well-known ignore file names.
pub const GITIGNORE_FILES: &[&str] = &[".gitignore", ".git/info/exclude"];

/// Common patterns that should almost always be ignored.
pub const COMMON_IGNORES: &[&str] = &[
    "node_modules/",
    "target/",
    ".DS_Store",
    "Thumbs.db",
    "*.pyc",
    "__pycache__/",
    ".env",
    ".env.local",
    "dist/",
    "build/",
    "*.log",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_patterns() {
        let pat = IgnorePattern::parse("*.log").unwrap();
        assert!(pat.matches("app.log", false));
        assert!(pat.matches("logs/app.log", false));
        assert!(!pat.matches("app.txt", false));
    }

    #[test]
    fn directory_only() {
        let pat = IgnorePattern::parse("build/").unwrap();
        assert!(pat.matches("build", true));
        assert!(!pat.matches("build", false));
    }

    #[test]
    fn negation() {
        let mut rules = IgnoreRules::new();
        rules.add_file("*.log\n!important.log", ".gitignore");
        assert!(rules.is_ignored("debug.log", false));
        assert!(!rules.is_ignored("important.log", false));
    }

    #[test]
    fn double_star() {
        assert!(fnmatch("**/foo", "bar/baz/foo"));
        assert!(fnmatch("**/foo", "foo"));
    }

    #[test]
    fn anchored_pattern() {
        let pat = IgnorePattern::parse("/build").unwrap();
        assert!(pat.anchored);
    }
}
