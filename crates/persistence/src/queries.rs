use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use uuid::Uuid;

use crate::models::*;

// ---------------------------------------------------------------------------
// Projects
// ---------------------------------------------------------------------------

pub fn insert_project(conn: &Connection, project: &Project) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO projects (id, path, name, created_at) VALUES (?1, ?2, ?3, ?4)",
        params![
            project.id.to_string(),
            project.path,
            project.name,
            project.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_all_projects(conn: &Connection) -> anyhow::Result<Vec<Project>> {
    let mut stmt = conn.prepare("SELECT id, path, name, created_at FROM projects")?;
    let rows = stmt.query_map([], |row| {
        Ok(ProjectRow {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_project()))
        .collect()
}

pub fn get_project_by_id(conn: &Connection, id: Uuid) -> anyhow::Result<Option<Project>> {
    let mut stmt =
        conn.prepare("SELECT id, path, name, created_at FROM projects WHERE id = ?1")?;
    let mut rows = stmt.query_map(params![id.to_string()], |row| {
        Ok(ProjectRow {
            id: row.get(0)?,
            path: row.get(1)?,
            name: row.get(2)?,
            created_at: row.get(3)?,
        })
    })?;
    match rows.next() {
        Some(r) => Ok(Some(r?.into_project()?)),
        None => Ok(None),
    }
}

pub fn delete_project(conn: &Connection, id: Uuid) -> anyhow::Result<()> {
    conn.execute("DELETE FROM projects WHERE id = ?1", params![id.to_string()])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Threads
// ---------------------------------------------------------------------------

pub fn insert_thread(conn: &Connection, thread: &Thread) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO threads (id, project_id, title, color, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        params![
            thread.id.to_string(),
            thread.project_id.to_string(),
            thread.title,
            thread.color,
            thread.created_at.to_rfc3339(),
            thread.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_threads_by_project(conn: &Connection, project_id: Uuid) -> anyhow::Result<Vec<Thread>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, color, created_at, updated_at FROM threads WHERE project_id = ?1 ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id.to_string()], |row| {
        Ok(ThreadRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            color: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_thread()))
        .collect()
}

pub fn get_thread_by_id(conn: &Connection, id: Uuid) -> anyhow::Result<Option<Thread>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, color, created_at, updated_at FROM threads WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id.to_string()], |row| {
        Ok(ThreadRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            color: row.get(3)?,
            created_at: row.get(4)?,
            updated_at: row.get(5)?,
        })
    })?;
    match rows.next() {
        Some(r) => Ok(Some(r?.into_thread()?)),
        None => Ok(None),
    }
}

pub fn update_thread_title(conn: &Connection, id: Uuid, title: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE threads SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, id.to_string()],
    )?;
    Ok(())
}

pub fn update_thread_color(
    conn: &Connection,
    id: Uuid,
    color: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE threads SET color = ?1, updated_at = ?2 WHERE id = ?3",
        params![color, now, id.to_string()],
    )?;
    Ok(())
}

