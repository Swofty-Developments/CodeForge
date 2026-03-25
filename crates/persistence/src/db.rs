use std::path::Path;

use rusqlite::Connection;

use crate::migrations::run_migrations;

/// Database connection manager wrapping a rusqlite connection.
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Open (or create) a database file at the given path and run migrations.
    pub fn open(path: &Path) -> anyhow::Result<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Open an in-memory database (useful for testing).
    pub fn open_in_memory() -> anyhow::Result<Self> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch("PRAGMA foreign_keys=ON;")?;
        run_migrations(&conn)?;
        Ok(Self { conn })
    }

    /// Borrow the underlying connection.
    pub fn conn(&self) -> &Connection {
        &self.conn
    }
}
