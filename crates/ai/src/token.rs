//! Token counting, budget management, and truncation strategies.
//!
//! Provides abstractions for estimating token counts across different
//! models, managing token budgets within context windows, and
//! truncating messages to fit within limits.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Trait for counting tokens in text.
pub trait TokenCounter {
    /// Count the number of tokens in the given text.
    fn count(&self, text: &str) -> usize;

    /// Count tokens in a batch of texts.
    fn count_batch(&self, texts: &[&str]) -> Vec<usize> {
        texts.iter().map(|t| self.count(t)).collect()
    }
}

/// A simple approximate token counter based on character/word ratios.
///
/// This provides a rough estimate without requiring a real tokenizer.
/// On average, 1 token is approximately 4 characters or 0.75 words for English.
#[derive(Debug, Clone, Copy)]
pub struct ApproxTokenCounter {
    /// Characters per token ratio.
    pub chars_per_token: f64,
}

impl Default for ApproxTokenCounter {
    fn default() -> Self {
        Self {
            chars_per_token: 4.0,
        }
    }
}

impl ApproxTokenCounter {
    /// Create a counter with a specific ratio.
    pub fn with_ratio(chars_per_token: f64) -> Self {
        Self { chars_per_token }
    }

    /// Create a counter tuned for code (typically more tokens per character).
    pub fn for_code() -> Self {
        Self {
            chars_per_token: 3.5,
        }
    }
}

impl TokenCounter for ApproxTokenCounter {
    fn count(&self, text: &str) -> usize {
        (text.len() as f64 / self.chars_per_token).ceil() as usize
    }
}

/// Known model token limits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTokenLimits {
    /// The model identifier.
    pub model_id: String,
    /// Maximum context window size (input + output).
    pub context_window: usize,
    /// Maximum output tokens.
    pub max_output_tokens: usize,
    /// Maximum input tokens (derived from context_window - max_output).
    pub max_input_tokens: usize,
}

impl ModelTokenLimits {
    /// Create limits for a model.
    pub fn new(model_id: impl Into<String>, context: usize, max_output: usize) -> Self {
        Self {
            model_id: model_id.into(),
            context_window: context,
            max_output_tokens: max_output,
            max_input_tokens: context.saturating_sub(max_output),
        }
    }

    /// Get limits for Claude Sonnet 3.5/4.
    pub fn claude_sonnet() -> Self {
        Self::new("claude-sonnet-4-20250514", 200_000, 16_384)
    }

    /// Get limits for Claude Opus 4.
    pub fn claude_opus() -> Self {
        Self::new("claude-opus-4-20250514", 200_000, 16_384)
    }

    /// Get limits for Claude Haiku 3.5.
    pub fn claude_haiku() -> Self {
        Self::new("claude-3-5-haiku-20241022", 200_000, 8_192)
    }

    /// Get limits for GPT-4o.
    pub fn gpt_4o() -> Self {
        Self::new("gpt-4o", 128_000, 16_384)
    }

    /// Get limits for GPT-4o mini.
    pub fn gpt_4o_mini() -> Self {
        Self::new("gpt-4o-mini", 128_000, 16_384)
    }

    /// Look up limits by model ID prefix.
    pub fn for_model(model_id: &str) -> Option<Self> {
        let id = model_id.to_lowercase();
        if id.contains("opus") {
            Some(Self::claude_opus())
        } else if id.contains("sonnet") {
            Some(Self::claude_sonnet())
        } else if id.contains("haiku") {
            Some(Self::claude_haiku())
        } else if id.contains("gpt-4o-mini") {
            Some(Self::gpt_4o_mini())
        } else if id.contains("gpt-4o") {
            Some(Self::gpt_4o())
        } else {
            None
        }
    }
}

impl fmt::Display for ModelTokenLimits {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {}k context, {}k max output",
            self.model_id,
            self.context_window / 1000,
            self.max_output_tokens / 1000
        )
    }
}

/// Token budget tracker for a conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenBudget {
    /// Total available tokens.
    pub total: usize,
    /// Tokens reserved for system prompt.
    pub system_reserved: usize,
    /// Tokens reserved for output.
    pub output_reserved: usize,
    /// Tokens currently used by messages.
    pub used: usize,
}

impl TokenBudget {
    /// Create a new budget from model limits.
    pub fn from_limits(limits: &ModelTokenLimits, system_tokens: usize) -> Self {
        Self {
            total: limits.max_input_tokens,
            system_reserved: system_tokens,
            output_reserved: limits.max_output_tokens,
            used: system_tokens,
        }
    }

    /// Create a budget with explicit values.
    pub fn new(total: usize, system_reserved: usize, output_reserved: usize) -> Self {
        Self {
            total,
            system_reserved,
            output_reserved,
            used: system_reserved,
        }
    }

    /// Return the number of tokens remaining for messages.
    pub fn remaining(&self) -> usize {
        self.total.saturating_sub(self.used)
    }

    /// Check if adding `tokens` would exceed the budget.
    pub fn would_exceed(&self, tokens: usize) -> bool {
        self.used + tokens > self.total
    }

    /// Add tokens to the used count. Returns false if it would exceed budget.
    pub fn consume(&mut self, tokens: usize) -> bool {
        if self.would_exceed(tokens) {
            return false;
        }
        self.used += tokens;
        true
    }

