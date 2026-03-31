//! Observability, metrics, and health check types for CodeForge.
//!
//! Provides metric recording, distributed tracing, and health check
//! abstractions for monitoring the application's operational state.

pub mod analytics;
pub mod crash;
pub mod health;
pub mod logger;
pub mod metrics;
pub mod performance;
pub mod trace;

pub use health::{ComponentHealth, HealthCheck, HealthReport, HealthStatus};
pub use metrics::{Metric, MetricRecorder, SessionMetrics};
pub use trace::{SpanStatus, TraceContext, TraceSpan};