pub fn delete_thread(conn: &Connection, id: Uuid) -> anyhow::Result<()> {
    conn.execute("DELETE FROM threads WHERE id = ?1", params![id.to_string()])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Messages
// ---------------------------------------------------------------------------

pub fn insert_message(conn: &Connection, message: &Message) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO messages (id, thread_id, role, content, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            message.id.to_string(),
            message.thread_id.to_string(),
            message.role.as_str(),
            message.content,
            message.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_messages_by_thread(conn: &Connection, thread_id: Uuid) -> anyhow::Result<Vec<Message>> {
    let mut stmt = conn.prepare(
        "SELECT id, thread_id, role, content, created_at FROM messages WHERE thread_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![thread_id.to_string()], |row| {
        Ok(MessageRow {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_message()))
        .collect()
}

pub fn delete_messages_by_thread(conn: &Connection, thread_id: Uuid) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM messages WHERE thread_id = ?1",
        params![thread_id.to_string()],
    )?;
    Ok(())
}

pub fn delete_messages_after(
    conn: &Connection,
    thread_id: Uuid,
    message_id: Uuid,
) -> anyhow::Result<u64> {
    // Get the created_at of the reference message
    let created_at: String = conn.query_row(
        "SELECT created_at FROM messages WHERE id = ?1 AND thread_id = ?2",
        params![message_id.to_string(), thread_id.to_string()],
        |row| row.get(0),
    )?;
    // Delete all messages in the thread created after this message (exclusive)
    let deleted = conn.execute(
        "DELETE FROM messages WHERE thread_id = ?1 AND created_at > ?2",
        params![thread_id.to_string(), created_at],
    )?;
    Ok(deleted as u64)
}

// ---------------------------------------------------------------------------
// Sessions
// ---------------------------------------------------------------------------

pub fn insert_session(conn: &Connection, session: &Session) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO sessions (id, thread_id, provider, status, approval_mode, pid, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            session.id.to_string(),
            session.thread_id.to_string(),
            session.provider.as_str(),
            session.status,
            session.approval_mode,
            session.pid,
            session.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_sessions_by_thread(
    conn: &Connection,
    thread_id: Uuid,
) -> anyhow::Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, thread_id, provider, status, approval_mode, pid, created_at FROM sessions WHERE thread_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![thread_id.to_string()], |row| {
        Ok(SessionRow {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            provider: row.get(2)?,
            status: row.get(3)?,
            approval_mode: row.get(4)?,
            pid: row.get(5)?,
            created_at: row.get(6)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_session()))
        .collect()
}

pub fn update_session_status(conn: &Connection, id: Uuid, status: &str) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE sessions SET status = ?1 WHERE id = ?2",
        params![status, id.to_string()],
    )?;
    Ok(())
}

pub fn update_session_claude_id(
    conn: &Connection,
    id: Uuid,
    claude_session_id: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE sessions SET claude_session_id = ?1 WHERE id = ?2",
        params![claude_session_id, id.to_string()],
    )?;
    Ok(())
}

/// Get the most recent Claude session ID for a thread (for --resume).
pub fn get_latest_claude_session_id(
    conn: &Connection,
    thread_id: Uuid,
) -> anyhow::Result<Option<String>> {
    let mut stmt = conn.prepare(
        "SELECT claude_session_id FROM sessions WHERE thread_id = ?1 AND claude_session_id IS NOT NULL ORDER BY created_at DESC LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![thread_id.to_string()], |row| {
        row.get::<_, Option<String>>(0)
    })?;
    match rows.next() {
        Some(r) => Ok(r?),
        None => Ok(None),
    }
}

pub fn delete_session(conn: &Connection, id: Uuid) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM sessions WHERE id = ?1",
        params![id.to_string()],
    )?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Settings
// ---------------------------------------------------------------------------

pub fn get_setting(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
    let mut rows = stmt.query_map(params![key], |row| row.get::<_, Option<String>>(0))?;
    match rows.next() {
        Some(r) => Ok(r?),
        None => Ok(None),
    }
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_settings_batch(
    conn: &Connection,
    keys: &[String],
) -> anyhow::Result<std::collections::HashMap<String, String>> {
    if keys.is_empty() {
        return Ok(std::collections::HashMap::new());
    }
    // Build a parameterized IN clause
    let placeholders: Vec<String> = (1..=keys.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "SELECT key, value FROM settings WHERE key IN ({})",
        placeholders.join(", ")
    );
    let mut stmt = conn.prepare(&sql)?;
    let params: Vec<&dyn rusqlite::types::ToSql> =
        keys.iter().map(|k| k as &dyn rusqlite::types::ToSql).collect();
    let rows = stmt.query_map(params.as_slice(), |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?))
    })?;
    let mut map = std::collections::HashMap::new();
    for row in rows {
        let (key, value) = row?;
        if let Some(v) = value {
            map.insert(key, v);
        }
    }
    Ok(map)
}

