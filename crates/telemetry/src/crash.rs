//! Crash reporting: stack traces, system info, and panic handling.
//!
//! Provides types for capturing crash reports with diagnostic information,
//! installing panic hooks, and writing crash files for later analysis.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// A crash report capturing diagnostic information about a failure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReport {
    /// Unique identifier for this crash report.
    pub id: String,
    /// When the crash occurred (ISO 8601).
    pub timestamp: String,
    /// The crash reason / panic message.
    pub reason: String,
    /// Stack trace frames.
    pub stack_trace: Vec<StackFrame>,
    /// System information.
    pub system_info: SystemInfo,
    /// Application version.
    pub app_version: String,
    /// Reproduction steps or context.
    pub context: HashMap<String, String>,
    /// The thread that crashed.
    pub thread_name: Option<String>,
    /// Whether this was a panic (vs. a signal).
    pub is_panic: bool,
    /// Severity level.
    pub severity: CrashSeverity,
}

impl CrashReport {
    /// Create a new crash report from a panic message.
    pub fn from_panic(message: impl Into<String>) -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
            reason: message.into(),
            stack_trace: Vec::new(),
            system_info: SystemInfo::current(),
            app_version: String::new(),
            context: HashMap::new(),
            thread_name: std::thread::current().name().map(String::from),
            is_panic: true,
            severity: CrashSeverity::Fatal,
        }
    }

    /// Set the application version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.app_version = version.into();
        self
    }

    /// Add a context key-value pair.
    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.insert(key.into(), value.into());
        self
    }

    /// Add stack trace frames.
    pub fn with_stack_trace(mut self, frames: Vec<StackFrame>) -> Self {
        self.stack_trace = frames;
        self
    }

    /// Generate a human-readable summary.
    pub fn summary(&self) -> String {
        let mut lines = Vec::new();
        lines.push(format!("Crash Report: {}", self.id));
        lines.push(format!("Time: {}", self.timestamp));
        lines.push(format!("Reason: {}", self.reason));
        lines.push(format!("Version: {}", self.app_version));
        if let Some(ref thread) = self.thread_name {
            lines.push(format!("Thread: {thread}"));
        }
        lines.push(format!("OS: {} {}", self.system_info.os, self.system_info.os_version));
        lines.push(format!("Arch: {}", self.system_info.arch));
        if !self.stack_trace.is_empty() {
            lines.push(String::new());
            lines.push("Stack trace:".to_string());
            for (i, frame) in self.stack_trace.iter().enumerate() {
                lines.push(format!("  #{i}: {frame}"));
            }
        }
        if !self.context.is_empty() {
            lines.push(String::new());
            lines.push("Context:".to_string());
            for (k, v) in &self.context {
                lines.push(format!("  {k}: {v}"));
            }
        }
        lines.join("\n")
    }

    /// Serialize to JSON for file writing.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Generate the crash file path.
    pub fn crash_file_path(&self, dir: &std::path::Path) -> PathBuf {
        dir.join(format!("crash-{}.json", &self.id[..8]))
    }
}

impl fmt::Display for CrashReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} - {} ({} frames)",
            self.severity,
            self.reason,
            self.timestamp,
            self.stack_trace.len()
        )
    }
}

/// Severity of a crash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrashSeverity {
    /// Non-fatal error that was caught.
    Warning,
    /// Error that caused a feature to fail.
    Error,
    /// Fatal crash that terminated the application.
    Fatal,
}

impl fmt::Display for CrashSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CrashSeverity::Warning => write!(f, "WARNING"),
            CrashSeverity::Error => write!(f, "ERROR"),
            CrashSeverity::Fatal => write!(f, "FATAL"),
        }
    }
}

/// A single frame in a stack trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StackFrame {
    /// The function name (demangled if available).
    pub function: String,
    /// The source file path.
    pub file: Option<String>,
    /// The line number.
    pub line: Option<u32>,
    /// The column number.
    pub column: Option<u32>,
    /// The module or crate name.
    pub module: Option<String>,
    /// The instruction pointer address.
    pub address: Option<String>,
}

impl StackFrame {
    /// Create a frame with just a function name.
    pub fn new(function: impl Into<String>) -> Self {
        Self {
            function: function.into(),
            file: None,
            line: None,
            column: None,
            module: None,
            address: None,
        }
    }

    /// Set the file and line.
    pub fn with_location(mut self, file: impl Into<String>, line: u32) -> Self {
        self.file = Some(file.into());
        self.line = Some(line);
        self
    }

    /// Set the module name.
    pub fn with_module(mut self, module: impl Into<String>) -> Self {
        self.module = Some(module.into());
        self
    }

    /// Check if this frame has source location information.
    pub fn has_location(&self) -> bool {
        self.file.is_some() && self.line.is_some()
    }
}

