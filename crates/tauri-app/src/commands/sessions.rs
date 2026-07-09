use std::sync::{Arc, Mutex};

use forge_core::{SessionInfo, StartSessionOpts};
use forge_session::{AgentEvent, SessionMode};
use tauri::State;
use tokio::sync::mpsc;

use crate::db::Database;
use crate::runtime::{repo_util, session_forward};
use crate::state::AppState;
use crate::queries;

/// Start (or resume, when `opts.resume_session_id` names a recorded SDK session)
/// an embedded Claude Code session with cwd = repo root, and spawn the event
/// forwarder that re-emits `AgentEvent`s on the `agent-event` channel.
///
/// Precondition (CONTRACT-2): the repo must be OPEN — otherwise there is no
/// repo row to anchor persistence, so this returns `Err("repo is not open")`.
#[tauri::command]
pub async fn start_session(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    opts: StartSessionOpts,
) -> Result<SessionInfo, String> {
    let repo_key = repo_util::canonical(&opts.repo_path)?;

    // CONTRACT-2 precondition: the repo must be open (mirrors get_features). The
    // open repo's row id anchors the thread — every started session has a repo
    // + thread row by construction.
    let repo_id = {
        let repos = state.repos.lock().await;
        repos.get(&repo_key).map(|rt| rt.repo_id.clone()).ok_or("repo is not open")?
    };

    // Fix 1: the engagement mode is decided here, from the DB — not discovered
    // by a resume→fresh cascade. A resume target must be a recorded SDK session.
    let mode = resolve_session_mode(&state.db, opts.resume_session_id.clone()).await?;
    let sdk_hint = match &mode {
        SessionMode::Resume { claude_session_id } => Some(claude_session_id.clone()),
        _ => None,
    };

    let (tx, rx) = mpsc::channel::<AgentEvent>(1024);
    let info = {
        let mut mgr = state.sessions.lock().await;
        mgr.start_session(opts, mode, tx).await.map_err(|e| format!("{e:#}"))?
    };

    // CONTRACT-2: a started session has a thread row by construction. If the
    // rows can't be written, tear the session down rather than run it headless.
    let app_thread_id = match create_thread_and_session(&state.db, &repo_id, &info).await {
        Ok(tid) => tid,
        Err(e) => {
            let _ = state.sessions.lock().await.stop(&info.id).await;
            return Err(format!("failed to persist session rows: {e:#}"));
        }
    };

    state.session_repos.lock().await.insert(info.id.clone(), repo_key.clone());

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

/// Decide the DB-authoritative [`SessionMode`]. `None` resume id → Fresh. A
/// given id must be a recorded `sessions.claude_session_id`; an unrecorded id is
/// a named error (surfaced), never a silent fresh start.
async fn resolve_session_mode(
    db: &Arc<Mutex<Database>>,
    resume_session_id: Option<String>,
) -> Result<SessionMode, String> {
    let Some(claude_id) = resume_session_id else {
        return Ok(SessionMode::Fresh);
    };
    if claude_session_exists(db, &claude_id).await? {
        Ok(SessionMode::Resume { claude_session_id: claude_id })
    } else {
        Err(format!("cannot resume: no recorded Claude session {claude_id}"))
    }
}

async fn claude_session_exists(db: &Arc<Mutex<Database>>, claude_id: &str) -> Result<bool, String> {
    let db = db.clone();
    let claude_id = claude_id.to_string();
    tokio::task::spawn_blocking(move || -> anyhow::Result<bool> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        queries::claude_session_exists(db.conn(), &claude_id)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
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

/// Create the DB thread + session rows for a session, returning the new thread
/// id. The repo is open (CONTRACT-2), so its row id is known and passed in; a
/// failure here is real (surfaced by the caller), never swallowed.
async fn create_thread_and_session(
    db: &Arc<Mutex<Database>>,
    repo_id: &str,
    info: &SessionInfo,
) -> anyhow::Result<String> {
    let db = db.clone();
    let repo_id = repo_id.to_string();
    let title = info.title.clone();
    let session_id = info.id.clone();
    let model = info.model.clone();
    let thread_id = uuid::Uuid::new_v4().to_string();
    let tid = thread_id.clone();

    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        let conn = db.conn();
        queries::insert_thread(conn, &tid, &repo_id, &title)?;
        queries::insert_session(conn, &session_id, &tid, "ready", model.as_deref())?;
        Ok(())
    })
    .await??;

    Ok(thread_id)
}
