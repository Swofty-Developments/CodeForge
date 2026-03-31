//! Token usage tracking and cost calculation.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;

/// A report of token usage for a single API call.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UsageReport {
    /// Number of input tokens consumed.
    pub input_tokens: u64,
    /// Number of output tokens generated.
    pub output_tokens: u64,
    /// Number of tokens used for caching (cache reads).
    pub cache_read_tokens: u64,
    /// Number of tokens written to cache.
    pub cache_write_tokens: u64,
    /// The model that was used.
    pub model: Option<String>,
    /// Timestamp of this usage event.
    pub timestamp: Option<DateTime<Utc>>,
}

impl UsageReport {
    /// Returns the total token count (input + output).
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl fmt::Display for UsageReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "in: {} | out: {} | total: {}",
            self.input_tokens,
            self.output_tokens,
            self.total_tokens()
        )?;
        if self.cache_read_tokens > 0 {
            write!(f, " | cache read: {}", self.cache_read_tokens)?;
        }
        Ok(())
    }
}

/// Accumulates usage across multiple API calls within a session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CostTracker {
    /// All usage reports accumulated so far.
    reports: Vec<UsageReport>,
    /// Running total of input tokens.
    total_input: u64,
    /// Running total of output tokens.
    total_output: u64,
    /// Running total of cache read tokens.
    total_cache_read: u64,
    /// Running total of cache write tokens.
    total_cache_write: u64,
    /// Number of API calls made.
    api_calls: u64,
}

impl CostTracker {
    /// Create a new empty cost tracker.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new usage report.
    pub fn record(&mut self, report: UsageReport) {
        self.total_input += report.input_tokens;
        self.total_output += report.output_tokens;
        self.total_cache_read += report.cache_read_tokens;
        self.total_cache_write += report.cache_write_tokens;
        self.api_calls += 1;
        self.reports.push(report);
    }

    /// Returns the total input tokens across all API calls.
    pub fn total_input_tokens(&self) -> u64 {
        self.total_input
    }

    /// Returns the total output tokens across all API calls.
    pub fn total_output_tokens(&self) -> u64 {
        self.total_output
    }

    /// Returns the grand total of all tokens.
    pub fn total_tokens(&self) -> u64 {
        self.total_input + self.total_output
    }

    /// Returns the number of API calls tracked.
    pub fn api_call_count(&self) -> u64 {
        self.api_calls
    }

    /// Returns the average tokens per API call.
    pub fn avg_tokens_per_call(&self) -> f64 {
        if self.api_calls == 0 {
            return 0.0;
        }
        self.total_tokens() as f64 / self.api_calls as f64
    }

    /// Estimate the cost in USD based on model pricing.
    ///
    /// Uses approximate pricing: input $3/MTok, output $15/MTok for Sonnet-class models.
    pub fn estimated_cost_usd(&self) -> f64 {
        let input_cost = self.total_input as f64 * 3.0 / 1_000_000.0;
        let output_cost = self.total_output as f64 * 15.0 / 1_000_000.0;
        let cache_read_cost = self.total_cache_read as f64 * 0.30 / 1_000_000.0;
        let cache_write_cost = self.total_cache_write as f64 * 3.75 / 1_000_000.0;
        input_cost + output_cost + cache_read_cost + cache_write_cost
    }

    /// Returns a reference to all recorded usage reports.
    pub fn reports(&self) -> &[UsageReport] {
        &self.reports
    }

    /// Reset the tracker, clearing all accumulated data.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

impl fmt::Display for CostTracker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} calls | {} total tokens | ~${:.4}",
            self.api_calls,
            self.total_tokens(),
            self.estimated_cost_usd()
        )
    }
}
