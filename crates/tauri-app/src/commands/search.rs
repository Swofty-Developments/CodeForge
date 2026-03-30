use rusqlite::params;
use serde::Serialize;
use tauri::State;

use crate::state::TauriState;

#[derive(Debug, Serialize)]
pub struct SearchResult {
    pub thread_id: String,
    pub thread_title: String,
    pub project_name: String,
    pub message_id: String,
    pub role: String,
    pub content_snippet: String,
    pub match_index: usize,
}

#[tauri::command]
pub fn search_messages(
    state: State<'_, TauriState>,
    query: String,
) -> Result<Vec<SearchResult>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }

    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let conn = db.conn();

    let sql = "
        SELECT m.id, m.thread_id, m.role, m.content,
               t.title AS thread_title,
               p.name AS project_name
        FROM messages m
        JOIN threads t ON m.thread_id = t.id
        JOIN projects p ON t.project_id = p.id
        WHERE m.content LIKE '%' || ?1 || '%'
        LIMIT 50
    ";

    let mut stmt = conn.prepare(sql).map_err(|e| e.to_string())?;
    let query_lower = query.to_lowercase();

    let rows = stmt
        .query_map(params![query], |row| {
            let id: String = row.get(0)?;
            let thread_id: String = row.get(1)?;
            let role: String = row.get(2)?;
            let content: String = row.get(3)?;
            let thread_title: String = row.get(4)?;
            let project_name: String = row.get(5)?;
            Ok((id, thread_id, role, content, thread_title, project_name))
        })
        .map_err(|e| e.to_string())?;

    let mut results = Vec::new();
    for row in rows {
        let (id, thread_id, role, content, thread_title, project_name) =
            row.map_err(|e| e.to_string())?;

        let content_lower = content.to_lowercase();
        if let Some(idx) = content_lower.find(&query_lower) {
            let start = idx.saturating_sub(25);
            let end = (idx + query.len() + 25).min(content.len());
            let snippet = content[start..end].to_string();

            results.push(SearchResult {
                thread_id,
                thread_title,
                project_name,
                message_id: id,
                role,
                content_snippet: snippet,
                match_index: idx,
            });
        }
    }

    Ok(results)
}
