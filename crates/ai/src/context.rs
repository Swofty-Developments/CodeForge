//! Context window tracking and compaction strategies.

use serde::{Deserialize, Serialize};

/// Tracks token usage within the AI context window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContextWindow {
    /// Maximum tokens the model supports.
    pub capacity: u64,
    /// Tokens used by the system prompt.
    pub system_tokens: u64,
    /// Tokens used by conversation messages.
    pub message_tokens: u64,
    /// Tokens used by tool definitions.
    pub tool_tokens: u64,
    /// Tokens reserved for the model's response.
    pub response_reserve: u64,
}

impl ContextWindow {
    /// Create a new context window with the given capacity.
    pub fn new(capacity: u64) -> Self {
        Self {
            capacity,
            ..Default::default()
        }
    }

    /// Returns the total tokens currently used.
    pub fn used(&self) -> u64 {
        self.system_tokens + self.message_tokens + self.tool_tokens
    }

    /// Returns the number of tokens available for new messages.
    pub fn available(&self) -> u64 {
        self.capacity
            .saturating_sub(self.used())
            .saturating_sub(self.response_reserve)
    }

    /// Returns the usage as a fraction (0.0 to 1.0).
    pub fn usage_fraction(&self) -> f64 {
        if self.capacity == 0 {
            return 0.0;
        }
        self.used() as f64 / self.capacity as f64
    }

    /// Returns `true` if context compaction should be triggered.
    ///
    /// Defaults to triggering at 90% usage.
    pub fn needs_compaction(&self) -> bool {
        self.usage_fraction() > 0.9
    }

    /// Returns `true` if the context window is effectively full
    /// (less than 5% remaining after response reserve).
    pub fn is_full(&self) -> bool {
        self.available() < (self.capacity / 20)
    }
}

impl std::fmt::Display for ContextWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let pct = (self.usage_fraction() * 100.0) as u32;
        write!(
            f,
            "{}/{} tokens ({}% used, {} available)",
            self.used(),
            self.capacity,
            pct,
            self.available()
        )
    }
}

/// Trait for implementing context compaction strategies.
pub trait ContextManager {
    /// The error type for compaction operations.
    type Error: std::error::Error;

    /// Compact the context by summarizing or removing older messages.
    ///
    /// Returns the number of tokens freed.
    fn compact(&self, window: &ContextWindow) -> Result<CompactionResult, Self::Error>;

    /// Estimate the token count for a given text.
    fn estimate_tokens(&self, text: &str) -> u64;
}

/// The result of a context compaction operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompactionResult {
    /// Number of tokens freed by compaction.
    pub tokens_freed: u64,
    /// Number of messages removed or summarized.
    pub messages_affected: usize,
    /// The compaction strategy that was used.
    pub strategy: CompactionStrategy,
}

/// Available strategies for compacting the context window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompactionStrategy {
    /// Remove the oldest messages.
    TruncateOldest,
    /// Summarize older messages into a condensed form.
    Summarize,
    /// Remove tool result content but keep tool use records.
    StripToolResults,
    /// Remove thinking blocks from assistant messages.
    StripThinking,
}
