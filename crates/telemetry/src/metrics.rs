//! Metric types and recording traits for application observability.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A recorded metric value.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum Metric {
    /// A monotonically increasing counter.
    Counter(u64),
    /// A value that can go up or down.
    Gauge(f64),
    /// A distribution of values (stores individual observations).
    Histogram(Vec<f64>),
}

impl Metric {
    /// Create a counter with the given value.
    pub fn counter(value: u64) -> Self {
        Self::Counter(value)
    }

    /// Create a gauge with the given value.
    pub fn gauge(value: f64) -> Self {
        Self::Gauge(value)
    }

    /// Create an empty histogram.
    pub fn histogram() -> Self {
        Self::Histogram(Vec::new())
    }
}

impl fmt::Display for Metric {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Counter(v) => write!(f, "counter({v})"),
            Self::Gauge(v) => write!(f, "gauge({v:.2})"),
            Self::Histogram(values) => {
                if values.is_empty() {
                    write!(f, "histogram(empty)")
                } else {
                    let sum: f64 = values.iter().sum();
                    let avg = sum / values.len() as f64;
                    write!(f, "histogram(n={}, avg={avg:.2})", values.len())
                }
            }
        }
    }
}

/// Trait for recording metrics.
pub trait MetricRecorder: Send + Sync {
    /// Increment a counter by the given amount.
    fn increment_counter(&self, name: &str, value: u64, labels: &Labels);

    /// Set a gauge to the given value.
    fn set_gauge(&self, name: &str, value: f64, labels: &Labels);

    /// Record an observation in a histogram.
    fn record_histogram(&self, name: &str, value: f64, labels: &Labels);
}

/// Labels (key-value pairs) attached to metrics for dimensionality.
pub type Labels = HashMap<String, String>;

/// Helper to create a label set.
pub fn labels(pairs: &[(&str, &str)]) -> Labels {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

/// Aggregated metrics for a single session.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionMetrics {
    /// Session identifier.
    pub session_id: Option<String>,
    /// When the session started.
    pub started_at: Option<DateTime<Utc>>,
    /// When the session ended.
    pub ended_at: Option<DateTime<Utc>>,
    /// Total number of user messages sent.
    pub messages_sent: u64,
    /// Total number of assistant responses received.
    pub responses_received: u64,
    /// Total number of tool invocations.
    pub tool_invocations: u64,
    /// Number of tool invocations that failed.
    pub tool_failures: u64,
    /// Total input tokens consumed.
    pub input_tokens: u64,
    /// Total output tokens generated.
    pub output_tokens: u64,
    /// Total response latency in milliseconds (sum of all responses).
    pub total_latency_ms: u64,
    /// Number of context compaction events.
    pub compaction_count: u64,
    /// Number of errors encountered.
    pub error_count: u64,
}

impl SessionMetrics {
    /// Create a new session metrics tracker.
    pub fn new(session_id: impl Into<String>) -> Self {
        Self {
            session_id: Some(session_id.into()),
            started_at: Some(Utc::now()),
            ..Default::default()
        }
    }

    /// Record a completed response.
    pub fn record_response(&mut self, input_tokens: u64, output_tokens: u64, latency_ms: u64) {
        self.responses_received += 1;
        self.input_tokens += input_tokens;
        self.output_tokens += output_tokens;
        self.total_latency_ms += latency_ms;
    }

    /// Record a tool invocation.
    pub fn record_tool_use(&mut self, success: bool) {
        self.tool_invocations += 1;
        if !success {
            self.tool_failures += 1;
        }
    }

    /// Returns the average response latency in milliseconds.
    pub fn avg_latency_ms(&self) -> f64 {
        if self.responses_received == 0 {
            return 0.0;
        }
        self.total_latency_ms as f64 / self.responses_received as f64
    }

    /// Returns the tool success rate as a fraction (0.0 to 1.0).
    pub fn tool_success_rate(&self) -> f64 {
        if self.tool_invocations == 0 {
            return 1.0;
        }
        (self.tool_invocations - self.tool_failures) as f64 / self.tool_invocations as f64
    }

    /// Returns the total token count.
    pub fn total_tokens(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }

    /// Mark the session as ended.
    pub fn end(&mut self) {
        self.ended_at = Some(Utc::now());
    }

    /// Returns the session duration, if both start and end times are available.
    pub fn duration(&self) -> Option<chrono::Duration> {
        match (self.started_at, self.ended_at) {
            (Some(start), Some(end)) => Some(end.signed_duration_since(start)),
            _ => None,
        }
    }
}

impl fmt::Display for SessionMetrics {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "msgs: {} | tools: {} ({:.0}% ok) | tokens: {} | avg latency: {:.0}ms",
            self.messages_sent,
            self.tool_invocations,
            self.tool_success_rate() * 100.0,
            self.total_tokens(),
            self.avg_latency_ms()
        )
    }
}