pub fn delete_setting(conn: &Connection, key: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Usage Logs
// ---------------------------------------------------------------------------

pub fn insert_usage_log(
    conn: &Connection,
    id: &str,
    thread_id: &str,
    session_id: Option<&str>,
    input_tokens: i64,
    output_tokens: i64,
    cache_read_tokens: i64,
    cache_write_tokens: i64,
    cost_usd: f64,
    model: Option<&str>,
    created_at: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO usage_logs (id, thread_id, session_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, model, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![id, thread_id, session_id, input_tokens, output_tokens, cache_read_tokens, cache_write_tokens, cost_usd, model, created_at],
    )?;
    Ok(())
}

pub fn get_usage_totals(conn: &Connection) -> anyhow::Result<(i64, i64, i64, i64, f64)> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COALESCE(SUM(cache_read_tokens),0), COALESCE(SUM(cache_write_tokens),0), COALESCE(SUM(cost_usd),0.0) FROM usage_logs",
    )?;
    let result = stmt.query_row([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;
    Ok(result)
}

pub fn get_usage_by_thread(conn: &Connection) -> anyhow::Result<Vec<(String, f64, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT u.thread_id, COALESCE(SUM(u.cost_usd),0.0), COALESCE(SUM(u.input_tokens + u.output_tokens),0) FROM usage_logs u GROUP BY u.thread_id ORDER BY SUM(u.cost_usd) DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    rows.map(|r| r.map_err(Into::into)).collect()
}

pub fn get_usage_by_model(conn: &Connection) -> anyhow::Result<Vec<(String, f64, i64)>> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(model,'unknown'), COALESCE(SUM(cost_usd),0.0), COALESCE(SUM(input_tokens + output_tokens),0) FROM usage_logs GROUP BY model ORDER BY SUM(cost_usd) DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, f64>(1)?,
            row.get::<_, i64>(2)?,
        ))
    })?;
    rows.map(|r| r.map_err(Into::into)).collect()
}

pub fn get_usage_for_thread(conn: &Connection, thread_id: &str) -> anyhow::Result<(i64, i64, i64, i64, f64)> {
    let mut stmt = conn.prepare(
        "SELECT COALESCE(SUM(input_tokens),0), COALESCE(SUM(output_tokens),0), COALESCE(SUM(cache_read_tokens),0), COALESCE(SUM(cache_write_tokens),0), COALESCE(SUM(cost_usd),0.0) FROM usage_logs WHERE thread_id = ?1",
    )?;
    let result = stmt.query_row(params![thread_id], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, f64>(4)?,
        ))
    })?;
    Ok(result)
}

// ---------------------------------------------------------------------------
// Internal row types for mapping from SQLite text columns
// ---------------------------------------------------------------------------

struct ProjectRow {
    id: String,
    path: String,
    name: String,
    created_at: String,
}

impl ProjectRow {
    fn into_project(self) -> anyhow::Result<Project> {
        Ok(Project {
            id: Uuid::parse_str(&self.id)?,
            path: self.path,
            name: self.name,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
        })
    }
}

struct ThreadRow {
    id: String,
    project_id: String,
    title: String,
    color: Option<String>,
    created_at: String,
    updated_at: String,
}

impl ThreadRow {
    fn into_thread(self) -> anyhow::Result<Thread> {
        Ok(Thread {
            id: Uuid::parse_str(&self.id)?,
            project_id: Uuid::parse_str(&self.project_id)?,
            title: self.title,
            color: self.color,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)?.with_timezone(&Utc),
        })
    }
}

struct MessageRow {
    id: String,
    thread_id: String,
    role: String,
    content: String,
    created_at: String,
}

impl MessageRow {
    fn into_message(self) -> anyhow::Result<Message> {
        Ok(Message {
            id: Uuid::parse_str(&self.id)?,
            thread_id: Uuid::parse_str(&self.thread_id)?,
            role: MessageRole::from_str(&self.role).map_err(|e| anyhow::anyhow!(e))?,
            content: self.content,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
        })
    }
}

struct SessionRow {
    id: String,
    thread_id: String,
    provider: String,
    status: String,
    approval_mode: Option<String>,
    pid: Option<i64>,
    created_at: String,
}

impl SessionRow {
    fn into_session(self) -> anyhow::Result<Session> {
        Ok(Session {
            id: Uuid::parse_str(&self.id)?,
            thread_id: Uuid::parse_str(&self.thread_id)?,
            provider: Provider::from_str(&self.provider).map_err(|e| anyhow::anyhow!(e))?,
            status: self.status,
            approval_mode: self.approval_mode,
            pid: self.pid,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
        })
    }
}
