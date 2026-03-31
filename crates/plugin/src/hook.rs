//! Plugin hook types for intercepting application events.

use serde::{Deserialize, Serialize};

/// Lifecycle hooks that plugins can register for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Hook {
    /// Invoked before a tool is executed.
    PreToolUse,
    /// Invoked after a tool has been executed.
    PostToolUse,
    /// Invoked before a message is sent to the AI.
    PreMessage,
    /// Invoked after a message is received from the AI.
    PostMessage,
    /// Invoked when a new session starts.
    SessionStart,
    /// Invoked when a session ends.
    SessionEnd,
    /// Invoked when a thread is created.
    ThreadCreated,
    /// Invoked when the workspace changes.
    WorkspaceChanged,
}

impl Hook {
    /// Returns a human-readable name for this hook.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::PreToolUse => "Pre Tool Use",
            Self::PostToolUse => "Post Tool Use",
            Self::PreMessage => "Pre Message",
            Self::PostMessage => "Post Message",
            Self::SessionStart => "Session Start",
            Self::SessionEnd => "Session End",
            Self::ThreadCreated => "Thread Created",
            Self::WorkspaceChanged => "Workspace Changed",
        }
    }

    /// Returns `true` if this is a "pre" hook that can modify or cancel the action.
    pub fn is_pre_hook(&self) -> bool {
        matches!(self, Self::PreToolUse | Self::PreMessage)
    }
}

/// The result returned by a hook handler.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "action")]
pub enum HookResult {
    /// Continue with the original action unmodified.
    Continue,
    /// Continue but with modified data.
    Modify {
        /// The modified data as a JSON value.
        data: serde_json::Value,
    },
    /// Cancel the action entirely.
    Cancel {
        /// Reason for cancellation.
        reason: String,
    },
    /// An error occurred in the hook handler.
    Error {
        /// Error message.
        message: String,
    },
}

impl HookResult {
    /// Returns `true` if the action should proceed.
    pub fn should_continue(&self) -> bool {
        matches!(self, Self::Continue | Self::Modify { .. })
    }

    /// Returns `true` if the action was cancelled.
    pub fn is_cancelled(&self) -> bool {
        matches!(self, Self::Cancel { .. })
    }
}

/// Context provided to hook handlers.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HookContext {
    /// The hook being invoked.
    pub hook: Hook,
    /// The plugin that registered this handler.
    pub plugin_id: String,
    /// Data associated with the hook event.
    pub data: serde_json::Value,
    /// Timestamp of the hook invocation.
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Trait implemented by hook handler functions.
pub trait HookHandler: Send + Sync {
    /// Handle a hook invocation and return a result.
    fn handle(&self, context: &HookContext) -> HookResult;

    /// The hook this handler is registered for.
    fn hook(&self) -> Hook;

    /// Priority of this handler (lower runs first).
    fn priority(&self) -> i32 {
        0
    }
}
