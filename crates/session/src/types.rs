use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Provider, SessionId};

/// Current status of an agent session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    Connecting,
    Ready,
    Running,
    Interrupted,
    Stopped,
}

/// How command approvals are handled in a session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalMode {
    /// Every command execution requires explicit approval.
    Supervised,
    /// Commands are automatically approved.
    AutoApprove,
}

/// Events emitted by an agent subprocess.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Incremental text output from the agent.
    ContentDelta { text: String },
    /// A new turn has started.
    TurnStarted { turn_id: String },
    /// A turn completed successfully.
    TurnCompleted { turn_id: String },
    /// A turn was aborted.
    TurnAborted { reason: String },
    /// The agent is requesting approval to execute a command.
    ApprovalRequired {
        request_id: String,
        description: String,
    },
    /// The session is ready to accept input.
    SessionReady {
        /// The Claude CLI session ID (for `--resume`), if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        claude_session_id: Option<String>,
        /// The model confirmed by the SDK.
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// An error occurred in the session.
    SessionError { message: String },
    /// Usage/cost report for a completed turn.
    UsageReport {
        input_tokens: u64,
        output_tokens: u64,
        cache_read_tokens: u64,
        cache_write_tokens: u64,
        cost_usd: f64,
        model: String,
    },
    /// A tool use block has started (agent is calling a tool).
    ToolUseStart {
        tool_id: String,
        tool_name: String,
    },
    /// Incremental JSON input being generated for a tool call.
    ToolInputDelta {
        tool_id: String,
        input_json: String,
    },
    /// Tool input is complete; the tool is now executing.
    ToolUseEnd {
        tool_id: String,
    },
    /// Result returned from a tool execution.
    ToolResult {
        tool_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
    /// Incremental thinking/reasoning output from the agent.
    ThinkingDelta { text: String },
}

/// Metadata for an active session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    pub id: SessionId,
    pub provider: Provider,
    pub status: SessionStatus,
    /// PID of the child process, if available.
    pub pid: Option<u32>,
    pub approval_mode: ApprovalMode,
}

impl Session {
    pub fn new(provider: Provider, approval_mode: ApprovalMode) -> Self {
        Self {
            id: Uuid::new_v4(),
            provider,
            status: SessionStatus::Connecting,
            pid: None,
            approval_mode,
        }
    }
}
