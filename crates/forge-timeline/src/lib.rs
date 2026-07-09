//! forge-timeline — per-repo append-only event log.
//!
//! Storage: SQLite at `<repo>/.featureforge/runtime/timeline.db`, WAL mode,
//! `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000` (same pragma set
//! as the app DB). Events are immutable once appended; `id` is the rowid.

#![allow(dead_code)] // scaffold: fields are consumed once bodies are implemented

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use forge_core::{Actor, EventKind, TimelineEvent, TimelineFilter};
use rusqlite::Connection;
use tokio::sync::broadcast;

/// Timeline errors.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, Error>;

/// A not-yet-appended event: everything of [`TimelineEvent`] except `id` and `ts`,
/// which are assigned at write time.
#[derive(Debug, Clone)]
pub struct NewEvent {
    pub session_id: Option<String>,
    pub actor: Actor,
    pub kind: EventKind,
    pub feature_slugs: Vec<String>,
    pub payload: serde_json::Value,
}

/// Append-only per-repo event store with live subscriptions.
///
/// Cheap to share behind an `Arc`; the connection is internally synchronised.
pub struct TimelineStore {
    repo_root: PathBuf,
    db_path: PathBuf,
    conn: Mutex<Connection>,
    /// Every successful [`append`](Self::append) is also broadcast to subscribers.
    events_tx: broadcast::Sender<TimelineEvent>,
}

impl TimelineStore {
    /// Open (creating if needed) `<repo_root>/.featureforge/runtime/timeline.db`
    /// and run idempotent migrations.
    pub fn open(repo_root: &Path) -> Result<Self> {
        let _ = repo_root;
        // IMPLEMENT(agent): create .featureforge/runtime/, open SQLite with WAL
        // pragmas, run idempotent migrations (events table + indexes on kind/ts,
        // feature_slugs stored as JSON array text), construct broadcast channel
        // (capacity 256).
        todo!("TimelineStore::open")
    }

    /// Append an event. `id` and `ts` are assigned here; the stored event is
    /// returned and broadcast to all subscribers.
    pub fn append(&self, event: NewEvent) -> Result<TimelineEvent> {
        let _ = event;
        // IMPLEMENT(agent): INSERT, read back rowid, broadcast, return.
        todo!("TimelineStore::append")
    }

    /// Query events matching `filter`, newest first. Default limit 200.
    pub fn query(&self, filter: &TimelineFilter) -> Result<Vec<TimelineEvent>> {
        let _ = filter;
        // IMPLEMENT(agent): build WHERE from filter (feature_slug matches JSON
        // array containment, kinds via IN, since via ts >=), ORDER BY id DESC.
        todo!("TimelineStore::query")
    }

    /// Subscribe to live events (everything appended after this call).
    pub fn subscribe(&self) -> broadcast::Receiver<TimelineEvent> {
        self.events_tx.subscribe()
    }

    /// Absolute path of the underlying database file.
    pub fn db_path(&self) -> &Path {
        &self.db_path
    }
}
