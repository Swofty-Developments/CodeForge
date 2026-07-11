//! NDJSON protocol glue: sidecar stdout lines → [`AgentEvent`], plus the
//! first-query augmentation that splices session init params into the initial
//! `query` command (subsequent queries rely on sidecar-side state).

use std::sync::Mutex;

use serde_json::{Map, Value};
use tracing::debug;

use crate::{AgentEvent, SessionMode};

/// Parameters captured at session creation time, injected into the first
/// `query` command sent to the sidecar (consumed exactly once).
pub(crate) struct SidecarInitParams {
    pub cwd: String,
    pub model: Option<String>,
    pub permission_mode: Option<String>,
    /// The DB-authoritative engagement mode for the FIRST query (Fresh/Resume).
    pub mode: SessionMode,
}

/// Stamp `{ mode, resumeSessionId? }` onto a `query` command object so the
/// sidecar engages the SDK from a named mode rather than inferring it.
fn stamp_mode(obj: &mut Map<String, Value>, mode: &SessionMode) {
    obj.insert("mode".into(), Value::String(mode.wire().to_string()));
    if let SessionMode::Resume { claude_session_id } = mode {
        obj.insert("resumeSessionId".into(), Value::String(claude_session_id.clone()));
    }
}

/// For a `{"type":"query",...}` command, stamp the engagement mode. The FIRST
/// query also gets cwd/model/permissionMode from the (once-consumed) init
/// params and its DB-decided mode; every subsequent query is
/// [`SessionMode::ContinueInProcess`]. Explicit model/permissionMode already on
/// the command win; cwd and mode are always set by us. Non-`query` lines pass
/// through untouched.
pub(crate) fn augment_query_if_needed(msg: &str, init: &Mutex<Option<SidecarInitParams>>) -> String {
    let mut parsed: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(_) => return msg.to_string(),
    };
    if parsed.get("type").and_then(Value::as_str) != Some("query") {
        return msg.to_string();
    }

    let params = {
        let mut guard = init.lock().unwrap_or_else(|e| e.into_inner());
        guard.take()
    };

    if let Some(obj) = parsed.as_object_mut() {
        match params {
            Some(p) => {
                obj.insert("cwd".into(), Value::String(p.cwd));
                if let Some(m) = p.model {
                    obj.entry("model").or_insert(Value::String(m));
                }
                if let Some(pm) = p.permission_mode {
                    obj.entry("permissionMode").or_insert(Value::String(pm));
                }
                stamp_mode(obj, &p.mode);
            }
            // Init already consumed → this is a follow-up query in the same
            // live sidecar: continue the in-process SDK conversation.
            None => stamp_mode(obj, &SessionMode::ContinueInProcess),
        }
    }

    // Infallible: `parsed` came from valid JSON and we only inserted strings.
    serde_json::to_string(&parsed).expect("re-serializing an augmented query cannot fail")
}

