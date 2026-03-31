//! Analytics event tracking, batching, anonymization, and consent.
//!
//! Provides types for recording application events, batching them
//! for efficient transmission, anonymizing user data, and managing
//! user consent for telemetry collection.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Types of analytics events that can be recorded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum EventType {
    /// A new session was started.
    SessionStart,
    /// A session ended.
    SessionEnd,
    /// A tool was invoked.
    ToolUsed,
    /// An error occurred.
    ErrorOccurred,
    /// A feature was used.
    FeatureUsed,
    /// A model was queried.
    ModelQueried,
    /// A file was opened.
    FileOpened,
    /// A command was executed.
    CommandExecuted,
    /// A thread was created.
    ThreadCreated,
    /// A plugin was activated.
    PluginActivated,
    /// User changed settings.
    SettingsChanged,
    /// The application was updated.
    AppUpdated,
}

impl EventType {
    /// Return all event types.
    pub fn all() -> &'static [EventType] {
        &[
            EventType::SessionStart,
            EventType::SessionEnd,
            EventType::ToolUsed,
            EventType::ErrorOccurred,
            EventType::FeatureUsed,
            EventType::ModelQueried,
            EventType::FileOpened,
            EventType::CommandExecuted,
            EventType::ThreadCreated,
            EventType::PluginActivated,
            EventType::SettingsChanged,
            EventType::AppUpdated,
        ]
    }

    /// Whether this event type contains potentially sensitive data.
    pub fn is_sensitive(&self) -> bool {
        matches!(
            self,
            EventType::FileOpened | EventType::CommandExecuted | EventType::ErrorOccurred
        )
    }
}

impl fmt::Display for EventType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EventType::SessionStart => write!(f, "session.start"),
            EventType::SessionEnd => write!(f, "session.end"),
            EventType::ToolUsed => write!(f, "tool.used"),
            EventType::ErrorOccurred => write!(f, "error.occurred"),
            EventType::FeatureUsed => write!(f, "feature.used"),
            EventType::ModelQueried => write!(f, "model.queried"),
            EventType::FileOpened => write!(f, "file.opened"),
            EventType::CommandExecuted => write!(f, "command.executed"),
            EventType::ThreadCreated => write!(f, "thread.created"),
            EventType::PluginActivated => write!(f, "plugin.activated"),
            EventType::SettingsChanged => write!(f, "settings.changed"),
            EventType::AppUpdated => write!(f, "app.updated"),
        }
    }
}

/// A single analytics event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyticsEvent {
    /// The event type.
    pub event_type: EventType,
    /// When the event occurred.
    pub timestamp: DateTime<Utc>,
    /// Anonymous session identifier.
    pub session_id: String,
    /// Event properties.
    pub properties: HashMap<String, serde_json::Value>,
    /// Application version.
    pub app_version: String,
    /// The platform (e.g., "macos", "linux", "windows").
    pub platform: String,
}

impl AnalyticsEvent {
    /// Create a new event.
    pub fn new(event_type: EventType, session_id: impl Into<String>) -> Self {
        Self {
            event_type,
            timestamp: Utc::now(),
            session_id: session_id.into(),
            properties: HashMap::new(),
            app_version: String::new(),
            platform: std::env::consts::OS.to_string(),
        }
    }

    /// Set the app version.
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.app_version = version.into();
        self
    }

    /// Add a string property.
    pub fn with_property(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties
            .insert(key.into(), serde_json::Value::String(value.into()));
        self
    }

    /// Add a numeric property.
    pub fn with_number(mut self, key: impl Into<String>, value: f64) -> Self {
        self.properties.insert(
            key.into(),
            serde_json::Value::Number(
                serde_json::Number::from_f64(value).unwrap_or_else(|| serde_json::Number::from(0)),
            ),
        );
        self
    }

    /// Add a boolean property.
    pub fn with_bool(mut self, key: impl Into<String>, value: bool) -> Self {
        self.properties
            .insert(key.into(), serde_json::Value::Bool(value));
        self
    }

    /// Anonymize sensitive fields in this event.
    pub fn anonymize(&mut self) {
        // Remove any properties that could identify the user.
        let sensitive_keys = ["path", "file", "command", "error_message", "user", "email"];
        for key in &sensitive_keys {
            if let Some(value) = self.properties.get_mut(*key) {
                if let Some(s) = value.as_str() {
                    *value = serde_json::Value::String(anonymize_string(s));
                }
            }
        }
    }
}

