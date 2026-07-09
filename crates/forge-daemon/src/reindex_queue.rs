//! Server-owned queue of file edits the daemon could not classify.
//!
//! When a `FileEdited` hook lands on a path that matches no feature (a brand-new
//! or renamed file, or a stale index), the timeline event is marked
//! `unclassified` and the path is queued here. A later reindex drains the queue
//! (via [`ReindexQueue::pending`] + [`ReindexQueue::clear`]) to retroactively
//! resolve the file→feature link instead of losing it forever.
//!
//! Storage: SQLite at `<repo>/.codeforge/runtime/reindex.db` — daemon-owned
//! state, kept separate from the append-only timeline DB.

use std::path::Path;
use std::sync::Mutex;

use chrono::Utc;
use rusqlite::{params, Connection};

use crate::{Error, Result};

/// One queued path awaiting reclassification after the next reindex.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PendingReindex {
    /// Repo-relative path that classified to no feature.
    pub path: String,
    /// Timeline event id that recorded the unclassified edit.
    pub event_id: i64,
    /// Claude session that made the edit, if agent-originated.
    pub session_id: Option<String>,
    /// RFC3339 timestamp of the most recent enqueue for this path.
    pub enqueued_at: String,
}

/// Durable queue of unclassified edited paths (daemon-owned state).
pub struct ReindexQueue {
    conn: Mutex<Connection>,
}

impl ReindexQueue {
    /// Open (creating if needed) `<repo>/.codeforge/runtime/reindex.db`.
    pub fn open(repo_root: &Path) -> Result<Self> {
        let runtime_dir = repo_root.join(".codeforge").join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let conn = Connection::open(runtime_dir.join("reindex.db"))?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA busy_timeout=5000;
             CREATE TABLE IF NOT EXISTS pending_reindex (
                 path        TEXT PRIMARY KEY,
                 event_id    INTEGER NOT NULL,
                 session_id  TEXT,
                 enqueued_at TEXT NOT NULL
             );",
        )?;
        tracing::debug!(repo = %repo_root.display(), "reindex queue opened");
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// Record that repo-relative `path` was edited but classified to no feature,
    /// linked to the timeline `event_id` that named the state. Repeated edits to
    /// the same path collapse to one row carrying the most recent event.
    pub fn enqueue(&self, path: &str, event_id: i64, session_id: Option<&str>) -> Result<()> {
        let conn = self.lock()?;
        conn.execute(
            "INSERT INTO pending_reindex (path, event_id, session_id, enqueued_at)
             VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(path) DO UPDATE SET
                 event_id = excluded.event_id,
                 session_id = excluded.session_id,
                 enqueued_at = excluded.enqueued_at",
            params![path, event_id, session_id, Utc::now().to_rfc3339()],
        )?;
        tracing::debug!(path, event_id, "queued unclassified edit for reindex");
        Ok(())
    }

    /// Every queued path, oldest first. A reindex reclassifies these, then calls
    /// [`Self::clear`] with the paths it resolved.
    pub fn pending(&self) -> Result<Vec<PendingReindex>> {
        let conn = self.lock()?;
        let mut stmt = conn.prepare(
            "SELECT path, event_id, session_id, enqueued_at
             FROM pending_reindex
             ORDER BY enqueued_at ASC, path ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(PendingReindex {
                path: row.get(0)?,
                event_id: row.get(1)?,
                session_id: row.get(2)?,
                enqueued_at: row.get(3)?,
            })
        })?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    /// Remove queued entries whose path is in `paths` (a reindex resolved them).
    pub fn clear(&self, paths: &[String]) -> Result<()> {
        let mut conn = self.lock()?;
        let tx = conn.transaction()?;
        {
            let mut stmt = tx.prepare("DELETE FROM pending_reindex WHERE path = ?1")?;
            for path in paths {
                stmt.execute(params![path])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Number of queued paths.
    pub fn len(&self) -> Result<usize> {
        let conn = self.lock()?;
        let n: i64 = conn.query_row("SELECT COUNT(*) FROM pending_reindex", [], |r| r.get(0))?;
        Ok(n as usize)
    }

    /// True when nothing is queued.
    pub fn is_empty(&self) -> Result<bool> {
        Ok(self.len()? == 0)
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.conn
            .lock()
            .map_err(|_| Error::Other("reindex queue mutex poisoned".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open() -> (tempfile::TempDir, ReindexQueue) {
        let dir = tempfile::tempdir().unwrap();
        let q = ReindexQueue::open(dir.path()).unwrap();
        (dir, q)
    }

    #[test]
    fn enqueue_dedupes_by_path_keeping_latest_event() {
        let (_dir, q) = open();
        q.enqueue("src/new.rs", 1, Some("s1")).unwrap();
        q.enqueue("src/new.rs", 7, Some("s2")).unwrap();
        let pending = q.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, "src/new.rs");
        assert_eq!(pending[0].event_id, 7);
        assert_eq!(pending[0].session_id.as_deref(), Some("s2"));
    }

    #[test]
    fn clear_removes_resolved_paths_only() {
        let (_dir, q) = open();
        q.enqueue("a.rs", 1, None).unwrap();
        q.enqueue("b.rs", 2, None).unwrap();
        q.clear(&["a.rs".to_string()]).unwrap();
        let pending = q.pending().unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].path, "b.rs");
    }

    #[test]
    fn survives_reopen() {
        let dir = tempfile::tempdir().unwrap();
        {
            let q = ReindexQueue::open(dir.path()).unwrap();
            q.enqueue("keep.rs", 3, None).unwrap();
        }
        let q = ReindexQueue::open(dir.path()).unwrap();
        assert_eq!(q.len().unwrap(), 1);
        assert!(!q.is_empty().unwrap());
        assert_eq!(q.pending().unwrap()[0].path, "keep.rs");
    }
}
