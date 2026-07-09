use serde::{Deserialize, Serialize};

/// Events emitted by an agent sidecar process, mirroring the sidecar's NDJSON
/// out-events 1:1 (see agent-sidecar/index.mjs protocol doc-comment).
///
/// Serialized form (internal, not the IPC payload): `{"type":"content_delta",...}`.
/// The Tauri-facing flat form is [`crate::AgentEventPayload`].
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AgentEvent {
    /// Incremental text output from the agent.
    ContentDelta { text: String },
    /// Incremental thinking/reasoning output from the agent.
    ThinkingDelta { text: String },
    /// A new turn has started.
    TurnStarted { turn_id: String },
    /// A turn completed successfully. Always fires (sidecar `finally` guarantee).
    TurnCompleted { turn_id: String },
    /// A turn was aborted.
    TurnAborted { reason: String },
    /// The agent is requesting approval to execute a tool.
    ApprovalRequired { request_id: String, description: String },
    /// The session is ready to accept input.
    SessionReady {
        /// The Claude Agent SDK session ID (for resume), if available.
        #[serde(skip_serializing_if = "Option::is_none")]
        claude_session_id: Option<String>,
        /// The model confirmed by the SDK.
        #[serde(skip_serializing_if = "Option::is_none")]
        model: Option<String>,
    },
    /// Slash commands reported by the SDK init message (first-class here — no
    /// SessionError smuggling like CodeForge).
    SlashCommands { commands: Vec<String> },
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
    ToolUseStart { tool_id: String, tool_name: String },
    /// Incremental JSON input being generated for a tool call.
    ToolInputDelta { tool_id: String, input_json: String },
    /// Tool input is complete; the tool is now executing.
    ToolUseEnd { tool_id: String },
    /// Result returned from a tool execution.
    ToolResult {
        tool_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tagged_snake_case() {
        let e = AgentEvent::ContentDelta { text: "hi".into() };
        let json = serde_json::to_string(&e).unwrap();
        assert_eq!(json, r#"{"type":"content_delta","text":"hi"}"#);
        let e = AgentEvent::ToolUseStart { tool_id: "t1".into(), tool_name: "Read".into() };
        let json = serde_json::to_string(&e).unwrap();
        assert!(json.contains(r#""type":"tool_use_start""#));
    }
}
