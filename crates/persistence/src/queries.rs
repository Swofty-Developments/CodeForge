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
        "INSERT INTO threads (id, project_id, title, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![
            thread.id.to_string(),
            thread.project_id.to_string(),
            thread.title,
            thread.created_at.to_rfc3339(),
            thread.updated_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_threads_by_project(conn: &Connection, project_id: Uuid) -> anyhow::Result<Vec<Thread>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, created_at, updated_at FROM threads WHERE project_id = ?1 ORDER BY updated_at DESC",
    )?;
    let rows = stmt.query_map(params![project_id.to_string()], |row| {
        Ok(ThreadRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_thread()))
        .collect()
}

pub fn get_thread_by_id(conn: &Connection, id: Uuid) -> anyhow::Result<Option<Thread>> {
    let mut stmt = conn.prepare(
        "SELECT id, project_id, title, created_at, updated_at FROM threads WHERE id = ?1",
    )?;
    let mut rows = stmt.query_map(params![id.to_string()], |row| {
        Ok(ThreadRow {
            id: row.get(0)?,
            project_id: row.get(1)?,
            title: row.get(2)?,
            created_at: row.get(3)?,
            updated_at: row.get(4)?,
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

pub fn delete_setting(conn: &Connection, key: &str) -> anyhow::Result<()> {
    conn.execute("DELETE FROM settings WHERE key = ?1", params![key])?;
    Ok(())
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
    created_at: String,
    updated_at: String,
}

impl ThreadRow {
    fn into_thread(self) -> anyhow::Result<Thread> {
        Ok(Thread {
            id: Uuid::parse_str(&self.id)?,
            project_id: Uuid::parse_str(&self.project_id)?,
            title: self.title,
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
