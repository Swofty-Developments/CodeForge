use std::sync::{Arc, Mutex};

use forge_core::{SessionInfo, StartSessionOpts};
use forge_session::{AgentEvent, SessionManager, SessionMode};
use rusqlite::{params, OptionalExtension};
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

/// Queue a user prompt on a running session. On the first message, auto-generates
/// a title from the input text and updates both the DB and the live session.
#[tauri::command]
pub async fn send_session_input(state: State<'_, AppState>, id: String, text: String) -> Result<(), String> {
    // Check if this is the first message (thread_id exists but title is still "Session N").
    let should_auto_name = {
        let mgr = state.sessions.lock().await;
        if let Some(info) = mgr.list().iter().find(|s| s.id == id) {
            info.title.starts_with("Session ")
        } else {
            false
        }
    };

    // Send the message first.
    state
        .sessions
        .lock()
        .await
        .send(&id, &text)
        .map_err(|e| format!("{e}"))?;

    // Auto-rename the thread based on the first message.
    if should_auto_name {
        let auto_title = generate_title_from_text(&text);
        // Rename synchronously (it's fast, just a DB write + in-memory update).
        if let Err(e) = do_rename_session(state.inner(), &id, &auto_title).await {
            tracing::warn!("auto-rename failed: {e}");
        }
    }

    Ok(())
}

/// Answer a pending `approval_required` or `ask_user_question` event.
/// `answers` is present only for AskUserQuestion (question text → chosen label).
#[tauri::command]
pub async fn approve_session(
    state: State<'_, AppState>,
    id: String,
    request_id: String,
    approve: bool,
    answers: Option<serde_json::Value>,
) -> Result<(), String> {
    state
        .sessions
        .lock()
        .await
        .approve(&id, &request_id, approve, answers)
        .map_err(|e| format!("{e}"))
}

/// Abort the in-flight turn only — the session (and its transcript) stays
/// alive. This is what the composer's stop button calls; `stop_session` below
/// is the destructive close-tab path.
#[tauri::command]
pub async fn interrupt_session(state: State<'_, AppState>, id: String) -> Result<(), String> {
    state
        .sessions
        .lock()
        .await
        .abort(&id)
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

/// Switch a running session's permission mode mid-session (contract W5). `mode`
/// must be one of default|acceptEdits|plan|bypassPermissions — an unknown value
/// is a named error (validated in `SessionManager::set_mode`), never a silent
/// no-op. The sidecar applies the new mode on its next query turn.
#[tauri::command]
pub async fn set_session_mode(
    state: State<'_, AppState>,
    session_id: String,
    mode: String,
) -> Result<(), String> {
    state
        .sessions
        .lock()
        .await
        .set_mode(&session_id, &mode)
        .map_err(|e| format!("{e}"))
}

/// List past sessions for a repo (resumable sessions with a claude_session_id).
/// Excludes sessions currently running in the SessionManager — the DB doesn't
/// track running state, so the frontend filters live sessions by id comparison.
#[tauri::command]
pub async fn list_past_sessions(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<Vec<PastSessionDto>, String> {
    let repo_key = repo_util::canonical(&repo_path)?;
    let repo_id = {
        let repos = state.repos.lock().await;
        repos
            .get(&repo_key)
            .map(|rt| rt.repo_id.clone())
            .ok_or("repo is not open")?
    };

    let db = state.db.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<Vec<PastSessionDto>> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        let sessions = queries::list_past_sessions(db.conn(), &repo_id)?;
        Ok(sessions
            .into_iter()
            .map(|s| PastSessionDto {
                id: s.id,
                thread_id: s.thread_id,
                claude_session_id: s.claude_session_id,
                title: s.title,
                model: s.model,
                created_at: s.created_at,
                updated_at: s.updated_at,
            })
            .collect())
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))
}

