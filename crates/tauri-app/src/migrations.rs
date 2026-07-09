//! Idempotent, hand-written migrations. No framework, no version table:
//! `CREATE ... IF NOT EXISTS` for new objects, `column_exists` PRAGMA guard for
//! column additions. Runs on every open.

use rusqlite::Connection;

pub fn run_migrations(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS repos (
            id TEXT PRIMARY KEY NOT NULL,              -- UUID string
            path TEXT NOT NULL UNIQUE,                 -- canonical repo root
            name TEXT NOT NULL,
            created_at TEXT NOT NULL,                  -- RFC3339
            last_opened_at TEXT,
            indexed_at TEXT
        );

        CREATE TABLE IF NOT EXISTS threads (
            id TEXT PRIMARY KEY NOT NULL,
            repo_id TEXT NOT NULL REFERENCES repos(id) ON DELETE CASCADE,
            title TEXT NOT NULL,
            created_at TEXT,
            updated_at TEXT
        );

        CREATE TABLE IF NOT EXISTS messages (
            id TEXT PRIMARY KEY NOT NULL,
            thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            role TEXT NOT NULL CHECK(role IN ('user','assistant','system')),
            content TEXT NOT NULL,
            created_at TEXT
        );

        CREATE TABLE IF NOT EXISTS sessions (
            id TEXT PRIMARY KEY NOT NULL,
            thread_id TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            status TEXT NOT NULL,
            model TEXT,
            pid INTEGER,
            created_at TEXT,
            claude_session_id TEXT                     -- used for --resume
        );

        CREATE TABLE IF NOT EXISTS settings (
            key TEXT PRIMARY KEY NOT NULL,
            value TEXT
        );

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

        CREATE INDEX IF NOT EXISTS idx_threads_repo_id ON threads(repo_id);
        CREATE INDEX IF NOT EXISTS idx_messages_thread_id ON messages(thread_id);
        CREATE INDEX IF NOT EXISTS idx_messages_thread_created ON messages(thread_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_thread_id ON sessions(thread_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_thread_claude ON sessions(thread_id, claude_session_id);
        CREATE INDEX IF NOT EXISTS idx_usage_logs_thread_id ON usage_logs(thread_id);",
    )?;

    // Column-addition template for future migrations:
    // if !column_exists(conn, "threads", "color")? {
    //     conn.execute_batch("ALTER TABLE threads ADD COLUMN color TEXT;")?;
    // }

    Ok(())
}

/// PRAGMA-based guard so `ALTER TABLE ... ADD COLUMN` stays idempotent.
#[allow(dead_code)]
pub fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col| col.as_deref() == Ok(column));
    Ok(exists)
}
