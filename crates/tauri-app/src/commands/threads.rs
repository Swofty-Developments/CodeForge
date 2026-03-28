use tauri::State;
use uuid::Uuid;

use crate::commands::projects::ThreadResponse;
use crate::state::TauriState;

#[tauri::command]
pub fn create_thread(
    state: State<'_, TauriState>,
    project_id: String,
    title: String,
    _provider: String,
) -> Result<ThreadResponse, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let id = Uuid::new_v4();
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
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let uid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    codeforge_persistence::queries::update_thread_title(db.conn(), uid, &title)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_thread_color(
    state: State<'_, TauriState>,
    id: String,
    color: Option<String>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let uid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    codeforge_persistence::queries::update_thread_color(db.conn(), uid, color.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_thread(state: State<'_, TauriState>, id: String) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let uid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    let _ = codeforge_persistence::queries::delete_messages_by_thread(db.conn(), uid);
    codeforge_persistence::queries::delete_thread(db.conn(), uid).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn move_thread_to_project(
    state: State<'_, TauriState>,
    thread_id: String,
    target_project_id: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let tid = Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;
    let pid = Uuid::parse_str(&target_project_id).map_err(|e| e.to_string())?;
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
pub fn persist_user_message(
    state: State<'_, TauriState>,
    thread_id: String,
    content: String,
) -> Result<String, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let tid = Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;
    let msg_id = Uuid::new_v4();
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