impl fmt::Display for StackFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.function)?;
        if let (Some(ref file), Some(line)) = (&self.file, self.line) {
            write!(f, " at {file}:{line}")?;
            if let Some(col) = self.column {
                write!(f, ":{col}")?;
            }
        }
        Ok(())
    }
}

/// System information captured at crash time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system name.
    pub os: String,
    /// Operating system version.
    pub os_version: String,
    /// CPU architecture.
    pub arch: String,
    /// Number of CPU cores.
    pub cpu_count: usize,
    /// Total system memory in bytes.
    pub total_memory_bytes: u64,
    /// Available memory at crash time in bytes.
    pub available_memory_bytes: Option<u64>,
    /// Process uptime in seconds.
    pub uptime_secs: Option<u64>,
    /// Rust version the binary was compiled with.
    pub rust_version: Option<String>,
}

impl SystemInfo {
    /// Capture current system information.
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            os_version: String::new(), // Would need platform-specific code.
            arch: std::env::consts::ARCH.to_string(),
            cpu_count: 0, // Would need sys-info crate.
            total_memory_bytes: 0,
            available_memory_bytes: None,
            uptime_secs: None,
            rust_version: option_env!("RUSTC_VERSION").map(String::from),
        }
    }
}

impl fmt::Display for SystemInfo {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} {} ({})", self.os, self.os_version, self.arch)
    }
}

/// Configuration for crash reporting.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashReportConfig {
    /// Whether crash reporting is enabled.
    pub enabled: bool,
    /// Directory to write crash files to.
    pub crash_dir: PathBuf,
    /// Maximum number of crash files to retain.
    pub max_files: u32,
    /// Whether to include the stack trace.
    pub include_stack_trace: bool,
    /// Whether to include system information.
    pub include_system_info: bool,
    /// Custom metadata to include in every report.
    pub metadata: HashMap<String, String>,
}

impl Default for CrashReportConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            crash_dir: PathBuf::from("crashes"),
            max_files: 50,
            include_stack_trace: true,
            include_system_info: true,
            metadata: HashMap::new(),
        }
    }
}

/// A log of past crash reports for analysis.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CrashHistory {
    /// All recorded crash summaries.
    pub entries: Vec<CrashSummary>,
}

/// A summary of a crash for the history log.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrashSummary {
    /// The crash report ID.
    pub id: String,
    /// When the crash occurred.
    pub timestamp: String,
    /// The crash reason.
    pub reason: String,
    /// Severity.
    pub severity: CrashSeverity,
    /// App version at crash time.
    pub app_version: String,
}

impl CrashHistory {
    /// Add a crash summary.
    pub fn add(&mut self, report: &CrashReport) {
        self.entries.push(CrashSummary {
            id: report.id.clone(),
            timestamp: report.timestamp.clone(),
            reason: report.reason.clone(),
            severity: report.severity,
            app_version: report.app_version.clone(),
        });
    }

    /// Return crashes in the last N days.
    pub fn recent(&self, days: u32) -> Vec<&CrashSummary> {
        // Simple filter based on string comparison (ISO 8601 is sortable).
        let cutoff = chrono::Utc::now() - chrono::Duration::days(days as i64);
        let cutoff_str = cutoff.to_rfc3339();
        self.entries
            .iter()
            .filter(|e| e.timestamp >= cutoff_str)
            .collect()
    }

    /// Count crashes by version.
    pub fn by_version(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for entry in &self.entries {
            *counts.entry(entry.app_version.clone()).or_insert(0) += 1;
        }
        counts
    }

    /// Count crashes by reason (grouping similar reasons).
    pub fn by_reason(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for entry in &self.entries {
            // Take the first line of the reason as the group key.
            let key = entry.reason.lines().next().unwrap_or(&entry.reason).to_string();
            *counts.entry(key).or_insert(0) += 1;
        }
        counts
    }

    /// Total number of recorded crashes.
    pub fn total(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crash_report_summary() {
        let report = CrashReport::from_panic("index out of bounds")
            .with_version("1.0.0")
            .with_context("session_id", "abc123")
            .with_stack_trace(vec![
                StackFrame::new("main::do_thing").with_location("src/main.rs", 42),
                StackFrame::new("std::panic::catch"),
            ]);
        let summary = report.summary();
        assert!(summary.contains("index out of bounds"));
        assert!(summary.contains("src/main.rs:42"));
    }

    #[test]
    fn crash_history() {
        let mut history = CrashHistory::default();
        let report = CrashReport::from_panic("oops").with_version("1.0.0");
        history.add(&report);
        assert_eq!(history.total(), 1);
        let by_version = history.by_version();
        assert_eq!(by_version.get("1.0.0"), Some(&1));
    }

    #[test]
    fn stack_frame_display() {
        let frame = StackFrame::new("my_crate::module::function")
            .with_location("src/module.rs", 100);
        assert_eq!(
            frame.to_string(),
            "my_crate::module::function at src/module.rs:100"
        );
    }
}
