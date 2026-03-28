use std::sync::Arc;

use serde::Serialize;
use tauri::Emitter;
use tokio::sync::mpsc;
use uuid::Uuid;

use codeforge_persistence::Database;
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
}

pub fn spawn_event_forwarder(
    app_handle: tauri::AppHandle,
    session_id: Uuid,
    thread_id: Uuid,
    mut rx: mpsc::UnboundedReceiver<AgentEvent>,
    db: Arc<std::sync::Mutex<Database>>,
) {
    let app = app_handle;
    let mut accumulated_content = String::new();
    let mut streaming_msg_id: Option<Uuid> = None;

    tokio::spawn(async move {
        while let Some(event) = rx.recv().await {
            let payload = match &event {
                AgentEvent::ContentDelta { text } => {
                    accumulated_content.push_str(text);
                    if streaming_msg_id.is_none() {
                        streaming_msg_id = Some(Uuid::new_v4());
                    }

                    AgentEventPayload {
                        session_id: session_id.to_string(),
                        thread_id: thread_id.to_string(),
                        event_type: "content_delta".into(),
                        text: Some(text.clone()),
                        ..default_payload()
                    }
                }
                AgentEvent::TurnStarted { turn_id } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "turn_started".into(),
                    turn_id: Some(turn_id.clone()),
                    ..default_payload()
                },
                AgentEvent::TurnCompleted { turn_id } => {
                    // Persist accumulated message
                    if let Some(msg_id) = streaming_msg_id.take() {
                        let content = std::mem::take(&mut accumulated_content);
                        if let Ok(db) = db.lock() {
                            let db_msg = codeforge_persistence::Message {
                                id: msg_id,
                                thread_id,
                                role: codeforge_persistence::MessageRole::Assistant,
                                content,
                                created_at: chrono::Utc::now(),
                            };
                            let _ = codeforge_persistence::queries::insert_message(
                                db.conn(),
                                &db_msg,
                            );
                        }
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
                AgentEvent::SessionReady => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "session_ready".into(),
                    ..default_payload()
                },
                AgentEvent::SessionError { message } => AgentEventPayload {
                    session_id: session_id.to_string(),
                    thread_id: thread_id.to_string(),
                    event_type: "session_error".into(),
                    message: Some(message.clone()),
                    ..default_payload()
                },
            };

            let _ = app.emit("agent-event", &payload);
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
    }
}
