use std::path::Path;
use std::sync::{Arc, Mutex};

use forge_core::{SessionInfo, StartSessionOpts};
use forge_session::AgentEvent;
use tauri::State;
use tokio::sync::mpsc;

use crate::db::Database;
use crate::runtime::{repo_util, session_forward};
use crate::state::AppState;
use crate::queries;

/// Start (or resume, when `opts.resume_session_id` is set) an embedded Claude
/// Code session with cwd = repo root, and spawn the event forwarder that re-emits
/// `AgentEvent`s on the `agent-event` channel (payload: `AgentEventPayload`).
#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    opts: StartSessionOpts,
) -> Result<SessionInfo, String> {
    let repo_key = repo_util::canonical(&opts.repo_path)?;
    let sdk_hint = opts.resume_session_id.clone();

    let (tx, rx) = mpsc::channel::<AgentEvent>(1024);
    let info = {
        let mut mgr = state.sessions.lock().await;
        mgr.start_session(opts, tx).await.map_err(|e| format!("{e:#}"))?
    };

    state.session_repos.lock().await.insert(info.id.clone(), repo_key.clone());

    // Best-effort persistence root: a thread+session row for this conversation.
    let app_thread_id = create_thread_and_session(&state.db, &repo_key, &info).await;

    session_forward::spawn_forwarder(
        app,
        info.id.clone(),
        app_thread_id,
        info.thread_id.clone().or(sdk_hint),
        rx,
        state.db.clone(),
    );

    Ok(info)
}

/// Queue a user prompt on a running session.
#[tauri::command]
pub async fn send_session_input(state: State<'_, AppState>, id: String, text: String) -> Result<(), String> {
    state
        .sessions
        .lock()
        .await
        .send(&id, &text)
        .map_err(|e| format!("{e}"))
}

/// Answer a pending `approval_required` event.
#[tauri::command]
pub async fn approve_session(
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    approve: bool,
) -> Result<(), String> {
    state
        .sessions
        .lock()
        .await
        .approve(&id, &request_id, approve)
        .map_err(|e| format!("{e}"))
}

/// Stop a session (kills its sidecar) and forget its repo mapping.
#[tauri::command]
pub async fn stop_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    {
        let mut mgr = state.sessions.lock().await;
        mgr.stop(&id).await.map_err(|e| format!("{e}"))?;
    }
    state.session_repos.lock().await.remove(&id);
    Ok(())
}

/// Snapshot of all live sessions (session pane).
#[tauri::command]
pub async fn list_sessions(state: State<'_, AppState>) -> Result<Vec<SessionInfo>, String> {
    Ok(state.sessions.lock().await.list())
}

/// Create the DB thread + session rows for a session. Returns the thread id, or
/// `None` when persistence can't proceed (e.g. the repo row is missing) — the
/// session still runs, only its history isn't stored.
async fn create_thread_and_session(
    db: &Arc<Mutex<Database>>,
    repo_key: &Path,
    info: &SessionInfo,
) -> Option<String> {
    let db = db.clone();
    let path_s = repo_key.to_string_lossy().into_owned();
    let title = info.title.clone();
    let session_id = info.id.clone();
    let model = info.model.clone();
    let thread_id = uuid::Uuid::new_v4().to_string();
    let tid = thread_id.clone();

    let joined = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        let conn = db.conn();
        let repo_id = queries::get_repo_id_by_path(conn, &path_s)?
            .ok_or_else(|| anyhow::anyhow!("repo row not found for {path_s}"))?;
        queries::insert_thread(conn, &tid, &repo_id, &title)?;
        queries::insert_session(conn, &session_id, &tid, "ready", model.as_deref())?;
        Ok(())
    })
    .await;

    match joined {
        Ok(Ok(())) => Some(thread_id),
        Ok(Err(e)) => {
            tracing::warn!("session persistence skipped: {e}");
            None
        }
        Err(e) => {
            tracing::warn!("session persistence task join error: {e}");
            None
        }
    }
}