impl fmt::Display for AnalyticsEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "[{}] {} (session: {}, props: {})",
            self.timestamp.format("%H:%M:%S"),
            self.event_type,
            &self.session_id[..self.session_id.len().min(8)],
            self.properties.len()
        )
    }
}

/// Anonymize a string by hashing it.
pub fn anonymize_string(s: &str) -> String {
    // Simple hash-based anonymization.
    let mut hash: u64 = 0x517cc1b727220a95;
    for byte in s.bytes() {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(0x6c62272e07bb0142);
    }
    format!("anon_{hash:016x}")
}

/// Anonymize a file path, keeping only the extension.
pub fn anonymize_path(path: &str) -> String {
    let ext = path
        .rsplit('.')
        .next()
        .filter(|e| e.len() <= 10)
        .unwrap_or("");
    if ext.is_empty() {
        "anon_file".to_string()
    } else {
        format!("anon_file.{ext}")
    }
}

/// A batch of events for efficient transmission.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventBatch {
    /// The events in this batch.
    pub events: Vec<AnalyticsEvent>,
    /// When this batch was created.
    pub created_at: DateTime<Utc>,
    /// Batch sequence number.
    pub sequence: u64,
    /// Whether events have been anonymized.
    pub anonymized: bool,
}

impl EventBatch {
    /// Create a new batch from events.
    pub fn new(events: Vec<AnalyticsEvent>, sequence: u64) -> Self {
        Self {
            events,
            created_at: Utc::now(),
            sequence,
            anonymized: false,
        }
    }

    /// Return the number of events.
    pub fn len(&self) -> usize {
        self.events.len()
    }

    /// Check if empty.
    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    /// Anonymize all events in the batch.
    pub fn anonymize_all(&mut self) {
        for event in &mut self.events {
            event.anonymize();
        }
        self.anonymized = true;
    }

    /// Serialize to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Count events by type.
    pub fn event_counts(&self) -> HashMap<EventType, usize> {
        let mut counts = HashMap::new();
        for event in &self.events {
            *counts.entry(event.event_type).or_insert(0) += 1;
        }
        counts
    }
}

impl fmt::Display for EventBatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Batch #{}: {} events ({})",
            self.sequence,
            self.events.len(),
            if self.anonymized {
                "anonymized"
            } else {
                "raw"
            }
        )
    }
}

/// User consent state for telemetry collection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsentState {
    /// User has not been asked yet.
    Unknown,
    /// User has granted consent.
    Granted,
    /// User has denied consent.
    Denied,
    /// User has granted consent for anonymized data only.
    AnonymousOnly,
}

impl Default for ConsentState {
    fn default() -> Self {
        ConsentState::Unknown
    }
}

impl fmt::Display for ConsentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsentState::Unknown => write!(f, "unknown"),
            ConsentState::Granted => write!(f, "granted"),
            ConsentState::Denied => write!(f, "denied"),
            ConsentState::AnonymousOnly => write!(f, "anonymous-only"),
        }
    }
}

/// Consent tracking configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsentConfig {
    /// Current consent state.
    pub state: ConsentState,
    /// When consent was last updated (ISO 8601).
    pub updated_at: Option<String>,
    /// Which event types the user has consented to.
    pub allowed_events: Vec<EventType>,
    /// Privacy policy version the user consented to.
    pub policy_version: Option<String>,
}

impl Default for ConsentConfig {
    fn default() -> Self {
        Self {
            state: ConsentState::Unknown,
            updated_at: None,
            allowed_events: Vec::new(),
            policy_version: None,
        }
    }
}

impl ConsentConfig {
    /// Create a config with full consent.
    pub fn full_consent() -> Self {
        Self {
            state: ConsentState::Granted,
            updated_at: Some(Utc::now().to_rfc3339()),
            allowed_events: EventType::all().to_vec(),
            policy_version: None,
        }
    }

