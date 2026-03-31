//! Application-level event types for cross-component communication.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Metadata attached to every application event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    /// Unique identifier for this event.
    pub event_id: Uuid,
    /// Timestamp when the event was created.
    pub timestamp: DateTime<Utc>,
    /// The thread this event is associated with, if any.
    pub thread_id: Option<Uuid>,
    /// The session this event is associated with, if any.
    pub session_id: Option<String>,
}

impl EventMetadata {
    /// Create new event metadata with the current timestamp.
    pub fn now() -> Self {
        Self {
            event_id: Uuid::new_v4(),
            timestamp: Utc::now(),
            thread_id: None,
            session_id: None,
        }
    }

    /// Attach a thread ID to this metadata.
    pub fn with_thread(mut self, thread_id: Uuid) -> Self {
        self.thread_id = Some(thread_id);
        self
    }

    /// Attach a session ID to this metadata.
    pub fn with_session(mut self, session_id: impl Into<String>) -> Self {
        self.session_id = Some(session_id.into());
        self
    }
}

/// All application events that can occur during CodeForge operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum AppEvent {
    /// A new thread was created.
    ThreadCreated {
        /// Event metadata.
        meta: EventMetadata,
        /// Name assigned to the thread.
        thread_name: String,
    },

    /// A thread was deleted.
    ThreadDeleted {
        /// Event metadata.
        meta: EventMetadata,
    },

    /// A message was received from the user or assistant.
    MessageReceived {
        /// Event metadata.
        meta: EventMetadata,
        /// The role that sent the message.
        role: MessageRole,
        /// Byte length of the message content.
        content_length: usize,
    },

    /// A session was started or resumed.
    SessionStarted {
        /// Event metadata.
        meta: EventMetadata,
        /// The AI model used for the session.
        model: String,
        /// Whether this is a resumed session.
        resumed: bool,
    },

    /// A session has ended.
    SessionEnded {
        /// Event metadata.
        meta: EventMetadata,
        /// Total tokens consumed during the session.
        total_tokens: u64,
    },

    /// A tool was executed by the AI assistant.
    ToolExecuted {
        /// Event metadata.
        meta: EventMetadata,
        /// Name of the tool that was executed.
        tool_name: String,
        /// Whether the execution succeeded.
        success: bool,
        /// Duration of tool execution in milliseconds.
        duration_ms: u64,
    },

    /// A git operation completed.
    GitOperationCompleted {
        /// Event metadata.
        meta: EventMetadata,
        /// The git operation that was performed.
        operation: String,
        /// Whether the operation succeeded.
        success: bool,
    },

    /// Workspace state changed (file created, deleted, modified).
    WorkspaceChanged {
        /// Event metadata.
        meta: EventMetadata,
        /// Path that changed, relative to workspace root.
        path: String,
        /// The kind of change.
        change_kind: ChangeKind,
    },

    /// An error occurred that should be surfaced to the user.
    ErrorOccurred {
        /// Event metadata.
        meta: EventMetadata,
        /// Error severity level.
        severity: ErrorSeverity,
        /// Human-readable error message.
        message: String,
    },
}

/// The role of a message sender.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MessageRole {
    /// Message from the human user.
    User,
    /// Message from the AI assistant.
    Assistant,
    /// System-level message.
    System,
}

/// The kind of filesystem change observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ChangeKind {
    /// A file or directory was created.
    Created,
    /// A file was modified.
    Modified,
    /// A file or directory was deleted.
    Deleted,
    /// A file or directory was renamed.
    Renamed,
}

/// Severity level for error events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorSeverity {
    /// Informational notice, not an actual error.
    Info,
    /// A warning that does not prevent operation.
    Warning,
    /// An error that affects the current operation.
    Error,
    /// A critical failure requiring immediate attention.
    Critical,
}
