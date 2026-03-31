//! Distributed tracing types for correlating operations across components.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// A span representing a unit of work in a distributed trace.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceSpan {
    /// Unique identifier for this span.
    pub span_id: Uuid,
    /// The trace this span belongs to.
    pub trace_id: Uuid,
    /// The parent span, if this is a child span.
    pub parent_span_id: Option<Uuid>,
    /// Human-readable operation name.
    pub operation: String,
    /// When the span started.
    pub start_time: DateTime<Utc>,
    /// When the span ended (None if still in progress).
    pub end_time: Option<DateTime<Utc>>,
    /// The outcome of the operation.
    pub status: SpanStatus,
    /// Key-value attributes attached to this span.
    pub attributes: HashMap<String, SpanAttribute>,
    /// Events (logs) recorded during this span.
    pub events: Vec<SpanEvent>,
}

impl TraceSpan {
    /// Create a new span for the given operation.
    pub fn new(trace_id: Uuid, operation: impl Into<String>) -> Self {
        Self {
            span_id: Uuid::new_v4(),
            trace_id,
            parent_span_id: None,
            operation: operation.into(),
            start_time: Utc::now(),
            end_time: None,
            status: SpanStatus::InProgress,
            attributes: HashMap::new(),
            events: Vec::new(),
        }
    }

    /// Create a child span of this span.
    pub fn child(&self, operation: impl Into<String>) -> Self {
        let mut span = Self::new(self.trace_id, operation);
        span.parent_span_id = Some(self.span_id);
        span
    }

    /// Set an attribute on this span.
    pub fn set_attribute(&mut self, key: impl Into<String>, value: impl Into<SpanAttribute>) {
        self.attributes.insert(key.into(), value.into());
    }

    /// Record an event (log entry) on this span.
    pub fn add_event(&mut self, name: impl Into<String>) {
        self.events.push(SpanEvent {
            name: name.into(),
            timestamp: Utc::now(),
            attributes: HashMap::new(),
        });
    }

    /// End this span with a success status.
    pub fn end_ok(&mut self) {
        self.end_time = Some(Utc::now());
        self.status = SpanStatus::Ok;
    }

    /// End this span with an error status.
    pub fn end_error(&mut self, message: impl Into<String>) {
        self.end_time = Some(Utc::now());
        self.status = SpanStatus::Error {
            message: message.into(),
        };
    }

    /// Returns the span duration, if the span has ended.
    pub fn duration(&self) -> Option<chrono::Duration> {
        self.end_time
            .map(|end| end.signed_duration_since(self.start_time))
    }

    /// Returns `true` if the span is still in progress.
    pub fn is_in_progress(&self) -> bool {
        matches!(self.status, SpanStatus::InProgress)
    }
}

impl fmt::Display for TraceSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}] {}", self.span_id, self.operation)?;
        if let Some(duration) = self.duration() {
            write!(f, " ({}ms)", duration.num_milliseconds())?;
        }
        write!(f, " {}", self.status)
    }
}

/// The status of a trace span.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum SpanStatus {
    /// The span is still in progress.
    InProgress,
    /// The span completed successfully.
    Ok,
    /// The span completed with an error.
    Error {
        /// Error message.
        message: String,
    },
}

impl fmt::Display for SpanStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InProgress => write!(f, "IN_PROGRESS"),
            Self::Ok => write!(f, "OK"),
            Self::Error { message } => write!(f, "ERROR: {message}"),
        }
    }
}

/// An attribute value attached to a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum SpanAttribute {
    /// A string value.
    String(String),
    /// An integer value.
    Int(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Bool(bool),
}

impl From<String> for SpanAttribute {
    fn from(s: String) -> Self {
        Self::String(s)
    }
}

impl From<&str> for SpanAttribute {
    fn from(s: &str) -> Self {
        Self::String(s.to_string())
    }
}

impl From<i64> for SpanAttribute {
    fn from(v: i64) -> Self {
        Self::Int(v)
    }
}

impl From<f64> for SpanAttribute {
    fn from(v: f64) -> Self {
        Self::Float(v)
    }
}

impl From<bool> for SpanAttribute {
    fn from(v: bool) -> Self {
        Self::Bool(v)
    }
}

/// An event (log entry) recorded within a span.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpanEvent {
    /// Event name / message.
    pub name: String,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Attributes attached to this event.
    pub attributes: HashMap<String, SpanAttribute>,
}

/// Context for propagating trace information across boundaries.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceContext {
    /// The trace ID.
    pub trace_id: Uuid,
    /// The current span ID.
    pub span_id: Uuid,
    /// Sampling decision.
    pub sampled: bool,
}

impl TraceContext {
    /// Create a new trace context, starting a new trace.
    pub fn new() -> Self {
        Self {
            trace_id: Uuid::new_v4(),
            span_id: Uuid::new_v4(),
            sampled: true,
        }
    }

    /// Serialize to the W3C traceparent format.
    pub fn to_traceparent(&self) -> String {
        let sampled_flag = if self.sampled { "01" } else { "00" };
        format!(
            "00-{}-{}-{sampled_flag}",
            self.trace_id.as_simple(),
            &self.span_id.as_simple().to_string()[..16],
        )
    }
}

impl Default for TraceContext {
    fn default() -> Self {
        Self::new()
    }
}
