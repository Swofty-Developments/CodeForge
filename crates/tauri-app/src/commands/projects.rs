use serde::Serialize;
use tauri::State;
use uuid::Uuid;

use crate::state::TauriState;

#[derive(Debug, Serialize)]
pub struct ProjectResponse {
    pub id: String,
    pub name: String,
    pub path: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ThreadResponse {
    pub id: String,
    pub project_id: String,
    pub title: String,
    pub color: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: String,
    pub thread_id: String,
    pub role: String,
    pub content: String,
}

#[tauri::command]
pub fn get_all_projects(state: State<'_, TauriState>) -> Result<Vec<ProjectResponse>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let projects = codeforge_persistence::queries::get_all_projects(db.conn())
        .map_err(|e| e.to_string())?;
    Ok(projects
        .into_iter()
        .map(|p| ProjectResponse {
            id: p.id.to_string(),
            name: p.name,
            path: p.path,
            color: None,
        })
        .collect())
}

#[tauri::command]
pub fn get_threads_by_project(
    state: State<'_, TauriState>,
    project_id: String,
) -> Result<Vec<ThreadResponse>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let pid = Uuid::parse_str(&project_id).map_err(|e| e.to_string())?;
    let threads = codeforge_persistence::queries::get_threads_by_project(db.conn(), pid)
        .map_err(|e| e.to_string())?;
    Ok(threads
        .into_iter()
        .map(|t| ThreadResponse {
            id: t.id.to_string(),
            project_id: t.project_id.to_string(),
            title: t.title,
            color: t.color,
        })
        .collect())
}

#[tauri::command]
pub fn get_messages_by_thread(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<Vec<MessageResponse>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let tid = Uuid::parse_str(&thread_id).map_err(|e| e.to_string())?;
    let messages = codeforge_persistence::queries::get_messages_by_thread(db.conn(), tid)
        .map_err(|e| e.to_string())?;
    Ok(messages
        .into_iter()
        .map(|m| MessageResponse {
            id: m.id.to_string(),
            thread_id: m.thread_id.to_string(),
            role: m.role.as_str().to_string(),
            content: m.content,
        })
        .collect())
}

#[tauri::command]
pub fn create_project(
    state: State<'_, TauriState>,
    name: String,
    path: String,
) -> Result<ProjectResponse, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let id = Uuid::new_v4();
    let project = codeforge_persistence::Project {
        id,
        name: name.clone(),
        path: path.clone(),
        created_at: chrono::Utc::now(),
    };
    codeforge_persistence::queries::insert_project(db.conn(), &project)
        .map_err(|e| e.to_string())?;
    Ok(ProjectResponse {
        id: id.to_string(),
        name,
        path,
        color: None,
    })
}

#[tauri::command]
pub fn rename_project(
    state: State<'_, TauriState>,
    id: String,
    name: String,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let uid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    db.conn()
        .execute(
            "UPDATE projects SET name = ?1 WHERE id = ?2",
            rusqlite::params![name, uid.to_string()],
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub fn delete_project(
    state: State<'_, TauriState>,
    id: String,
    delete_threads: bool,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let uid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;

    if delete_threads {
        let threads =
            codeforge_persistence::queries::get_threads_by_project(db.conn(), uid)
                .map_err(|e| e.to_string())?;
        for t in threads {
            let _ = codeforge_persistence::queries::delete_messages_by_thread(db.conn(), t.id);
            let _ = codeforge_persistence::queries::delete_thread(db.conn(), t.id);
        }
    }

    codeforge_persistence::queries::delete_project(db.conn(), uid)
        .map_err(|e| e.to_string())?;
    Ok(())
}
