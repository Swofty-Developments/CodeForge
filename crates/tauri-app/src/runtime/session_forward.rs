//! Forward a session's [`AgentEvent`] stream to the frontend `agent-event`
//! channel and persist durable side-effects (assistant text, usage, resume id).
//!
//! Every emitted payload is stamped with the forge `sessionId` (CONTRACT-1) so
//! the frontend demuxes by one id space; `threadId` carries the Claude session
//! uuid for display/resume only. Persistence is UNCONDITIONAL (CONTRACT-2: the
//! thread row exists by construction); a write FAILURE is a named, surfaced
//! state — a `session_persistence_degraded` agent-event plus an error log — not
//! a swallowed warning. All DB writes go through `spawn_blocking`, so the app-DB
//! std mutex is never held across an `.await`.

use std::sync::{Arc, Mutex};

use forge_session::{AgentEvent, AgentEventPayload};
use tauri::{AppHandle, Emitter};
use tokio::sync::mpsc;

use crate::db::Database;
use crate::{events, queries};

/// Spawn the per-session forwarder.
///
/// - `session_id`: session-manager uuid (IPC id, and `sessions.id`) — the
///   CONTRACT-1 routing key stamped on every payload.
/// - `app_thread_id`: local `threads.id` row (non-optional — the session has a
///   thread row by construction).
/// - `sdk_thread_hint`: resume id, if any — seeds the payload `threadId` until
///   `session_ready` confirms the real one.
pub fn spawn_forwarder(
    app: AppHandle,
    session_id: String,
    app_thread_id: String,
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
                    if let Err(e) = persist_claude_id(&db, &session_id, cid).await {
                        report_degraded(&app, &session_id, &sdk_thread, "record resume id", &e);
                    }
                }
                AgentEvent::TurnCompleted { .. } => {
                    let content = std::mem::take(&mut buffer);
                    if !content.trim().is_empty() {
                        if let Err(e) = persist_message(&db, &app_thread_id, "assistant", content).await {
                            report_degraded(&app, &session_id, &sdk_thread, "store assistant message", &e);
                        }
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
                    let usage = Usage {
                        thread_id: app_thread_id.clone(),
                        session_id: session_id.clone(),
                        input_tokens: *input_tokens,
                        output_tokens: *output_tokens,
                        cache_read_tokens: *cache_read_tokens,
                        cache_write_tokens: *cache_write_tokens,
                        cost_usd: *cost_usd,
                        model: model.clone(),
                    };
                    if let Err(e) = persist_usage(&db, usage).await {
                        report_degraded(&app, &session_id, &sdk_thread, "record usage", &e);
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

/// Surface a persistence failure as a named state: an error log plus a
/// `session_persistence_degraded` event so the UI can show the session's stored
/// history/usage may be incomplete. Never swallowed into a benign warn.
fn report_degraded(app: &AppHandle, session_id: &str, sdk_thread: &str, action: &str, err: &anyhow::Error) {
    tracing::error!(session = %session_id, "session persistence failed to {action}: {err:#}");
    let payload = AgentEventPayload::persistence_degraded(
        session_id,
        sdk_thread,
        &format!("could not {action}: {err}"),
    );
    let _ = app.emit(events::AGENT_EVENT, &payload);
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

async fn persist_message(
    db: &Arc<Mutex<Database>>,
    thread_id: &str,
    role: &'static str,
    content: String,
) -> anyhow::Result<()> {
    let db = db.clone();
    let thread_id = thread_id.to_string();
    run_write(db, move |conn| {
        queries::insert_message(conn, &uuid::Uuid::new_v4().to_string(), &thread_id, role, &content)
    })
    .await
}

async fn persist_claude_id(db: &Arc<Mutex<Database>>, session_id: &str, claude_id: &str) -> anyhow::Result<()> {
    let db = db.clone();
    let session_id = session_id.to_string();
    let claude_id = claude_id.to_string();
    run_write(db, move |conn| queries::update_session_claude_id(conn, &session_id, &claude_id)).await
}

async fn persist_usage(db: &Arc<Mutex<Database>>, u: Usage) -> anyhow::Result<()> {
    let db = db.clone();
    run_write(db, move |conn| {
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
    })
    .await
}

/// Run one DB write on the blocking pool, propagating both the join and the
/// query error so the caller can surface a degraded state.
async fn run_write(
    db: Arc<Mutex<Database>>,
    f: impl FnOnce(&rusqlite::Connection) -> anyhow::Result<()> + Send + 'static,
) -> anyhow::Result<()> {
    tokio::task::spawn_blocking(move || {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        f(db.conn())
    })
    .await?
}
