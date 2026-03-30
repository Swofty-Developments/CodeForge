use std::path::PathBuf;

use tauri::{AppHandle, State};
use uuid::Uuid;

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
) -> Result<(), String> {
    let tid = Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;

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
            _ => Provider::ClaudeCode,
        };
        let cwd_path = PathBuf::from(&cwd);

        let mut mgr = state.session_manager.lock().await;
        let (session_id, event_rx) = mgr
            .create_session(prov, &cwd_path, model.as_deref())
            .await
            .map_err(|e| format!("{e:#}"))?;

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
pub async fn stop_session(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<(), String> {
    let tid = Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;
    let session_id = {
        let mut sessions = state.thread_sessions.lock().await;
        sessions.remove(&tid)
    };
    if let Some(session_id) = session_id {
        let mut mgr = state.session_manager.lock().await;
        let _ = mgr.stop_session(session_id).await;
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
    let sid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    let mgr = state.session_manager.lock().await;
    mgr.respond_to_approval(sid, &request_id, approve)
        .map_err(|e| format!("{e:#}"))
}
