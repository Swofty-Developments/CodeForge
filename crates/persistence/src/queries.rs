use chrono::{DateTime, Utc};
use codeforge_core::id::{MessageId, ProjectId, SessionId, ThreadId, WorktreeId};
use rusqlite::{params, Connection};

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

pub fn get_project_by_id(conn: &Connection, id: ProjectId) -> anyhow::Result<Option<Project>> {
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

pub fn delete_project(conn: &Connection, id: ProjectId) -> anyhow::Result<()> {
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

pub fn get_threads_by_project(conn: &Connection, project_id: ProjectId) -> anyhow::Result<Vec<Thread>> {
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

pub fn get_thread_by_id(conn: &Connection, id: ThreadId) -> anyhow::Result<Option<Thread>> {
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

pub fn update_thread_title(conn: &Connection, id: ThreadId, title: &str) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE threads SET title = ?1, updated_at = ?2 WHERE id = ?3",
        params![title, now, id.to_string()],
    )?;
    Ok(())
}

pub fn update_thread_color(
    conn: &Connection,
    id: ThreadId,
    color: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE threads SET color = ?1, updated_at = ?2 WHERE id = ?3",
        params![color, now, id.to_string()],
    )?;
    Ok(())
}

pub fn delete_thread(conn: &Connection, id: ThreadId) -> anyhow::Result<()> {
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

pub fn get_messages_by_thread(conn: &Connection, thread_id: ThreadId) -> anyhow::Result<Vec<Message>> {
    get_messages_by_thread_paginated(conn, thread_id, None, None)
}

/// Load messages with optional limit (most recent N) and offset.
/// When limit is Some, returns the LAST N messages (ordered ascending).
pub fn get_messages_by_thread_paginated(
    conn: &Connection,
    thread_id: ThreadId,
    limit: Option<u32>,
    offset: Option<u32>,
) -> anyhow::Result<Vec<Message>> {
    let query = match (limit, offset) {
        (Some(lim), Some(off)) => format!(
            "SELECT id, thread_id, role, content, created_at FROM messages \
             WHERE thread_id = ?1 ORDER BY created_at DESC LIMIT {} OFFSET {}",
            lim, off
        ),
        (Some(lim), None) => format!(
            "SELECT id, thread_id, role, content, created_at FROM messages \
             WHERE thread_id = ?1 ORDER BY created_at DESC LIMIT {}",
            lim
        ),
        _ => "SELECT id, thread_id, role, content, created_at FROM messages \
              WHERE thread_id = ?1 ORDER BY created_at ASC"
            .to_string(),
    };

    let mut stmt = conn.prepare(&query)?;
    let rows = stmt.query_map(params![thread_id.to_string()], |row| {
        Ok(MessageRow {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            role: row.get(2)?,
            content: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    let mut messages: Vec<Message> = rows
        .map(|r| r.map_err(Into::into).and_then(|r| r.into_message()))
        .collect::<anyhow::Result<Vec<_>>>()?;

    // If we used DESC ordering (for limit), reverse to get chronological order
    if limit.is_some() {
        messages.reverse();
    }
    Ok(messages)
}

pub fn delete_messages_by_thread(conn: &Connection, thread_id: ThreadId) -> anyhow::Result<()> {
    conn.execute(
        "DELETE FROM messages WHERE thread_id = ?1",
        params![thread_id.to_string()],
    )?;
    Ok(())
}

pub fn delete_messages_after(
    conn: &Connection,
    thread_id: ThreadId,
    message_id: MessageId,
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
        "INSERT INTO sessions (id, thread_id, provider, status, approval_mode, pid, claude_session_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            session.id.to_string(),
            session.thread_id.to_string(),
            session.provider.as_str(),
            session.status,
            session.approval_mode,
            session.pid,
            session.claude_session_id,
            session.created_at.to_rfc3339(),
        ],
    )?;
    Ok(())
}

pub fn get_sessions_by_thread(
    conn: &Connection,
    thread_id: ThreadId,
) -> anyhow::Result<Vec<Session>> {
    let mut stmt = conn.prepare(
        "SELECT id, thread_id, provider, status, approval_mode, pid, claude_session_id, created_at FROM sessions WHERE thread_id = ?1 ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map(params![thread_id.to_string()], |row| {
        Ok(SessionRow {
            id: row.get(0)?,
            thread_id: row.get(1)?,
            provider: row.get(2)?,
            status: row.get(3)?,
            approval_mode: row.get(4)?,
            pid: row.get(5)?,
            claude_session_id: row.get(6)?,
            created_at: row.get(7)?,
        })
    })?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_session()))
        .collect()
}

pub fn update_session_status(conn: &Connection, id: SessionId, status: &str) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE sessions SET status = ?1 WHERE id = ?2",
        params![status, id.to_string()],
    )?;
    Ok(())
}

