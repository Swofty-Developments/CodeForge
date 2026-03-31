//! Structured logging with levels, field extraction, and output formatting.
//!
//! Provides types for structured log entries with key-value fields,
//! log filtering by level and source, and output formatters for
//! JSON, pretty, and compact output styles.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Log severity levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    /// Extremely detailed diagnostic information.
    Trace,
    /// Detailed diagnostic information.
    Debug,
    /// Normal operational messages.
    Info,
    /// Potential issues that deserve attention.
    Warn,
    /// Errors that need investigation.
    Error,
    /// Critical failures requiring immediate action.
    Fatal,
}

impl LogLevel {
    /// Return the short label (4 chars, uppercase).
    pub fn label(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRCE",
            LogLevel::Debug => "DBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "EROR",
            LogLevel::Fatal => "FATL",
        }
    }

    /// Return the ANSI color code for this level.
    pub fn color_code(&self) -> &'static str {
        match self {
            LogLevel::Trace => "\x1b[90m",    // gray
            LogLevel::Debug => "\x1b[36m",    // cyan
            LogLevel::Info => "\x1b[32m",     // green
            LogLevel::Warn => "\x1b[33m",     // yellow
            LogLevel::Error => "\x1b[31m",    // red
            LogLevel::Fatal => "\x1b[1;31m",  // bold red
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.label())
    }
}

impl std::str::FromStr for LogLevel {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" | "warning" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            "fatal" => Ok(LogLevel::Fatal),
            _ => Err(format!("unknown log level: {s}")),
        }
    }
}

/// A structured log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    /// The severity level.
    pub level: LogLevel,
    /// The log message.
    pub message: String,
    /// When this entry was created.
    pub timestamp: DateTime<Utc>,
    /// The source module or component.
    pub source: Option<String>,
    /// Structured fields (key-value pairs).
    pub fields: HashMap<String, serde_json::Value>,
    /// Span/trace ID for correlation.
    pub trace_id: Option<String>,
    /// The file and line that produced this log.
    pub location: Option<LogLocation>,
}

/// Source code location of a log entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLocation {
    /// The file path.
    pub file: String,
    /// The line number.
    pub line: u32,
    /// The function name.
    pub function: Option<String>,
}

impl LogEntry {
    /// Create a new log entry.
    pub fn new(level: LogLevel, message: impl Into<String>) -> Self {
        Self {
            level,
            message: message.into(),
            timestamp: Utc::now(),
            source: None,
            fields: HashMap::new(),
            trace_id: None,
            location: None,
        }
    }

    /// Set the source component.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Add a string field.
    pub fn with_field(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.fields
            .insert(key.into(), serde_json::Value::String(value.into()));
        self
    }

    /// Add a numeric field.
    pub fn with_number(mut self, key: impl Into<String>, value: f64) -> Self {
        self.fields.insert(
            key.into(),
            serde_json::Value::Number(serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0))),
        );
        self
    }

    /// Add a boolean field.
    pub fn with_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.fields
            .insert(key.into(), serde_json::Value::Bool(value));
        self
    }

    /// Set the trace ID.
    pub fn with_trace_id(mut self, id: impl Into<String>) -> Self {
        self.trace_id = Some(id.into());
        self
    }

    /// Convenience constructors.
    pub fn info(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, message)
    }

    /// Create a warning entry.
    pub fn warn(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, message)
    }

    /// Create an error entry.
    pub fn error(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, message)
    }

    /// Create a debug entry.
    pub fn debug(message: impl Into<String>) -> Self {
        Self::new(LogLevel::Debug, message)
    }
}

/// Output format for log entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogOutputFormat {
    /// JSON format (one JSON object per line).
    Json,
    /// Pretty-printed human-readable format with colors.
    Pretty,
    /// Compact single-line format.
    Compact,
    /// Logfmt-style key=value format.
    Logfmt,
}

impl Default for LogOutputFormat {
    fn default() -> Self {
        LogOutputFormat::Pretty
    }
}