    /// Create a config with anonymous-only consent.
    pub fn anonymous_only() -> Self {
        Self {
            state: ConsentState::AnonymousOnly,
            updated_at: Some(Utc::now().to_rfc3339()),
            allowed_events: EventType::all()
                .iter()
                .filter(|e| !e.is_sensitive())
                .copied()
                .collect(),
            policy_version: None,
        }
    }

    /// Check if an event type is allowed under current consent.
    pub fn is_allowed(&self, event_type: EventType) -> bool {
        match self.state {
            ConsentState::Granted => true,
            ConsentState::AnonymousOnly => self.allowed_events.contains(&event_type),
            ConsentState::Denied | ConsentState::Unknown => false,
        }
    }

    /// Whether any telemetry is allowed.
    pub fn is_telemetry_enabled(&self) -> bool {
        matches!(
            self.state,
            ConsentState::Granted | ConsentState::AnonymousOnly
        )
    }
}

/// Simple in-memory event collector with batching.
#[derive(Debug, Clone)]
pub struct EventCollector {
    /// Pending events not yet batched.
    pending: Vec<AnalyticsEvent>,
    /// Maximum events before auto-flushing.
    max_pending: usize,
    /// Next batch sequence number.
    next_sequence: u64,
    /// Consent configuration.
    consent: ConsentConfig,
}

impl EventCollector {
    /// Create a new collector.
    pub fn new(consent: ConsentConfig) -> Self {
        Self {
            pending: Vec::new(),
            max_pending: 100,
            next_sequence: 0,
            consent,
        }
    }

    /// Record an event (respecting consent).
    pub fn record(&mut self, event: AnalyticsEvent) {
        if !self.consent.is_allowed(event.event_type) {
            return;
        }
        self.pending.push(event);
    }

    /// Flush pending events into a batch.
    pub fn flush(&mut self) -> Option<EventBatch> {
        if self.pending.is_empty() {
            return None;
        }
        let events = std::mem::take(&mut self.pending);
        let mut batch = EventBatch::new(events, self.next_sequence);
        self.next_sequence += 1;

        if self.consent.state == ConsentState::AnonymousOnly {
            batch.anonymize_all();
        }

        Some(batch)
    }

    /// Check if there are pending events.
    pub fn has_pending(&self) -> bool {
        !self.pending.is_empty()
    }

    /// Return the number of pending events.
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Should auto-flush.
    pub fn should_flush(&self) -> bool {
        self.pending.len() >= self.max_pending
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_creation() {
        let event = AnalyticsEvent::new(EventType::ToolUsed, "session-1")
            .with_property("tool", "read_file")
            .with_number("duration_ms", 42.0);
        assert_eq!(event.event_type, EventType::ToolUsed);
        assert_eq!(event.properties.len(), 2);
    }

    #[test]
    fn anonymization() {
        let mut event = AnalyticsEvent::new(EventType::FileOpened, "s1")
            .with_property("path", "/home/user/secret.txt");
        event.anonymize();
        let path = event.properties.get("path").unwrap().as_str().unwrap();
        assert!(path.starts_with("anon_"));
    }

    #[test]
    fn consent_filtering() {
        let consent = ConsentConfig::anonymous_only();
        assert!(!consent.is_allowed(EventType::FileOpened)); // sensitive
        assert!(consent.is_allowed(EventType::SessionStart)); // non-sensitive
    }

    #[test]
    fn event_collector() {
        let mut collector = EventCollector::new(ConsentConfig::full_consent());
        collector.record(AnalyticsEvent::new(EventType::SessionStart, "s1"));
        collector.record(AnalyticsEvent::new(EventType::ToolUsed, "s1"));
        assert_eq!(collector.pending_count(), 2);

        let batch = collector.flush().unwrap();
        assert_eq!(batch.len(), 2);
        assert_eq!(collector.pending_count(), 0);
    }

    #[test]
    fn denied_consent() {
        let consent = ConsentConfig::default(); // Unknown = denied
        let mut collector = EventCollector::new(consent);
        collector.record(AnalyticsEvent::new(EventType::SessionStart, "s1"));
        assert_eq!(collector.pending_count(), 0); // Dropped due to no consent.
    }
}
