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
    )
}
