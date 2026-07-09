use serde::Serialize;

use crate::AgentEvent;

/// Flat "union" payload emitted on the single Tauri `agent-event` channel.
///
/// One `event_type` discriminator plus optional fields, all camelCase on the
/// wire; the frontend demuxes by `sessionId` and switches on `eventType`.
/// `event_type` values are the snake_case [`AgentEvent`] variant names:
/// `content_delta | thinking_delta | turn_started | turn_completed |
///  turn_aborted | approval_required | session_ready | slash_commands |
///  session_error | session_resume_failed | usage_report | tool_use_start |
///  tool_input_delta | tool_use_end | tool_result`, plus the Rust-originated
/// `session_persistence_degraded` (see [`AgentEventPayload::persistence_degraded`]).
#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEventPayload {
    pub session_id: String,
    /// Claude Agent SDK session id ("thread"), when known.
    pub thread_id: String,
    pub event_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub turn_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub input_json: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_output: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_read_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_write_tokens: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commands: Option<Vec<String>>,
}

impl AgentEventPayload {
    /// Map an internal [`AgentEvent`] to the flat Tauri payload.
    pub fn from_event(session_id: &str, thread_id: &str, event: &AgentEvent) -> Self {
        let mut p = AgentEventPayload {
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            ..Default::default()
        };
        match event {
            AgentEvent::ContentDelta { text } => {
                p.event_type = "content_delta".into();
                p.text = Some(text.clone());
            }
            AgentEvent::ThinkingDelta { text } => {
                p.event_type = "thinking_delta".into();
                p.text = Some(text.clone());
            }
            AgentEvent::TurnStarted { turn_id } => {
                p.event_type = "turn_started".into();
                p.turn_id = Some(turn_id.clone());
            }
            AgentEvent::TurnCompleted { turn_id } => {
                p.event_type = "turn_completed".into();
                p.turn_id = Some(turn_id.clone());
            }
            AgentEvent::TurnAborted { reason } => {
                p.event_type = "turn_aborted".into();
                p.reason = Some(reason.clone());
            }
            AgentEvent::ApprovalRequired { request_id, description } => {
                p.event_type = "approval_required".into();
                p.request_id = Some(request_id.clone());
                p.description = Some(description.clone());
            }
            AgentEvent::SessionReady { claude_session_id, model } => {
                p.event_type = "session_ready".into();
                p.message = claude_session_id.clone();
                p.model = model.clone();
            }
            AgentEvent::SlashCommands { commands } => {
                p.event_type = "slash_commands".into();
                p.commands = Some(commands.clone());
            }
            AgentEvent::SessionError { message } => {
                p.event_type = "session_error".into();
                p.message = Some(message.clone());
            }
            AgentEvent::SessionResumeFailed { claude_session_id } => {
                p.event_type = "session_resume_failed".into();
                // `message` carries the SDK session id that could not be resumed.
                p.message = Some(claude_session_id.clone());
            }
            AgentEvent::UsageReport {
                input_tokens,
                output_tokens,
                cache_read_tokens,
                cache_write_tokens,
                cost_usd,
                model,
            } => {
                p.event_type = "usage_report".into();
                p.input_tokens = Some(*input_tokens);
                p.output_tokens = Some(*output_tokens);
                p.cache_read_tokens = Some(*cache_read_tokens);
                p.cache_write_tokens = Some(*cache_write_tokens);
                p.cost_usd = Some(*cost_usd);
                p.model = Some(model.clone());
            }
            AgentEvent::ToolUseStart { tool_id, tool_name } => {
                p.event_type = "tool_use_start".into();
                p.tool_id = Some(tool_id.clone());
                p.tool_name = Some(tool_name.clone());
            }
            AgentEvent::ToolInputDelta { tool_id, input_json } => {
                p.event_type = "tool_input_delta".into();
                p.tool_id = Some(tool_id.clone());
                p.input_json = Some(input_json.clone());
            }
            AgentEvent::ToolUseEnd { tool_id } => {
                p.event_type = "tool_use_end".into();
                p.tool_id = Some(tool_id.clone());
            }
            AgentEvent::ToolResult { tool_id, tool_name, content, is_error } => {
                p.event_type = "tool_result".into();
                p.tool_id = Some(tool_id.clone());
                p.tool_name = Some(tool_name.clone());
                p.tool_output = Some(content.clone());
                p.is_error = Some(*is_error);
            }
        }
        p
    }

    /// Rust-originated event (not a sidecar out-event): a durable persistence
    /// write for this session failed. The session keeps running but its stored
    /// history/usage may be incomplete — surfaced, never swallowed.
    pub fn persistence_degraded(session_id: &str, thread_id: &str, message: &str) -> Self {
        AgentEventPayload {
            session_id: session_id.to_string(),
            thread_id: thread_id.to_string(),
            event_type: "session_persistence_degraded".into(),
            message: Some(message.to_string()),
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn camel_case_and_skip_none() {
        let e = AgentEvent::ToolResult {
            tool_id: "t1".into(),
            tool_name: "Bash".into(),
            content: "ok".into(),
            is_error: false,
        };
        let p = AgentEventPayload::from_event("s1", "th1", &e);
        let json = serde_json::to_string(&p).unwrap();
        assert!(json.contains("\"sessionId\":\"s1\""));
        assert!(json.contains("\"eventType\":\"tool_result\""));
        assert!(json.contains("\"toolOutput\":\"ok\""));
        assert!(json.contains("\"isError\":false"));
        assert!(!json.contains("costUsd"));
    }
}
