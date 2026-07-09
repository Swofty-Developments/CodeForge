//! Minimal query layer over the app DB. Plain functions taking `&Connection`;
//! expand alongside features (row mappers intentionally minimal in the scaffold).

#![allow(dead_code)] // scaffold: consumed once command bodies are implemented

use rusqlite::{params, Connection, OptionalExtension};

pub fn get_setting(conn: &Connection, key: &str) -> anyhow::Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", params![key], |r| {
            r.get::<_, Option<String>>(0)
        })
        .optional()?
        .flatten())
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO settings (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

/// Upsert a repo row on open; returns its id.
pub fn upsert_repo(conn: &Connection, path: &str, name: &str) -> anyhow::Result<String> {
    let now = chrono::Utc::now().to_rfc3339();
    if let Some(id) = conn
        .query_row("SELECT id FROM repos WHERE path = ?1", params![path], |r| {
            r.get::<_, String>(0)
        })
        .optional()?
    {
        conn.execute(
            "UPDATE repos SET last_opened_at = ?1, name = ?2 WHERE id = ?3",
            params![now, name, id],
        )?;
        return Ok(id);
    }
    let id = uuid::Uuid::new_v4().to_string();
    conn.execute(
        "INSERT INTO repos (id, path, name, created_at, last_opened_at) VALUES (?1, ?2, ?3, ?4, ?4)",
        params![id, path, name, now],
    )?;
    Ok(id)
}

pub fn set_repo_indexed_at(conn: &Connection, repo_id: &str, indexed_at: &str) -> anyhow::Result<()> {
    conn.execute(
        "UPDATE repos SET indexed_at = ?1 WHERE id = ?2",
        params![indexed_at, repo_id],
    )?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub fn insert_usage_log(
    conn: &Connection,
    thread_id: &str,
    session_id: Option<&str>,
    input_tokens: u64,
    output_tokens: u64,
    cache_read_tokens: u64,
    cache_write_tokens: u64,
    cost_usd: f64,
    model: Option<&str>,
) -> anyhow::Result<()> {
    conn.execute(
        "INSERT INTO usage_logs (id, thread_id, session_id, input_tokens, output_tokens,
             cache_read_tokens, cache_write_tokens, cost_usd, model, created_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            uuid::Uuid::new_v4().to_string(),
            thread_id,
            session_id,
            input_tokens as i64,
            output_tokens as i64,
            cache_read_tokens as i64,
            cache_write_tokens as i64,
            cost_usd,
            model,
            chrono::Utc::now().to_rfc3339(),
        ],
    )?;
    Ok(())
}
