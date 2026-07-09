use std::path::Path;

use rusqlite::Connection;

use crate::migrations::run_migrations;

/// App-level SQLite database at `~/.featureforge/featureforge.db`.
///
/// Single connection behind `Arc<std::sync::Mutex<Database>>` in managed state.
/// The pragma set below is the tested combo for single-connection desktop use.
#[allow(dead_code)] // scaffold: consumed once command bodies are implemented
pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;
             PRAGMA cache_size=-8000;
             PRAGMA mmap_size=268435456;
             PRAGMA busy_timeout=5000;",
        )?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    #[cfg(test)]
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    #[allow(dead_code)] // scaffold: consumed once command bodies are implemented
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn migrations_are_idempotent() {
        let db = Database::open_in_memory().unwrap();
        // Running again must be a no-op, not an error.
        run_migrations(db.conn()).unwrap();
        let count: i64 = db
            .conn()
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table' AND name IN
                 ('repos','threads','messages','sessions','settings','usage_logs')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 6);
    }
}
