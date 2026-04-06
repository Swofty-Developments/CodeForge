use tauri::State;

use codeforge_persistence::{MessageId, ProjectId, ThreadId};

use crate::commands::projects::ThreadResponse;
use crate::state::TauriState;

#[tauri::command]
pub fn create_thread(
    state: State<'_, TauriState>,
    project_id: String,
    title: String,
    _provider: String,
) -> Result<ThreadResponse, String> {
    tracing::debug!("create_thread project_id={project_id}");
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let pid = project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    let id = ThreadId::new();
    let now = chrono::Utc::now();
    let thread = codeforge_persistence::Thread {
        id,
        project_id: pid,
        title: title.clone(),
        color: None,
        created_at: now,
        updated_at: now,
    };
    codeforge_persistence::queries::insert_thread(db.conn(), &thread)
        .map_err(|e| e.to_string())?;
    Ok(ThreadResponse {
        id: id.to_string(),
        project_id: project_id,
        title,
        color: None,
    })
}

#[tauri::command]
pub fn rename_thread(state: State<'_, TauriState>, id: String, title: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let uid = id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    codeforge_persistence::queries::update_thread_title(db.conn(), uid, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_thread_color(
    state: State<'_, TauriState>,
    id: String,
    color: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let uid = id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    codeforge_persistence::queries::update_thread_color(db.conn(), uid, color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_thread(state: State<'_, TauriState>, id: String) -> Result<(), String> {
    tracing::debug!("delete_thread id={id}");
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let uid = id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    if let Err(e) = codeforge_persistence::queries::delete_messages_by_thread(db.conn(), uid) {
        tracing::error!("Failed to delete messages for thread {uid}: {e}");
    }
    codeforge_persistence::queries::delete_thread(db.conn(), uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_thread_to_project(
    state: State<'_, TauriState>,
    thread_id: String,
    target_project_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    let pid = target_project_id.parse::<ProjectId>().map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().to_rfc3339();
    db.conn()
        .execute(
            "UPDATE threads SET project_id = ?1, updated_at = ?2 WHERE id = ?3",
            rusqlite::params![pid.to_string(), now, tid.to_string()],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_messages_after(
    state: State<'_, TauriState>,
    thread_id: String,
    message_id: String,
) -> Result<u64, String> {
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    let mid = message_id.parse::<MessageId>().map_err(|e| e.to_string())?;
    codeforge_persistence::queries::delete_messages_after(db.conn(), tid, mid)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn persist_user_message(
    state: State<'_, TauriState>,
    thread_id: String,
    content: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| {
        tracing::error!("Failed to lock database: {e}");
        format!("{e}")
    })?;
    let tid = thread_id.parse::<ThreadId>().map_err(|e| e.to_string())?;
    let msg_id = MessageId::new();
    let msg = codeforge_persistence::Message {
        id: msg_id,
        thread_id: tid,
        role: codeforge_persistence::MessageRole::User,
        content,
        created_at: chrono::Utc::now(),
    };
    codeforge_persistence::queries::insert_message(db.conn(), &msg)
        .map_err(|e| e.to_string())?;
    Ok(msg_id.to_string())
}
