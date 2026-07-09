//! Forward a session's [`AgentEvent`] stream to the frontend `agent-event`
//! channel and persist durable side-effects (assistant text, usage, resume id).
//!
//! All DB writes go through `spawn_blocking`, so the app-DB std mutex is never
//! held across an `.await` in this task.

use std::sync::{Arc, Mutex};

use forge_session::{AgentEvent, AgentEventPayload};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::db::Database;
use crate::{events, queries};

/// Spawn the per-session forwarder.
///
/// - `session_id`: session-manager uuid (IPC id, and `sessions.id`).
/// - `app_thread_id`: local `threads.id` row; `None` disables persistence
///   (e.g. the repo row was missing), events still stream.
/// - `sdk_thread_hint`: resume id, if any — seeds the payload `threadId` until
///   `session_ready` confirms the real one.
pub fn spawn_forwarder(
    app: AppHandle,
    session_id: String,
    app_thread_id: Option<String>,
    sdk_thread_hint: Option<String>,
    mut rx: mpsc::Receiver<AgentEvent>,
    db: Arc<Mutex<Database>>,
) {
    tokio::spawn(async move {
        let mut sdk_thread = sdk_thread_hint.unwrap_or_default();
        let mut buffer = String::new();

        while let Some(event) = rx.recv().await {
            match &event {
                AgentEvent::ContentDelta { text } => buffer.push_str(text),
                AgentEvent::SessionReady { claude_session_id: Some(cid), .. } => {
                    sdk_thread = cid.clone();
                    if app_thread_id.is_some() {
                        persist_claude_id(&db, &session_id, cid);
                    }
                }
                AgentEvent::TurnCompleted { .. } => {
                    let content = std::mem::take(&mut buffer);
                    if let (Some(tid), false) = (&app_thread_id, content.trim().is_empty()) {
                        persist_message(&db, tid, "assistant", content);
                    }
                }
                AgentEvent::TurnAborted { .. } | AgentEvent::SessionError { .. } => buffer.clear(),
                AgentEvent::UsageReport {
                    input_tokens,
                    output_tokens,
                    cache_read_tokens,
                    cache_write_tokens,
                    cost_usd,
                    model,
                } => {
                    if let Some(tid) = &app_thread_id {
                        persist_usage(
                            &db,
                            Usage {
                                thread_id: tid.clone(),
                                session_id: session_id.clone(),
                                input_tokens: *input_tokens,
                                output_tokens: *output_tokens,
                                cache_read_tokens: *cache_read_tokens,
                                cache_write_tokens: *cache_write_tokens,
                                cost_usd: *cost_usd,
                                model: model.clone(),
                            },
                        );
                    }
                }
                _ => {}
            }

            let payload = AgentEventPayload::from_event(&session_id, &sdk_thread, &event);
            let _ = app.emit(events::AGENT_EVENT, &payload);
        }
        tracing::debug!(session = %session_id, "session forwarder ended");
    });
}

struct Usage {
    thread_id: String,
    session_id: String,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cost_usd: f64,
    model: String,
}

fn persist_message(db: &Arc<Mutex<Database>>, thread_id: &str, role: &str, content: String) {
    let db = db.clone();
    let thread_id = thread_id.to_string();
    let role = role.to_string();
    tokio::task::spawn_blocking(move || {
        with_conn(&db, |conn| {
            queries::insert_message(conn, &uuid::Uuid::new_v4().to_string(), &thread_id, &role, &content)
        });
    });
}

fn persist_claude_id(db: &Arc<Mutex<Database>>, session_id: &str, claude_id: &str) {
    let db = db.clone();
    let session_id = session_id.to_string();
    let claude_id = claude_id.to_string();
    tokio::task::spawn_blocking(move || {
        with_conn(&db, |conn| queries::update_session_claude_id(conn, &session_id, &claude_id));
    });
}

fn persist_usage(db: &Arc<Mutex<Database>>, u: Usage) {
    let db = db.clone();
    tokio::task::spawn_blocking(move || {
        with_conn(&db, |conn| {
            queries::insert_usage_log(
                conn,
                &u.thread_id,
                Some(&u.session_id),
                u.input_tokens,
                u.output_tokens,
                u.cache_read_tokens,
                u.cache_write_tokens,
                u.cost_usd,
                Some(&u.model),
            )
        });
    });
}

fn with_conn(db: &Arc<Mutex<Database>>, f: impl FnOnce(&rusqlite::Connection) -> anyhow::Result<()>) {
    match db.lock() {
        Ok(db) => {
            if let Err(e) = f(db.conn()) {
                tracing::warn!("session db write failed: {e}");
            }
        }
        Err(_) => tracing::warn!("app db mutex poisoned; skipping session db write"),
    }
}