pub fn update_session_claude_id(
    conn: &Connection,
    id: SessionId,
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
    thread_id: ThreadId,
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

pub fn delete_session(conn: &Connection, id: SessionId) -> anyhow::Result<()> {
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
// Turn Checkpoints
// ---------------------------------------------------------------------------

pub fn insert_turn_checkpoint(
    conn: &Connection,
    id: &str,
    thread_id: &str,
    turn_id: &str,
    commit_sha: &str,
    created_at: &str,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO turn_checkpoints (id, thread_id, turn_id, commit_sha, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
        params![id, thread_id, turn_id, commit_sha, created_at],
    )?;
    Ok(())
}

pub fn get_turn_checkpoints(
    conn: &Connection,
    thread_id: &str,
) -> anyhow::Result<Vec<(String, String, String)>> {
    let mut stmt = conn.prepare(
        "SELECT turn_id, commit_sha, created_at FROM turn_checkpoints WHERE thread_id = ?1 ORDER BY created_at ASC",
    )?;
    let rows = stmt.query_map(params![thread_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    })?;
    rows.map(|r| r.map_err(Into::into)).collect()
}

// ---------------------------------------------------------------------------
// Worktrees
// ---------------------------------------------------------------------------

const WORKTREE_COLS: &str = "id, thread_id, project_id, branch, path, pr_number, status, created_at, updated_at, pr_state, pr_merge_commit, last_seen_comment_count, pr_url";

fn map_worktree_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorktreeRow> {
    Ok(WorktreeRow {
        id: row.get(0)?,
        thread_id: row.get(1)?,
        project_id: row.get(2)?,
        branch: row.get(3)?,
        path: row.get(4)?,
        pr_number: row.get(5)?,
        status: row.get(6)?,
        created_at: row.get(7)?,
        updated_at: row.get(8)?,
        pr_state: row.get(9)?,
        pr_merge_commit: row.get(10)?,
        last_seen_comment_count: row.get(11)?,
        pr_url: row.get(12)?,
    })
}

pub fn insert_worktree(conn: &Connection, worktree: &Worktree) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO worktrees (id, thread_id, project_id, branch, path, pr_number, status, created_at, updated_at, pr_state, pr_merge_commit, last_seen_comment_count, pr_url)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        params![
            worktree.id.to_string(),
            worktree.thread_id.to_string(),
            worktree.project_id.to_string(),
            worktree.branch,
            worktree.path,
            worktree.pr_number,
            worktree.status.as_str(),
            worktree.created_at.to_rfc3339(),
            worktree.updated_at.to_rfc3339(),
            worktree.pr_state.as_ref().map(|s| s.as_str()),
            worktree.pr_merge_commit,
            worktree.last_seen_comment_count,
            worktree.pr_url,
        ],
    )?;
    Ok(())
}

/// Returns the most recent ACTIVE worktree for a thread.
/// Use this for operations that should only work on editable worktrees
/// (merge/push/undo/sync).
pub fn get_active_worktree_by_thread(conn: &Connection, thread_id: ThreadId) -> anyhow::Result<Option<Worktree>> {
    let sql = format!("SELECT {WORKTREE_COLS} FROM worktrees WHERE thread_id = ?1 AND status = 'active' ORDER BY created_at DESC LIMIT 1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![thread_id.to_string()], map_worktree_row)?;
    match rows.next() {
        Some(r) => Ok(Some(r?.into_worktree()?)),
        None => Ok(None),
    }
}

/// Returns the most recent worktree for a thread, regardless of status.
/// Use this for display / state restoration. Use `get_active_worktree_by_thread`
/// for operations that should only work on active worktrees.
pub fn get_worktree_by_thread(conn: &Connection, thread_id: ThreadId) -> anyhow::Result<Option<Worktree>> {
    let sql = format!("SELECT {WORKTREE_COLS} FROM worktrees WHERE thread_id = ?1 ORDER BY created_at DESC LIMIT 1");
    let mut stmt = conn.prepare(&sql)?;
    let mut rows = stmt.query_map(params![thread_id.to_string()], map_worktree_row)?;
    match rows.next() {
        Some(r) => Ok(Some(r?.into_worktree()?)),
        None => Ok(None),
    }
}

pub fn get_worktrees_by_project(conn: &Connection, project_id: ProjectId) -> anyhow::Result<Vec<Worktree>> {
    let sql = format!("SELECT {WORKTREE_COLS} FROM worktrees WHERE project_id = ?1 ORDER BY created_at DESC");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(params![project_id.to_string()], map_worktree_row)?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_worktree()))
        .collect()
}

pub fn get_all_active_worktrees(conn: &Connection) -> anyhow::Result<Vec<Worktree>> {
    let sql = format!("SELECT {WORKTREE_COLS} FROM worktrees WHERE status = 'active'");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], map_worktree_row)?;
    rows.map(|r| r.map_err(Into::into).and_then(|r| r.into_worktree()))
        .collect()
}

pub fn update_worktree_status(conn: &Connection, id: WorktreeId, status: WorktreeStatus) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE worktrees SET status = ?1, updated_at = ?2 WHERE id = ?3",
        params![status.as_str(), now, id.to_string()],
    )?;
    Ok(())
}

