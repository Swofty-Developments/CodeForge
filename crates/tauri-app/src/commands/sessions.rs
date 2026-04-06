use std::path::PathBuf;

use tauri::{AppHandle, State};

use codeforge_persistence::ThreadId;
use codeforge_session::Provider;

use crate::state::TauriState;
use crate::streaming;

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    state: State<'_, TauriState>,
    thread_id: String,
    text: String,
    provider: String,
    cwd: String,
    model: Option<String>,
    permission_mode: Option<String>,
) -> Result<(), String> {
    tracing::debug!("send_message thread_id={thread_id} provider={provider}");
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;

    // Check if session exists, then release lock
    let existing_session = {
        let sessions = state.thread_sessions.lock().await;
        sessions.get(&tid).copied()
    };

    if let Some(session_id) = existing_session {
        let mut mgr = state.session_manager.lock().await;
        mgr.send_message(session_id, &text)
            .await
            .map_err(|e| format!("{e:#}"))?;
    } else {
        let prov = match provider.as_str() {
            "codex" => Provider::Codex,
            "claude" | "claude_code" => Provider::ClaudeCode,
            other => return Err(format!("Unknown provider: {other}")),
        };
        let cwd_path = PathBuf::from(&cwd);

        // Batch DB reads: permission_mode + claude_session_id in one lock
        let (perm_mode, previous_claude_session_id) = {
            let db = state.db.lock().map_err(|e| {
                tracing::error!("Failed to lock database: {e}");
                e.to_string()
            })?;
            let pm = permission_mode.or_else(|| {
                codeforge_persistence::queries::get_setting(db.conn(), "permission_mode")
                    .ok()
                    .flatten()
            });
            let csid = if prov == Provider::ClaudeCode {
                codeforge_persistence::queries::get_latest_claude_session_id(db.conn(), tid)
                    .unwrap_or(None)
            } else {
                None
            };
            (pm, csid)
        };

        let mut mgr = state.session_manager.lock().await;

        let (session_id, event_rx) = if let Some(ref claude_sid) = previous_claude_session_id {
            match mgr
                .resume_session(claude_sid, &cwd_path, model.as_deref())
                .await
            {
                Ok(result) => result,
                Err(e) => {
                    tracing::warn!("Failed to resume session, creating new one: {e:#}");
                    mgr.create_session(prov, &cwd_path, model.as_deref(), perm_mode.as_deref())
                        .await
                        .map_err(|e| format!("{e:#}"))?
                }
            }
        } else {
            mgr.create_session(prov, &cwd_path, model.as_deref(), perm_mode.as_deref())
                .await
                .map_err(|e| format!("{e:#}"))?
        };

        // Store session mapping
        {
            let mut sessions = state.thread_sessions.lock().await;
            sessions.insert(tid, session_id);
        }

        streaming::spawn_event_forwarder(
            app.clone(),
            session_id,
            tid,
            event_rx,
            state.db.clone(),
        );

        mgr.send_message(session_id, &text)
            .await
            .map_err(|e| format!("{e:#}"))?;
    }

    Ok(())
}

#[tauri::command]
pub async fn interrupt_session(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<(), String> {
    tracing::debug!("interrupt_session thread_id={thread_id}");
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    let session_id = {
        let sessions = state.thread_sessions.lock().await;
        sessions.get(&tid).copied()
    };
    if let Some(session_id) = session_id {
        let mgr = state.session_manager.lock().await;
        mgr.interrupt_session(session_id)
            .map_err(|e| format!("{e:#}"))?;
    } else {
        tracing::debug!("No active session found for thread {tid}");
    }
    Ok(())
}

#[tauri::command]
pub async fn stop_session(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<(), String> {
    tracing::debug!("stop_session thread_id={thread_id}");
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    let session_id = {
        let mut sessions = state.thread_sessions.lock().await;
        sessions.remove(&tid)
    };
    if let Some(session_id) = session_id {
        let mut mgr = state.session_manager.lock().await;
        if let Err(e) = mgr.stop_session(session_id).await {
            tracing::error!("Failed to stop session {session_id}: {e:#}");
        }
    } else {
        tracing::debug!("No active session found for thread {tid}");
    }
    Ok(())
}

#[tauri::command]
pub async fn respond_to_approval(
    state: State<'_, TauriState>,
    session_id: String,
    request_id: String,
    approve: bool,
) -> Result<(), String> {
    let sid = session_id.parse::<uuid::Uuid>().map_err(|e| e.to_string())?;
    let mgr = state.session_manager.lock().await;
    mgr.respond_to_approval(sid, &request_id, approve)
        .map_err(|e| format!("{e:#}"))
}