/// Parse one sidecar stdout NDJSON line into zero or more [`AgentEvent`]s.
/// Unparseable or unknown lines are skipped, never fatal — the SDK/CLI may
/// print stray non-JSON output.
pub(crate) fn parse_sidecar_line(line: &str) -> Vec<AgentEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let value: Value = match serde_json::from_str(trimmed) {
        Ok(v) => v,
        Err(_) => {
            debug!("skipping non-JSON sidecar stdout line: {trimmed}");
            return Vec::new();
        }
    };
    let Some(obj) = value.as_object() else {
        return Vec::new();
    };

    match str_of(obj, "type").as_str() {
        // Sidecar booted; the SDK session id only arrives with the first
        // query's init message (as `session_ready`).
        "ready" => vec![AgentEvent::SessionReady { claude_session_id: None, model: None }],
        "session_ready" => vec![AgentEvent::SessionReady {
            claude_session_id: opt_str(obj, "sessionId"),
            model: opt_str(obj, "model"),
        }],
        "slash_commands" => {
            let commands: Vec<String> = obj
                .get("commands")
                .and_then(Value::as_array)
                .map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect())
                .unwrap_or_default();
            if commands.is_empty() {
                Vec::new()
            } else {
                vec![AgentEvent::SlashCommands { commands }]
            }
        }
        "turn_started" => vec![AgentEvent::TurnStarted { turn_id: "sidecar".into() }],
        "turn_completed" => vec![AgentEvent::TurnCompleted { turn_id: str_of(obj, "sessionId") }],
        "text_delta" => {
            let text = str_of(obj, "text");
            // Empty deltas dropped here to avoid no-op renders downstream.
            if text.is_empty() {
                Vec::new()
            } else {
                vec![AgentEvent::ContentDelta { text }]
            }
        }
        "thinking_delta" => {
            let text = str_of(obj, "text");
            if text.is_empty() {
                Vec::new()
            } else {
                vec![AgentEvent::ThinkingDelta { text }]
            }
        }
        "tool_use_start" => vec![AgentEvent::ToolUseStart {
            tool_id: str_of(obj, "toolId"),
            tool_name: str_or(obj, "toolName", "tool"),
        }],
        "tool_use_input" => vec![AgentEvent::ToolInputDelta {
            tool_id: str_of(obj, "toolId"),
            input_json: str_of(obj, "inputJson"),
        }],
        "tool_result" => vec![AgentEvent::ToolResult {
            tool_id: str_of(obj, "toolId"),
            tool_name: str_of(obj, "toolName"),
            content: str_of(obj, "content"),
            is_error: obj.get("isError").and_then(Value::as_bool).unwrap_or(false),
        }],
        "approval_request" => vec![AgentEvent::ApprovalRequired {
            request_id: str_of(obj, "requestId"),
            description: format!("{}: {}", str_of(obj, "toolName"), pretty(obj.get("input"))),
        }],
        "ask_user_question" => vec![AgentEvent::AskUserQuestion {
            request_id: str_of(obj, "requestId"),
            questions: obj.get("questions").cloned().unwrap_or(serde_json::json!([])),
        }],
        "usage" => vec![AgentEvent::UsageReport {
            input_tokens: u64_of(obj, "inputTokens"),
            output_tokens: u64_of(obj, "outputTokens"),
            cache_read_tokens: u64_of(obj, "cacheRead"),
            cache_write_tokens: u64_of(obj, "cacheWrite"),
            cost_usd: obj.get("costUsd").and_then(Value::as_f64).unwrap_or(0.0),
            model: str_or(obj, "model", "unknown"),
        }],
        "error" => vec![AgentEvent::SessionError { message: str_or(obj, "message", "Unknown error") }],
        "session_resume_failed" => vec![AgentEvent::SessionResumeFailed {
            claude_session_id: str_of(obj, "claudeSessionId"),
        }],
        other => {
            debug!("unhandled sidecar event type: {other}");
            Vec::new()
        }
    }
}

fn str_of(obj: &Map<String, Value>, key: &str) -> String {
    str_or(obj, key, "")
}

fn str_or(obj: &Map<String, Value>, key: &str, default: &str) -> String {
    obj.get(key).and_then(Value::as_str).unwrap_or(default).to_string()
}

fn opt_str(obj: &Map<String, Value>, key: &str) -> Option<String> {
    obj.get(key).and_then(Value::as_str).map(String::from)
}

fn u64_of(obj: &Map<String, Value>, key: &str) -> u64 {
    obj.get(key).and_then(Value::as_u64).unwrap_or(0)
}