pub fn update_worktree_pr(conn: &Connection, thread_id: ThreadId, pr_number: u32) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE worktrees SET pr_number = ?1, updated_at = ?2 WHERE thread_id = ?3 AND status = 'active'",
        params![pr_number, now, thread_id.to_string()],
    )?;
    Ok(())
}

/// Update the cached GitHub PR state + merge commit for a worktree.
/// Called by the PR poller as it reconciles GitHub state.
pub fn update_worktree_pr_state(
    conn: &Connection,
    id: WorktreeId,
    pr_state: PrGhState,
    merge_commit: Option<&str>,
    pr_url: Option<&str>,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE worktrees SET pr_state = ?1, pr_merge_commit = ?2, pr_url = COALESCE(?3, pr_url), updated_at = ?4 WHERE id = ?5",
        params![pr_state.as_str(), merge_commit, pr_url, now, id.to_string()],
    )?;
    Ok(())
}

/// Persist the number of PR comments the poller has seen so far.
/// This is the high-water mark used to compute deltas between polls across
/// app restarts.
pub fn update_worktree_comment_count(
    conn: &Connection,
    id: WorktreeId,
    count: u32,
) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE worktrees SET last_seen_comment_count = ?1, updated_at = ?2 WHERE id = ?3",
        params![count, now, id.to_string()],
    )?;
    Ok(())
}

/// Clear PR linkage on a worktree. Used when the PR is deleted/transferred on
/// GitHub and the worktree should revert to "branch only, no PR".
pub fn clear_worktree_pr(conn: &Connection, id: WorktreeId) -> anyhow::Result<()> {
    let now = Utc::now().to_rfc3339();
    conn.execute(
        "UPDATE worktrees SET pr_number = NULL, pr_state = NULL, pr_merge_commit = NULL, pr_url = NULL, last_seen_comment_count = 0, updated_at = ?1 WHERE id = ?2",
        params![now, id.to_string()],
    )?;
    Ok(())
}

pub fn find_thread_for_pr_number(conn: &Connection, pr_number: u32) -> anyhow::Result<Option<ThreadId>> {
    let mut stmt = conn.prepare(
        "SELECT thread_id FROM worktrees WHERE pr_number = ?1 AND status = 'active' LIMIT 1",
    )?;
    let mut rows = stmt.query_map(params![pr_number], |row| row.get::<_, String>(0))?;
    match rows.next() {
        Some(r) => {
            let id_str = r?;
            Ok(Some(id_str.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?))
        }
        None => Ok(None),
    }
}

pub fn delete_worktree(conn: &Connection, id: WorktreeId) -> anyhow::Result<()> {
    conn.execute("DELETE FROM worktrees WHERE id = ?1", params![id.to_string()])?;
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
            id: self.id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
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
            id: self.id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            project_id: self.project_id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
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
            id: self.id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            thread_id: self.thread_id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            role: self.role.parse::<MessageRole>().map_err(|e| anyhow::anyhow!(e))?,
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
    claude_session_id: Option<String>,
    created_at: String,
}

impl SessionRow {
    fn into_session(self) -> anyhow::Result<Session> {
        Ok(Session {
            id: self.id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            thread_id: self.thread_id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            provider: self.provider.parse::<Provider>().map_err(|e| anyhow::anyhow!(e))?,
            status: self.status,
            approval_mode: self.approval_mode,
            pid: self.pid,
            claude_session_id: self.claude_session_id,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
        })
    }
}

struct WorktreeRow {
    id: String,
    thread_id: String,
    project_id: String,
    branch: String,
    path: String,
    pr_number: Option<u32>,
    status: String,
    created_at: String,
    updated_at: String,
    pr_state: Option<String>,
    pr_merge_commit: Option<String>,
    last_seen_comment_count: u32,
    pr_url: Option<String>,
}

impl WorktreeRow {
    fn into_worktree(self) -> anyhow::Result<Worktree> {
        let pr_state = match self.pr_state.as_deref() {
            Some(s) => Some(s.parse::<PrGhState>().map_err(|e| anyhow::anyhow!(e))?),
            None => None,
        };
        Ok(Worktree {
            id: self.id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            thread_id: self.thread_id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            project_id: self.project_id.parse().map_err(|e: codeforge_core::id::IdParseError| anyhow::anyhow!(e))?,
            branch: self.branch,
            path: self.path,
            pr_number: self.pr_number,
            status: self.status.parse::<WorktreeStatus>().map_err(|e| anyhow::anyhow!(e))?,
            created_at: DateTime::parse_from_rfc3339(&self.created_at)?.with_timezone(&Utc),
            updated_at: DateTime::parse_from_rfc3339(&self.updated_at)?.with_timezone(&Utc),
            pr_state,
            pr_merge_commit: self.pr_merge_commit,
            last_seen_comment_count: self.last_seen_comment_count,
            pr_url: self.pr_url,
        })
    }
}
