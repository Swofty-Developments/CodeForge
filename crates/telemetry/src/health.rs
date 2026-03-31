//! Health check types for monitoring application component status.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Trait implemented by components that can report their health.
pub trait HealthCheck: Send + Sync {
    /// The name of this component.
    fn name(&self) -> &str;

    /// Perform a health check and return the current status.
    fn check(&self) -> ComponentHealth;
}

/// The overall health status of a component.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    /// The component is fully operational.
    Healthy,
    /// The component is operational but experiencing issues.
    Degraded,
    /// The component is not operational.
    Unhealthy,
    /// The component's health is unknown (check failed or not yet performed).
    Unknown,
}

impl HealthStatus {
    /// Returns `true` if the component is at least partially operational.
    pub fn is_operational(&self) -> bool {
        matches!(self, Self::Healthy | Self::Degraded)
    }

    /// Merge two health statuses, taking the worse of the two.
    pub fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Unhealthy, _) | (_, Self::Unhealthy) => Self::Unhealthy,
            (Self::Unknown, _) | (_, Self::Unknown) => Self::Unknown,
            (Self::Degraded, _) | (_, Self::Degraded) => Self::Degraded,
            _ => Self::Healthy,
        }
    }
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Healthy => write!(f, "healthy"),
            Self::Degraded => write!(f, "degraded"),
            Self::Unhealthy => write!(f, "unhealthy"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// Detailed health information for a single component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    /// Name of the component.
    pub name: String,
    /// Current health status.
    pub status: HealthStatus,
    /// Human-readable status message.
    pub message: Option<String>,
    /// When this health check was performed.
    pub checked_at: DateTime<Utc>,
    /// How long the health check took, in milliseconds.
    pub check_duration_ms: Option<u64>,
    /// Additional metadata about the component's state.
    pub metadata: HashMap<String, String>,
}

impl ComponentHealth {
    /// Create a healthy component status.
    pub fn healthy(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Healthy,
            message: None,
            checked_at: Utc::now(),
            check_duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create a degraded component status.
    pub fn degraded(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Degraded,
            message: Some(message.into()),
            checked_at: Utc::now(),
            check_duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Create an unhealthy component status.
    pub fn unhealthy(name: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            status: HealthStatus::Unhealthy,
            message: Some(message.into()),
            checked_at: Utc::now(),
            check_duration_ms: None,
            metadata: HashMap::new(),
        }
    }

    /// Set the check duration.
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.check_duration_ms = Some(ms);
        self
    }

    /// Add a metadata entry.
    pub fn with_meta(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.metadata.insert(key.into(), value.into());
        self
    }
}

impl fmt::Display for ComponentHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.name, self.status)?;
        if let Some(ref msg) = self.message {
            write!(f, " - {msg}")?;
        }
        if let Some(ms) = self.check_duration_ms {
            write!(f, " ({ms}ms)")?;
        }
        Ok(())
    }
}

/// Aggregate health report for the entire application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthReport {
    /// Overall application health (worst of all components).
    pub status: HealthStatus,
    /// Individual component health statuses.
    pub components: Vec<ComponentHealth>,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
    /// Application version.
    pub version: Option<String>,
}

impl HealthReport {
    /// Generate a health report from a list of component checks.
    pub fn from_components(components: Vec<ComponentHealth>) -> Self {
        let status = components
            .iter()
            .map(|c| c.status)
            .fold(HealthStatus::Healthy, HealthStatus::merge);
        Self {
            status,
            components,
            generated_at: Utc::now(),
            version: None,
        }
    }

    /// Set the application version on this report.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }

    /// Returns `true` if all components are healthy.
    pub fn is_healthy(&self) -> bool {
        self.status == HealthStatus::Healthy
    }

    /// Returns the number of unhealthy components.
    pub fn unhealthy_count(&self) -> usize {
        self.components
            .iter()
            .filter(|c| c.status == HealthStatus::Unhealthy)
            .count()
    }
}

impl fmt::Display for HealthReport {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Health: {}", self.status)?;
        for component in &self.components {
            writeln!(f, "  {component}")?;
        }
        Ok(())
    }
}