fn pretty(v: Option<&Value>) -> String {
    v.map(|v| serde_json::to_string_pretty(v).unwrap_or_default()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn one(line: &str) -> AgentEvent {
        let mut events = parse_sidecar_line(line);
        assert_eq!(events.len(), 1, "expected exactly one event for {line}");
        events.remove(0)
    }

    #[test]
    fn parses_every_out_event_fixture() {
        assert!(matches!(
            one(r#"{"type":"ready"}"#),
            AgentEvent::SessionReady { claude_session_id: None, model: None }
        ));
        match one(r#"{"type":"session_ready","sessionId":"abc-123","model":"claude-haiku-4-5"}"#) {
            AgentEvent::SessionReady { claude_session_id, model } => {
                assert_eq!(claude_session_id.as_deref(), Some("abc-123"));
                assert_eq!(model.as_deref(), Some("claude-haiku-4-5"));
            }
            other => panic!("wrong event: {other:?}"),
        }
        match one(r#"{"type":"slash_commands","commands":["/commit","/review"]}"#) {
            AgentEvent::SlashCommands { commands } => assert_eq!(commands, vec!["/commit", "/review"]),
            other => panic!("wrong event: {other:?}"),
        }
        assert!(matches!(
            one(r#"{"type":"turn_started"}"#),
            AgentEvent::TurnStarted { turn_id } if turn_id == "sidecar"
        ));
        assert!(matches!(
            one(r#"{"type":"text_delta","text":"Hello"}"#),
            AgentEvent::ContentDelta { text } if text == "Hello"
        ));
        assert!(matches!(
            one(r#"{"type":"thinking_delta","text":"hmm"}"#),
            AgentEvent::ThinkingDelta { text } if text == "hmm"
        ));
        assert!(matches!(
            one(r#"{"type":"tool_use_start","toolId":"t1","toolName":"Bash"}"#),
            AgentEvent::ToolUseStart { tool_id, tool_name } if tool_id == "t1" && tool_name == "Bash"
        ));
        assert!(matches!(
            one(r#"{"type":"tool_use_input","toolId":"t1","inputJson":"{\"command\":\"ls\"}"}"#),
            AgentEvent::ToolInputDelta { tool_id, input_json }
                if tool_id == "t1" && input_json == r#"{"command":"ls"}"#
        ));
        match one(r#"{"type":"tool_result","toolId":"t1","toolName":"","content":"ok","isError":false}"#) {
            AgentEvent::ToolResult { tool_id, tool_name, content, is_error } => {
                assert_eq!((tool_id.as_str(), tool_name.as_str(), content.as_str(), is_error), ("t1", "", "ok", false));
            }
            other => panic!("wrong event: {other:?}"),
        }
        match one(r#"{"type":"approval_request","requestId":"1","toolName":"Bash","input":{"command":"rm -rf /tmp/x"}}"#) {
            AgentEvent::ApprovalRequired { request_id, description } => {
                assert_eq!(request_id, "1");
                assert!(description.starts_with("Bash: ") && description.contains("rm -rf /tmp/x"));
            }
            other => panic!("wrong event: {other:?}"),
        }
        match one(r#"{"type":"ask_user_question","requestId":"2","questions":[{"question":"Which one?"}]}"#) {
            AgentEvent::AskUserQuestion { request_id, questions } => {
                assert_eq!(request_id, "2");
                assert!(questions.is_array());
                let q = &questions.as_array().unwrap()[0];
                assert_eq!(q["question"], "Which one?");
            }
            other => panic!("wrong event: {other:?}"),
        }
        assert!(matches!(
            one(r#"{"type":"turn_completed","sessionId":"abc-123"}"#),
            AgentEvent::TurnCompleted { turn_id } if turn_id == "abc-123"
        ));
        match one(r#"{"type":"usage","inputTokens":10,"outputTokens":20,"cacheRead":5,"cacheWrite":2,"costUsd":0.01,"model":"claude-haiku-4-5"}"#) {
            AgentEvent::UsageReport { input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, model } => {
                assert_eq!((input_tokens, output_tokens, cache_read_tokens, cache_write_tokens), (10, 20, 5, 2));
                assert!((cost_usd - 0.01).abs() < f64::EPSILON);
                assert_eq!(model, "claude-haiku-4-5");
            }
            other => panic!("wrong event: {other:?}"),
        }
        assert!(matches!(
            one(r#"{"type":"error","message":"boom"}"#),
            AgentEvent::SessionError { message } if message == "boom"
        ));
        assert!(matches!(
            one(r#"{"type":"session_resume_failed","claudeSessionId":"abc-123"}"#),
            AgentEvent::SessionResumeFailed { claude_session_id } if claude_session_id == "abc-123"
        ));
    }

    #[test]
    fn defaults_and_skips() {
        // Missing toolName defaults to "tool"; missing error message to "Unknown error".
        assert!(matches!(
            one(r#"{"type":"tool_use_start","toolId":"t9"}"#),
            AgentEvent::ToolUseStart { tool_name, .. } if tool_name == "tool"
        ));
        assert!(matches!(
            one(r#"{"type":"error"}"#),
            AgentEvent::SessionError { message } if message == "Unknown error"
        ));
        // Empty deltas, unknown types, non-JSON, non-objects and blanks all skip.
        assert!(parse_sidecar_line(r#"{"type":"text_delta","text":""}"#).is_empty());
        assert!(parse_sidecar_line(r#"{"type":"wat"}"#).is_empty());
        assert!(parse_sidecar_line("npm WARN deprecated something").is_empty());
        assert!(parse_sidecar_line("[1,2,3]").is_empty());
        assert!(parse_sidecar_line("   ").is_empty());
    }

    fn init_slot() -> Mutex<Option<SidecarInitParams>> {
        Mutex::new(Some(SidecarInitParams {
            cwd: "/repo".into(),
            model: Some("haiku".into()),
            permission_mode: Some("bypassPermissions".into()),
            mode: SessionMode::Resume { claude_session_id: "sid-1".into() },
        }))
    }

    #[test]
    fn first_query_gets_init_mode_then_follow_ups_continue() {
        let slot = init_slot();
        let first = augment_query_if_needed(r#"{"type":"query","prompt":"hi"}"#, &slot);
        let v: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(v["cwd"], "/repo");
        assert_eq!(v["model"], "haiku");
        assert_eq!(v["permissionMode"], "bypassPermissions");
        assert_eq!(v["mode"], "resume");
        assert_eq!(v["resumeSessionId"], "sid-1");

        // Init consumed → follow-up query continues the in-process conversation.
        let second = augment_query_if_needed(r#"{"type":"query","prompt":"again"}"#, &slot);
        let v: Value = serde_json::from_str(&second).unwrap();
        assert_eq!(v["prompt"], "again");
        assert_eq!(v["mode"], "continue");
        assert!(v.get("cwd").is_none() && v.get("model").is_none() && v.get("resumeSessionId").is_none());
    }

    #[test]
    fn fresh_mode_stamps_neither_resume_nor_continue_on_first() {
        let slot = Mutex::new(Some(SidecarInitParams {
            cwd: "/repo".into(),
            model: None,
            permission_mode: None,
            mode: SessionMode::Fresh,
        }));
        let first = augment_query_if_needed(r#"{"type":"query","prompt":"hi"}"#, &slot);
        let v: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(v["mode"], "fresh");
        assert!(v.get("resumeSessionId").is_none());
    }

    #[test]
    fn non_query_passes_through_without_consuming_params() {
        let slot = init_slot();
        assert_eq!(augment_query_if_needed(r#"{"type":"abort"}"#, &slot), r#"{"type":"abort"}"#);
        assert_eq!(augment_query_if_needed("not json", &slot), "not json");
        // Params still pending for the actual first query.
        let first = augment_query_if_needed(r#"{"type":"query","prompt":"hi"}"#, &slot);
        let v: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(v["cwd"], "/repo");
        assert_eq!(v["mode"], "resume");
    }

    #[test]
    fn explicit_query_fields_win_over_init_params() {
        let slot = init_slot();
        let out = augment_query_if_needed(r#"{"type":"query","prompt":"hi","model":"opus"}"#, &slot);
        let v: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(v["model"], "opus");
        assert_eq!(v["cwd"], "/repo");
    }
}
