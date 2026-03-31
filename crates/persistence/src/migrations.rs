use rusqlite::Connection;

/// Run all database migrations. This is idempotent — tables are created only if
/// they do not already exist.
pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS projects (
            id         TEXT PRIMARY KEY NOT NULL,
            path       TEXT NOT NULL,
            name       TEXT NOT NULL,
            created_at TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS threads (
            id         TEXT PRIMARY KEY NOT NULL,
            project_id TEXT NOT NULL REFERENCES projects(id),
            title      TEXT NOT NULL,
            created_at TEXT,
            updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS messages (
            id         TEXT PRIMARY KEY NOT NULL,
            thread_id  TEXT NOT NULL REFERENCES threads(id),
            role       TEXT NOT NULL CHECK(role IN ('user', 'assistant', 'system')),
            content    TEXT NOT NULL,
            created_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id            TEXT PRIMARY KEY NOT NULL,
            thread_id     TEXT NOT NULL REFERENCES threads(id),
            provider      TEXT NOT NULL CHECK(provider IN ('claude', 'codex')),
            status        TEXT NOT NULL,
            approval_mode TEXT,
            pid           INTEGER,
            created_at    TEXT
        );

        CREATE TABLE IF NOT EXISTS settings (
            key   TEXT PRIMARY KEY NOT NULL,
            value TEXT
        );
        ",
    )?;

    // Add color column to threads if missing
    let has_color: bool = conn
        .prepare("PRAGMA table_info(threads)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col| col.as_deref() == Ok("color"));

    if !has_color {
        conn.execute_batch("ALTER TABLE threads ADD COLUMN color TEXT;")?;
    }

    // Usage logs table
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS usage_logs (
            id TEXT PRIMARY KEY NOT NULL,
            thread_id TEXT NOT NULL,
            session_id TEXT,
            input_tokens INTEGER NOT NULL DEFAULT 0,
            output_tokens INTEGER NOT NULL DEFAULT 0,
            cache_read_tokens INTEGER NOT NULL DEFAULT 0,
            cache_write_tokens INTEGER NOT NULL DEFAULT 0,
            cost_usd REAL NOT NULL DEFAULT 0,
            model TEXT,
            created_at TEXT NOT NULL
        );
        ",
    )?;

    // Add claude_session_id column to sessions if missing (used for --resume)
    let has_claude_session_id: bool = conn
        .prepare("PRAGMA table_info(sessions)")?
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col| col.as_deref() == Ok("claude_session_id"));

    if !has_claude_session_id {
        conn.execute_batch("ALTER TABLE sessions ADD COLUMN claude_session_id TEXT;")?;
    }

    Ok(())
}