/// Format a log entry according to the output format.
pub fn format_log_entry(entry: &LogEntry, format: LogOutputFormat) -> String {
    match format {
        LogOutputFormat::Json => {
            serde_json::to_string(entry).unwrap_or_else(|_| entry.message.clone())
        }
        LogOutputFormat::Pretty => {
            let time = entry.timestamp.format("%H:%M:%S%.3f");
            let color = entry.level.color_code();
            let reset = "\x1b[0m";
            let source = entry
                .source
                .as_ref()
                .map(|s| format!(" [{s}]"))
                .unwrap_or_default();
            let fields = if entry.fields.is_empty() {
                String::new()
            } else {
                let pairs: Vec<String> = entry
                    .fields
                    .iter()
                    .map(|(k, v)| format!("{k}={v}"))
                    .collect();
                format!(" {}", pairs.join(" "))
            };
            format!(
                "{time} {color}{}{reset}{source} {}{fields}",
                entry.level.label(),
                entry.message
            )
        }
        LogOutputFormat::Compact => {
            let time = entry.timestamp.format("%H:%M:%S");
            format!("{} {} {}", time, entry.level.label(), entry.message)
        }
        LogOutputFormat::Logfmt => {
            let mut parts = vec![
                format!("ts={}", entry.timestamp.to_rfc3339()),
                format!("level={}", entry.level.label().to_lowercase()),
                format!("msg=\"{}\"", entry.message),
            ];
            if let Some(ref source) = entry.source {
                parts.push(format!("source={source}"));
            }
            for (k, v) in &entry.fields {
                parts.push(format!("{k}={v}"));
            }
            parts.join(" ")
        }
    }
}

/// Filter for selecting which log entries to output.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LogFilter {
    /// Minimum level to include.
    pub min_level: Option<LogLevel>,
    /// Only include entries from these sources.
    pub sources: Vec<String>,
    /// Exclude entries from these sources.
    pub exclude_sources: Vec<String>,
    /// Only include entries matching this message substring.
    pub message_contains: Option<String>,
    /// Only include entries with this field present.
    pub required_field: Option<String>,
}

impl LogFilter {
    /// Create a filter with a minimum level.
    pub fn min_level(level: LogLevel) -> Self {
        Self {
            min_level: Some(level),
            ..Default::default()
        }
    }

    /// Check if a log entry passes this filter.
    pub fn matches(&self, entry: &LogEntry) -> bool {
        if let Some(min) = self.min_level {
            if entry.level < min {
                return false;
            }
        }
        if !self.sources.is_empty() {
            let source = entry.source.as_deref().unwrap_or("");
            if !self.sources.iter().any(|s| source.contains(s.as_str())) {
                return false;
            }
        }
        if !self.exclude_sources.is_empty() {
            let source = entry.source.as_deref().unwrap_or("");
            if self
                .exclude_sources
                .iter()
                .any(|s| source.contains(s.as_str()))
            {
                return false;
            }
        }
        if let Some(ref pattern) = self.message_contains {
            if !entry.message.to_lowercase().contains(&pattern.to_lowercase()) {
                return false;
            }
        }
        if let Some(ref field) = self.required_field {
            if !entry.fields.contains_key(field) {
                return false;
            }
        }
        true
    }
}

/// Configuration for log rotation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RotationPolicy {
    /// Maximum file size in bytes before rotating.
    pub max_size_bytes: u64,
    /// Maximum number of rotated files to keep.
    pub max_files: u32,
    /// Whether to compress rotated files.
    pub compress: bool,
    /// Maximum age of log files in days.
    pub max_age_days: Option<u32>,
}

impl Default for RotationPolicy {
    fn default() -> Self {
        Self {
            max_size_bytes: 10 * 1024 * 1024, // 10 MB
            max_files: 5,
            compress: true,
            max_age_days: Some(30),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Warn < LogLevel::Error);
        assert!(LogLevel::Error < LogLevel::Fatal);
    }

    #[test]
    fn log_entry_builder() {
        let entry = LogEntry::info("request completed")
            .with_source("http")
            .with_field("method", "GET")
            .with_number("status", 200.0)
            .with_number("duration_ms", 42.5);
        assert_eq!(entry.level, LogLevel::Info);
        assert_eq!(entry.fields.len(), 3);
    }

    #[test]
    fn format_json() {
        let entry = LogEntry::info("test");
        let json = format_log_entry(&entry, LogOutputFormat::Json);
        assert!(json.contains("\"level\":\"Info\""));
    }

    #[test]
    fn filter_by_level() {
        let filter = LogFilter::min_level(LogLevel::Warn);
        assert!(!filter.matches(&LogEntry::info("ignored")));
        assert!(filter.matches(&LogEntry::warn("included")));
        assert!(filter.matches(&LogEntry::error("included")));
    }
}
