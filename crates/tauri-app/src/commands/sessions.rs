use forge_core::{SessionInfo, StartSessionOpts};
use tauri::State;

use crate::state::AppState;

/// Start (or resume, when `opts.resume_session_id` is set) an embedded Claude
/// Code session with cwd = repo root, and spawn the event forwarder that re-emits
/// `AgentEvent`s on the `agent-event` channel (payload: `AgentEventPayload`).
#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    opts: StartSessionOpts,
) -> Result<SessionInfo, String> {
    let _ = (app, state, opts);
    // IMPLEMENT(agent): mpsc::channel(1024); sessions.lock().await.start_session
    // (opts, tx); record session_repos mapping; spawn forwarder task mapping
    // AgentEvent -> AgentEventPayload::from_event -> app.emit(events::AGENT_EVENT)
    // with DB side-effects via spawn_blocking (usage logs, claude_session_id).
    Err("not implemented: start_session".into())
}

/// Queue a user prompt on a running session.
#[tauri::command]
pub async fn send_session_input(
    state: State<'_, AppState>,
    id: String,
    text: String,
) -> Result<(), String> {
    let _ = (state, id, text);
    // IMPLEMENT(agent): state.sessions.lock().await.send(&id, &text)
    Err("not implemented: send_session_input".into())
}

/// Answer a pending `approval_required` event.
#[tauri::command]
pub async fn approve_session(
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    approve: bool,
) -> Result<(), String> {
    let _ = (state, id, request_id, approve);
    // IMPLEMENT(agent): state.sessions.lock().await.approve(&id, &request_id, approve)
    Err("not implemented: approve_session".into())
}

/// Stop a session (kills its sidecar) and emit a final `session:{id}:status`.
#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    let _ = (state, id);
    // IMPLEMENT(agent): sessions.lock().await.stop(&id); clean session_repos.
    Err("not implemented: stop_session".into())
}

/// Snapshot of all live sessions (session pane).
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
    let _ = state;
    // IMPLEMENT(agent): state.sessions.lock().await.list()
    Err("not implemented: list_sessions".into())
}
