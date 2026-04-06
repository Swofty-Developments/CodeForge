use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::mpsc;
use uuid::Uuid;

use codeforge_persistence::{Database, MessageId, SessionId, ThreadId};
use codeforge_session::AgentEvent;

#[derive(Debug, Clone, Serialize)]
pub struct AgentEventPayload {
    pub session_id: String,
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
}

pub fn spawn_event_forwarder(
    app_handle: tauri::AppHandle,
    session_id: Uuid,
    thread_id: codeforge_persistence::ThreadId,
    mut rx: mpsc::Receiver<AgentEvent>,
    db: Arc<std::sync::Mutex<Database>>,
) {
    let app = app_handle;
    let mut accumulated_content = String::new();
    let mut streaming_msg_id: Option<MessageId> = None;

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let payload = match &event {
                AgentEvent::ContentDelta { text } => {
                    accumulated_content.push_str(text);
                    if streaming_msg_id.is_none() {
                        streaming_msg_id = Some(MessageId::new());
                    }

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "content_delta".into(),
                        text: Some(text.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::TurnStarted { turn_id } => {
                    // Capture HEAD commit for per-turn diff and undo checkpoints
                    let db_clone = db.clone();
                    let tid_str = thread_id.to_string();
                    let turn_id_clone = turn_id.clone();
                    tokio::task::spawn_blocking(move || {
                        // Get worktree path from DB
                        if let Ok(db) = db_clone.lock() {
                            if let Ok(Some(wt)) = codeforge_persistence::queries::get_worktree_by_thread(
                                db.conn(),
                                tid_str.parse().unwrap(),
                            ) {
                                // Get HEAD commit in the worktree
                                if let Ok(output) = std::process::Command::new("git")
                                    .args(["rev-parse", "HEAD"])
                                    .current_dir(&wt.path)
                                    .output()
                                {
                                    if output.status.success() {
                                        let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
                                        let id = Uuid::new_v4().to_string();
                                        let now = chrono::Utc::now().to_rfc3339();
                                        let _ = codeforge_persistence::queries::insert_turn_checkpoint(
                                            db.conn(), &id, &tid_str, &turn_id_clone, &sha, &now,
                                        );
                                    }
                                }
                            }
                        }
                    });

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "turn_started".into(),
                        turn_id: Some(turn_id.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::TurnCompleted { turn_id } => {
                    // Persist accumulated message via spawn_blocking to avoid
                    // holding the std::sync::Mutex across an await point.
                    if let Some(msg_id) = streaming_msg_id.take() {
                        let content = std::mem::take(&mut accumulated_content);
                        let db_clone = db.clone();
                        let tid = thread_id;
                        tokio::task::spawn_blocking(move || {
                            if let Ok(db) = db_clone.lock() {
                                let db_msg = codeforge_persistence::Message {
                                    id: msg_id,
                                    thread_id: tid,
                                    role: codeforge_persistence::MessageRole::Assistant,
                                    content,
                                    created_at: chrono::Utc::now(),
                                };
                                if let Err(e) = codeforge_persistence::queries::insert_message(
                                    db.conn(),
                                    &db_msg,
                                ) {
                                    tracing::error!("Failed to insert message: {e}");
                                }
                            } else {
                                tracing::error!("Failed to lock database for message insert");
                            }
                        });
                    }

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "turn_completed".into(),
                        turn_id: Some(turn_id.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::TurnAborted { reason } => {
                    accumulated_content.clear();
                    streaming_msg_id = None;

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "turn_aborted".into(),
                        reason: Some(reason.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::ApprovalRequired {
                    request_id,
                    description,
                } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "approval_required".into(),
                    request_id: Some(request_id.clone()),
                    description: Some(description.clone()),
                    ..default_payload()
                },
                AgentEvent::UsageReport {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_write_tokens,
                    cost_usd,
                    ref model,
                } => {
                    // Persist usage via spawn_blocking
                    let db_clone = db.clone();
                    let tid_str = thread_id.to_string();
                    let sid_str = session_id.to_string();
                    let it = *input_tokens as i64;
                    let ot = *output_tokens as i64;
                    let crt = *cache_read_tokens as i64;
                    let cwt = *cache_write_tokens as i64;
                    let cu = *cost_usd;
                    let m = model.clone();
                    tokio::task::spawn_blocking(move || {
                        if let Ok(db) = db_clone.lock() {
                            let id = Uuid::new_v4().to_string();
                            let now = chrono::Utc::now().to_rfc3339();
                            if let Err(e) = codeforge_persistence::queries::insert_usage_log(
                                db.conn(),
                                &id,
                                &tid_str,
                                Some(&sid_str),
                                it, ot, crt, cwt, cu,
                                Some(&m),
                                &now,
                            ) {
                                tracing::error!("Failed to insert usage log: {e}");
                            }
                        } else {
                            tracing::error!("Failed to lock database for usage log insert");
                        }
                    });

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "usage_report".into(),
                        input_tokens: Some(*input_tokens),
                        output_tokens: Some(*output_tokens),
                        cost_usd: Some(*cost_usd),
                        model: Some(model.clone()),
                        cache_read_tokens: Some(*cache_read_tokens),
                        cache_write_tokens: Some(*cache_write_tokens),
                        ..default_payload()
                    }
                }
                AgentEvent::SessionReady { claude_session_id, ref model } => {
                    // Persist session record via spawn_blocking
                    let db_clone = db.clone();
                    let csid = claude_session_id.clone();
                    let sid: SessionId = SessionId::from(session_id);
                    let tid: ThreadId = thread_id;
                    tokio::task::spawn_blocking(move || {
                        if let Ok(db) = db_clone.lock() {
                            let db_session = codeforge_persistence::Session {
                                id: sid,
                                thread_id: tid,
                                provider: codeforge_persistence::Provider::Claude,
                                status: "ready".to_string(),
                                approval_mode: None,
                                pid: None,
                                claude_session_id: csid.clone(),
                                created_at: chrono::Utc::now(),
                            };
                            if let Err(e) = codeforge_persistence::queries::insert_session(
                                db.conn(),
                                &db_session,
                            ) {
                                tracing::error!("Failed to insert session: {e}");
                            }
                            if let Some(ref csid) = csid {
                                if let Err(e) = codeforge_persistence::queries::update_session_claude_id(
                                    db.conn(),
                                    sid,
                                    csid,
                                ) {
                                    tracing::error!("Failed to update session claude_id: {e}");
                                }
                            }
                        } else {
                            tracing::error!("Failed to lock database for session insert");
                        }
                    });

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "session_ready".into(),
                        model: model.clone(),
                        ..default_payload()
                    }
                }
                AgentEvent::SessionError { message } => {
                    accumulated_content.clear();
                    streaming_msg_id = None;

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "session_error".into(),
                        message: Some(message.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::ToolUseStart { tool_id, tool_name } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "tool_use_start".into(),
                    tool_id: Some(tool_id.clone()),
                    tool_name: Some(tool_name.clone()),
                    ..default_payload()
                },
                AgentEvent::ToolInputDelta { tool_id, input_json } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "tool_input_delta".into(),
                    tool_id: Some(tool_id.clone()),
                    input_json: Some(input_json.clone()),
                    ..default_payload()
                },
                AgentEvent::ToolUseEnd { tool_id } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "tool_use_end".into(),
                    tool_id: Some(tool_id.clone()),
                    ..default_payload()
                },
                AgentEvent::ToolResult { tool_id, tool_name, content, is_error } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "tool_result".into(),
                    tool_id: Some(tool_id.clone()),
                    tool_name: Some(tool_name.clone()),
                    tool_output: Some(content.clone()),
                    is_error: Some(*is_error),
                    ..default_payload()
                },
                AgentEvent::ThinkingDelta { text } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "thinking_delta".into(),
                    text: Some(text.clone()),
                    ..default_payload()
                },
            };

            tracing::debug!("Emitting agent-event: {} (text: {:?})", payload.event_type, payload.text.as_deref().map(|t| {
                let end = t.char_indices().nth(40).map(|(i, _)| i).unwrap_or(t.len());
                &t[..end]
            }));
            if let Err(e) = app.emit("agent-event", &payload) {
                tracing::error!("Failed to emit agent-event: {e}");
            }
        }
    });
}

fn default_payload() -> AgentEventPayload {
    AgentEventPayload {
        session_id: String::new(),
        thread_id: String::new(),
        event_type: String::new(),
        text: None,
        turn_id: None,
        reason: None,
        message: None,
        request_id: None,
        description: None,
        input_tokens: None,
        output_tokens: None,
        cost_usd: None,
        model: None,
        tool_id: None,
        tool_name: None,
        input_json: None,
        tool_output: None,
        is_error: None,
        cache_read_tokens: None,
        cache_write_tokens: None,
    }
}