/// Internal rename implementation.
async fn do_rename_impl(
    db: &Arc<Mutex<Database>>,
    sessions: &tokio::sync::Mutex<SessionManager>,
    session_id: &str,
    title: &str,
) -> Result<(), String> {
    if title.trim().is_empty() {
        return Err("title cannot be empty".to_string());
    }

    // Find the thread_id for this session — check live sessions first, then DB.
    let thread_id = {
        let mgr = sessions.lock().await;
        if let Some(info) = mgr.list().iter().find(|s| s.id == session_id) {
            info.thread_id.clone()
        } else {
            None
        }
    };

    let thread_id = if let Some(tid) = thread_id {
        tid
    } else {
        // Not a live session — query the DB for its thread_id.
        let db = db.clone();
        let sid = session_id.to_string();
        tokio::task::spawn_blocking(move || -> anyhow::Result<Option<String>> {
            let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
            db.conn()
                .query_row(
                    "SELECT thread_id FROM sessions WHERE id = ?1",
                    params![sid],
                    |r| r.get::<_, String>(0),
                )
                .optional()
                .map_err(Into::into)
        })
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e:#}"))?
        .ok_or("session not found")?
    };

    // Update the thread title in the DB.
    let db = db.clone();
    let new_title = title.to_string();
    let tid = thread_id.clone();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        queries::update_thread_title(db.conn(), &tid, &new_title)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| format!("{e:#}"))?;

    // Update the live session's title in the SessionManager if it's running.
    let sid = session_id.to_string();
    let new_title2 = title.to_string();
    let mut mgr = sessions.lock().await;
    let _ = mgr.update_title(&sid, new_title2);

    Ok(())
}

/// Internal helper for rename_session that works with &AppState (spawn-safe).
async fn do_rename_session(
    state: &AppState,
    session_id: &str,
    title: &str,
) -> Result<(), String> {
    do_rename_impl(&state.db, &state.sessions, session_id, title).await
}

/// Rename a session by updating its thread title (Tauri command wrapper).
#[tauri::command]
pub async fn rename_session(
    state: State<'_, AppState>,
    session_id: String,
    title: String,
) -> Result<(), String> {
    do_rename_session(&state, &session_id, &title).await
}

/// DTO for past sessions sent over the IPC wire.
#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PastSessionDto {
    pub id: String,
    pub thread_id: String,
    pub claude_session_id: Option<String>,
    pub title: String,
    pub model: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Generate a short, readable title from user input text. Takes the first
/// sentence or first ~60 chars, strips code fences and excessive whitespace.
fn generate_title_from_text(text: &str) -> String {
    let cleaned = text
        .lines()
        .filter(|line| !line.trim().starts_with("```")) // strip code fences
        .collect::<Vec<_>>()
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Take first sentence (up to . ! ?) or first 60 chars, whichever comes first.
    let first_sentence = cleaned
        .split_once(". ")
        .or_else(|| cleaned.split_once("! "))
        .or_else(|| cleaned.split_once("? "))
        .map(|(first, _)| first)
        .unwrap_or(&cleaned);

    let title = if first_sentence.len() > 60 {
        format!("{}...", &first_sentence[..60].trim())
    } else {
        first_sentence.to_string()
    };

    if title.is_empty() {
        "New chat".to_string()
    } else {
        title
    }
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

/// Persist a pasted clipboard image to a temp file and return its path, so the
/// composer can attach it as an `@<path>` reference like any picked file
/// (attachments pass paths only; the agent's own tools read the bytes).
#[tauri::command]
pub async fn save_pasted_image(data_base64: String, mime: String) -> Result<String, String> {
    use base64::Engine as _;

    let ext = match mime.as_str() {
        "image/png" => "png",
        "image/jpeg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        other => return Err(format!("unsupported pasted image type: {other}")),
    };
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(data_base64.as_bytes())
        .map_err(|e| format!("invalid pasted image data: {e}"))?;

    let dir = std::env::temp_dir().join("codeforge-pasted");
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("create paste dir: {e}"))?;
    let path = dir.join(format!("paste-{}.{ext}", uuid::Uuid::new_v4()));
    tokio::fs::write(&path, &bytes)
        .await
        .map_err(|e| format!("write pasted image: {e}"))?;
    Ok(path.to_string_lossy().into_owned())
}