    /// Return the utilization as a percentage (0.0-100.0).
    pub fn utilization_percent(&self) -> f64 {
        if self.total == 0 {
            return 0.0;
        }
        (self.used as f64 / self.total as f64) * 100.0
    }

    /// Check if the budget is critically low (< 10% remaining).
    pub fn is_critical(&self) -> bool {
        self.remaining() < self.total / 10
    }
}

impl fmt::Display for TokenBudget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}/{} tokens ({:.1}% used, {} remaining)",
            self.used,
            self.total,
            self.utilization_percent(),
            self.remaining()
        )
    }
}

/// Strategy for truncating messages to fit within a token budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TruncationStrategy {
    /// Remove oldest messages first.
    RemoveOldest,
    /// Remove from the middle, keeping the first and last messages.
    RemoveMiddle,
    /// Summarize old messages into a single condensed message.
    Summarize,
    /// Truncate individual long messages.
    TruncateMessages,
    /// Remove tool results first, keeping user/assistant messages.
    RemoveToolResults,
}

impl Default for TruncationStrategy {
    fn default() -> Self {
        TruncationStrategy::RemoveOldest
    }
}

impl fmt::Display for TruncationStrategy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TruncationStrategy::RemoveOldest => write!(f, "remove-oldest"),
            TruncationStrategy::RemoveMiddle => write!(f, "remove-middle"),
            TruncationStrategy::Summarize => write!(f, "summarize"),
            TruncationStrategy::TruncateMessages => write!(f, "truncate-messages"),
            TruncationStrategy::RemoveToolResults => write!(f, "remove-tool-results"),
        }
    }
}

/// A message with its token count for budget tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenizedMessage {
    /// The message role.
    pub role: MessageRole,
    /// The message content.
    pub content: String,
    /// The token count.
    pub tokens: usize,
    /// Whether this message can be removed during truncation.
    pub removable: bool,
}

/// Message role for token tracking.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MessageRole {
    /// System message.
    System,
    /// User message.
    User,
    /// Assistant message.
    Assistant,
    /// Tool result message.
    ToolResult,
}

/// Apply truncation to a list of tokenized messages to fit within a budget.
pub fn truncate_messages(
    messages: &[TokenizedMessage],
    max_tokens: usize,
    strategy: TruncationStrategy,
) -> Vec<TokenizedMessage> {
    let total: usize = messages.iter().map(|m| m.tokens).sum();
    if total <= max_tokens {
        return messages.to_vec();
    }

    let excess = total - max_tokens;
    let mut result = messages.to_vec();

    match strategy {
        TruncationStrategy::RemoveOldest => {
            let mut freed = 0;
            result.retain(|m| {
                if freed >= excess || !m.removable {
                    true
                } else {
                    freed += m.tokens;
                    false
                }
            });
        }
        TruncationStrategy::RemoveToolResults => {
            let mut freed = 0;
            result.retain(|m| {
                if freed >= excess {
                    true
                } else if m.role == MessageRole::ToolResult && m.removable {
                    freed += m.tokens;
                    false
                } else {
                    true
                }
            });
            // If still over, fall back to removing oldest.
            if result.iter().map(|m| m.tokens).sum::<usize>() > max_tokens {
                result = truncate_messages(&result, max_tokens, TruncationStrategy::RemoveOldest);
            }
        }
        TruncationStrategy::RemoveMiddle => {
            if result.len() <= 2 {
                return result;
            }
            let mut freed = 0;
            let mid_start = 1;
            let mid_end = result.len() - 1;
            let mut to_remove = Vec::new();
            for i in mid_start..mid_end {
                if freed >= excess {
                    break;
                }
                if result[i].removable {
                    freed += result[i].tokens;
                    to_remove.push(i);
                }
            }
            for i in to_remove.into_iter().rev() {
                result.remove(i);
            }
        }
        _ => {
            // For Summarize and TruncateMessages, fall back to RemoveOldest.
            result = truncate_messages(messages, max_tokens, TruncationStrategy::RemoveOldest);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn approx_token_counter() {
        let counter = ApproxTokenCounter::default();
        let count = counter.count("Hello, world!"); // 13 chars
        assert_eq!(count, 4); // ceil(13/4)
    }

    #[test]
    fn token_budget() {
        let limits = ModelTokenLimits::claude_sonnet();
        let mut budget = TokenBudget::from_limits(&limits, 5000);
        assert!(budget.remaining() > 0);
        assert!(budget.consume(1000));
        assert_eq!(budget.used, 6000);
    }

    #[test]
    fn model_lookup() {
        let limits = ModelTokenLimits::for_model("claude-sonnet-4-20250514");
        assert!(limits.is_some());
        assert_eq!(limits.unwrap().context_window, 200_000);
    }

    #[test]
    fn truncation_removes_oldest() {
        let messages = vec![
            TokenizedMessage {
                role: MessageRole::User,
                content: "first".into(),
                tokens: 100,
                removable: true,
            },
            TokenizedMessage {
                role: MessageRole::Assistant,
                content: "second".into(),
                tokens: 100,
                removable: true,
            },
            TokenizedMessage {
                role: MessageRole::User,
                content: "third".into(),
                tokens: 100,
                removable: true,
            },
        ];
        let result = truncate_messages(&messages, 200, TruncationStrategy::RemoveOldest);
        assert_eq!(result.len(), 2);
        assert_eq!(result[0].content, "second");
    }
}
