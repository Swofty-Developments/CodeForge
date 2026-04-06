use rusqlite::Connection;

fn column_exists(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare(&format!("PRAGMA table_info({table})"))?;
    let exists = stmt
        .query_map([], |row| row.get::<_, String>(1))?
        .any(|col| col.as_deref() == Ok(column));
    Ok(exists)
}

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
    if !column_exists(conn, "threads", "color")? {
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
    if !column_exists(conn, "sessions", "claude_session_id")? {
        conn.execute_batch("ALTER TABLE sessions ADD COLUMN claude_session_id TEXT;")?;
    }

    // Worktrees table — replaces worktree:* and pr:* settings keys
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS worktrees (
            id         TEXT PRIMARY KEY NOT NULL,
            thread_id  TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            branch     TEXT NOT NULL,
            path       TEXT NOT NULL,
            pr_number  INTEGER,
            status     TEXT NOT NULL DEFAULT 'active' CHECK(status IN ('active', 'merged', 'deleted', 'orphaned')),
            created_at TEXT NOT NULL,
            updated_at TEXT NOT NULL
        );
        ",
    )?;

    // Migrate existing worktree/pr data from settings table into worktrees table.
    // Format in settings: worktree:<thread_id> = <branch>|<path>, pr:<thread_id> = <number>
    {
        let mut wt_stmt = conn.prepare(
            "SELECT key, value FROM settings WHERE key LIKE 'worktree:%'"
        )?;
        let wt_rows: Vec<(String, String)> = wt_stmt
            .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))?
            .filter_map(|r| r.ok())
            .collect();

        for (key, val) in &wt_rows {
            let thread_id = key.strip_prefix("worktree:").unwrap_or(key);
            let parts: Vec<&str> = val.splitn(2, '|').collect();
            if parts.len() != 2 { continue; }
            let branch = parts[0];
            let path = parts[1];

            // Look up PR number if linked
            let pr_num: Option<u32> = conn
                .query_row(
                    "SELECT value FROM settings WHERE key = ?1",
                    rusqlite::params![format!("pr:{thread_id}")],
                    |row| row.get::<_, String>(0),
                )
                .ok()
                .and_then(|v| v.parse().ok());

            // Look up project_id from threads table
            let project_id: Option<String> = conn
                .query_row(
                    "SELECT project_id FROM threads WHERE id = ?1",
                    rusqlite::params![thread_id],
                    |row| row.get(0),
                )
                .ok();

            if let Some(project_id) = project_id {
                // Only migrate if not already present
                let exists: bool = conn
                    .query_row(
                        "SELECT COUNT(*) FROM worktrees WHERE thread_id = ?1",
                        rusqlite::params![thread_id],
                        |row| row.get::<_, i64>(0),
                    )
                    .unwrap_or(0) > 0;

                if !exists {
                    let id = uuid::Uuid::new_v4().to_string();
                    let now = chrono::Utc::now().to_rfc3339();
                    let _ = conn.execute(
                        "INSERT INTO worktrees (id, thread_id, project_id, branch, path, pr_number, status, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 'active', ?7, ?7)",
                        rusqlite::params![id, thread_id, project_id, branch, path, pr_num, now],
                    );
                }
            }
        }

        // Clean up migrated settings
        if !wt_rows.is_empty() {
            conn.execute_batch(
                "DELETE FROM settings WHERE key LIKE 'worktree:%'; DELETE FROM settings WHERE key LIKE 'pr:%';",
            )?;
        }
    }

    // Turn checkpoints — tracks HEAD commit at start of each AI turn for undo
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS turn_checkpoints (
            id          TEXT PRIMARY KEY NOT NULL,
            thread_id   TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
            turn_id     TEXT NOT NULL,
            commit_sha  TEXT NOT NULL,
            created_at  TEXT NOT NULL
        );
        ",
    )?;

    // ── Add full PR lifecycle columns to worktrees ──
    if !column_exists(conn, "worktrees", "pr_state")? {
        conn.execute_batch("ALTER TABLE worktrees ADD COLUMN pr_state TEXT;")?;
    }
    if !column_exists(conn, "worktrees", "pr_merge_commit")? {
        conn.execute_batch("ALTER TABLE worktrees ADD COLUMN pr_merge_commit TEXT;")?;
    }
    if !column_exists(conn, "worktrees", "last_seen_comment_count")? {
        conn.execute_batch(
            "ALTER TABLE worktrees ADD COLUMN last_seen_comment_count INTEGER NOT NULL DEFAULT 0;",
        )?;
    }
    if !column_exists(conn, "worktrees", "pr_url")? {
        conn.execute_batch("ALTER TABLE worktrees ADD COLUMN pr_url TEXT;")?;
    }

    // ── Expand the CHECK constraint on worktrees.status to include 'closed' ──
    // SQLite can't ALTER a CHECK constraint; we rebuild the table only if needed.
    let needs_status_rebuild: bool = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master
             WHERE type='table' AND name='worktrees'
             AND sql LIKE '%''active'', ''merged'', ''deleted'', ''orphaned''%'
             AND sql NOT LIKE '%''closed''%'",
            [],
            |row| row.get::<_, i64>(0),
        )
        .unwrap_or(0) > 0;

    if needs_status_rebuild {
        conn.execute_batch(
            "
            CREATE TABLE worktrees_new (
                id                        TEXT PRIMARY KEY NOT NULL,
                thread_id                 TEXT NOT NULL REFERENCES threads(id) ON DELETE CASCADE,
                project_id                TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
                branch                    TEXT NOT NULL,
                path                      TEXT NOT NULL,
                pr_number                 INTEGER,
                status                    TEXT NOT NULL DEFAULT 'active'
                    CHECK(status IN ('active', 'merged', 'deleted', 'orphaned', 'closed')),
                created_at                TEXT NOT NULL,
                updated_at                TEXT NOT NULL,
                pr_state                  TEXT,
                pr_merge_commit           TEXT,
                last_seen_comment_count   INTEGER NOT NULL DEFAULT 0,
                pr_url                    TEXT
            );
            INSERT INTO worktrees_new (id, thread_id, project_id, branch, path, pr_number, status, created_at, updated_at, pr_state, pr_merge_commit, last_seen_comment_count, pr_url)
                SELECT id, thread_id, project_id, branch, path, pr_number, status, created_at, updated_at, pr_state, pr_merge_commit, last_seen_comment_count, pr_url FROM worktrees;
            DROP TABLE worktrees;
            ALTER TABLE worktrees_new RENAME TO worktrees;
            ",
        )?;
    }

    // ── Add system_kind to messages for event vs state differentiation ──
    if !column_exists(conn, "messages", "system_kind")? {
        conn.execute_batch("ALTER TABLE messages ADD COLUMN system_kind TEXT;")?;
    }

    // Performance indexes — idempotent via IF NOT EXISTS
    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_threads_project_id ON threads(project_id);
        CREATE INDEX IF NOT EXISTS idx_messages_thread_id ON messages(thread_id);
        CREATE INDEX IF NOT EXISTS idx_messages_thread_created ON messages(thread_id, created_at);
        CREATE INDEX IF NOT EXISTS idx_sessions_thread_id ON sessions(thread_id);
        CREATE INDEX IF NOT EXISTS idx_sessions_thread_claude ON sessions(thread_id, claude_session_id);
        CREATE INDEX IF NOT EXISTS idx_usage_logs_thread_id ON usage_logs(thread_id);
        CREATE INDEX IF NOT EXISTS idx_worktrees_thread_id ON worktrees(thread_id);
        CREATE INDEX IF NOT EXISTS idx_worktrees_project_id ON worktrees(project_id);
        CREATE INDEX IF NOT EXISTS idx_turn_checkpoints_thread ON turn_checkpoints(thread_id);
        ",
    )?;

    Ok(())
}
