//! forge-timeline — per-repo append-only event log.
//!
//! Storage: SQLite at `<repo>/.featureforge/runtime/timeline.db`, WAL mode,
//! `foreign_keys=ON`, `synchronous=NORMAL`, `busy_timeout=5000` (same pragma set
//! as the app DB). Events are immutable once appended; `id` is the rowid.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use chrono::{DateTime, SecondsFormat, Utc};
use forge_core::{Actor, EventKind, TimelineEvent, TimelineFilter};
use rusqlite::{params, params_from_iter, Connection, Row, ToSql};
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

const DEFAULT_LIMIT: u32 = 200;
const BROADCAST_CAPACITY: usize = 512;

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
/// Cheap to clone (`Arc` inner); the connection is internally synchronised,
/// so the store is `Send + Sync`.
#[derive(Clone)]
pub struct TimelineStore {
    inner: Arc<Inner>,
}

struct Inner {
    #[allow(dead_code)] // kept for future daemon introspection
    repo_root: PathBuf,
    db_path: PathBuf,
    // Single connection behind a std Mutex: the Mutex serialises this process's
    // access (making the store Sync), while WAL keeps external readers non-blocking.
    conn: Mutex<Connection>,
    /// Every successful [`append`](TimelineStore::append) is also broadcast to subscribers.
    events_tx: broadcast::Sender<TimelineEvent>,
}

impl TimelineStore {
    /// Open (creating if needed) `<repo_root>/.featureforge/runtime/timeline.db`
    /// and run idempotent migrations.
    pub fn open(repo_root: &Path) -> Result<Self> {
        let runtime_dir = repo_root.join(".featureforge").join("runtime");
        std::fs::create_dir_all(&runtime_dir)?;
        let db_path = runtime_dir.join("timeline.db");

        let conn = Connection::open(&db_path)?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA foreign_keys=ON;
             PRAGMA synchronous=NORMAL;
             PRAGMA busy_timeout=5000;",
        )?;
        migrate(&conn)?;

        let (events_tx, _) = broadcast::channel(BROADCAST_CAPACITY);
        tracing::debug!(db = %db_path.display(), "timeline store opened");
        Ok(Self {
            inner: Arc::new(Inner {
                repo_root: repo_root.to_path_buf(),
                db_path,
                conn: Mutex::new(conn),
                events_tx,
            }),
        })
    }

    /// Append an event. `id` and `ts` are assigned here; the stored event is
    /// returned and broadcast to all subscribers.
    pub fn append(&self, event: NewEvent) -> Result<TimelineEvent> {
        // Truncate to micros so the returned ts is byte-identical to a later read-back.
        let now = Utc::now();
        let ts = DateTime::<Utc>::from_timestamp_micros(now.timestamp_micros())
            .ok_or_else(|| Error::Other("timestamp out of range".into()))?;

        let actor = enum_to_str(&event.actor)?;
        let kind = enum_to_str(&event.kind)?;
        let feature_slugs = serde_json::to_string(&event.feature_slugs)?;
        let payload = serde_json::to_string(&event.payload)?;

        let id = {
            let conn = self.lock_conn()?;
            conn.execute(
                "INSERT INTO events (ts, session_id, actor, kind, feature_slugs, payload)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![fmt_ts(&ts), event.session_id, actor, kind, feature_slugs, payload],
            )?;
            conn.last_insert_rowid()
        };

        let stored = TimelineEvent {
            id,
            ts,
            session_id: event.session_id,
            actor: event.actor,
            kind: event.kind,
            feature_slugs: event.feature_slugs,
            payload: event.payload,
        };
        tracing::trace!(id, kind = %kind_label(&stored.kind), "timeline event appended");
        // Ok(0) receivers / lagged subscribers are acceptable by contract.
        let _ = self.inner.events_tx.send(stored.clone());
        Ok(stored)
    }

    /// Query events matching `filter`, newest first. Default limit 200.
    pub fn query(&self, filter: &TimelineFilter) -> Result<Vec<TimelineEvent>> {
        let mut clauses: Vec<String> = Vec::new();
        let mut params: Vec<Box<dyn ToSql>> = Vec::new();

        if let Some(slug) = &filter.feature_slug {
            clauses.push(
                "EXISTS (SELECT 1 FROM json_each(events.feature_slugs) WHERE json_each.value = ?)"
                    .into(),
            );
            params.push(Box::new(slug.clone()));
        }
        if let Some(actor) = filter.actor {
            clauses.push("actor = ?".into());
            params.push(Box::new(enum_to_str(&actor)?));
        }
        if let Some(kinds) = &filter.kinds {
            if kinds.is_empty() {
                return Ok(Vec::new());
            }
            let placeholders = vec!["?"; kinds.len()].join(", ");
            clauses.push(format!("kind IN ({placeholders})"));
            for kind in kinds {
                params.push(Box::new(enum_to_str(kind)?));
            }
        }
        if let Some(since) = filter.since {
            clauses.push("ts >= ?".into());
            params.push(Box::new(fmt_ts(&since)));
        }

        let mut sql =
            String::from("SELECT id, ts, session_id, actor, kind, feature_slugs, payload FROM events");
        if !clauses.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&clauses.join(" AND "));
        }
        sql.push_str(" ORDER BY id DESC LIMIT ?");
        params.push(Box::new(i64::from(filter.limit.unwrap_or(DEFAULT_LIMIT))));

        let conn = self.lock_conn()?;
        let mut stmt = conn.prepare(&sql)?;
        let mut rows = stmt.query(params_from_iter(params.iter().map(|p| p.as_ref())))?;
        let mut events = Vec::new();
        while let Some(row) = rows.next()? {
            events.push(event_from_row(row)?);
        }
        Ok(events)
    }

    /// Subscribe to live events (everything appended after this call).
    pub fn subscribe(&self) -> broadcast::Receiver<TimelineEvent> {
        self.inner.events_tx.subscribe()
    }

    /// Absolute path of the underlying database file.
    pub fn db_path(&self) -> &Path {
        &self.inner.db_path
    }

    fn lock_conn(&self) -> Result<std::sync::MutexGuard<'_, Connection>> {
        self.inner
            .conn
            .lock()
            .map_err(|_| Error::Other("timeline connection mutex poisoned".into()))
    }
}

fn migrate(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS events (
            id            INTEGER PRIMARY KEY,
            ts            TEXT NOT NULL,
            session_id    TEXT,
            actor         TEXT NOT NULL,
            kind          TEXT NOT NULL,
            feature_slugs TEXT NOT NULL DEFAULT '[]',
            payload       TEXT NOT NULL DEFAULT 'null'
        );
        CREATE INDEX IF NOT EXISTS idx_events_kind ON events(kind);
        CREATE INDEX IF NOT EXISTS idx_events_ts ON events(ts);",
    )?;
    Ok(())
}

// Fixed-width RFC3339 (micros, Z) so `ts >= ?` string comparison is chronological.
fn fmt_ts(ts: &DateTime<Utc>) -> String {
    ts.to_rfc3339_opts(SecondsFormat::Micros, true)
}

fn parse_ts(s: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .map(|d| d.with_timezone(&Utc))
        .map_err(|e| Error::Other(format!("bad ts {s:?} in timeline.db: {e}")))
}

/// Serialize a snake_case serde enum (Actor/EventKind) to its string form.
fn enum_to_str<T: serde::Serialize>(value: &T) -> Result<String> {
    match serde_json::to_value(value)? {
        serde_json::Value::String(s) => Ok(s),
        other => Err(Error::Other(format!("expected string encoding, got {other}"))),
    }
}

fn enum_from_str<T: serde::de::DeserializeOwned>(s: &str) -> Result<T> {
    Ok(serde_json::from_value(serde_json::Value::String(
        s.to_owned(),
    ))?)
}

fn kind_label(kind: &EventKind) -> String {
    enum_to_str(kind).unwrap_or_else(|_| format!("{kind:?}"))
}

fn event_from_row(row: &Row<'_>) -> Result<TimelineEvent> {
    let ts: String = row.get(1)?;
    let actor: String = row.get(3)?;
    let kind: String = row.get(4)?;
    let feature_slugs: String = row.get(5)?;
    let payload: String = row.get(6)?;
    Ok(TimelineEvent {
        id: row.get(0)?,
        ts: parse_ts(&ts)?,
        session_id: row.get(2)?,
        actor: enum_from_str(&actor)?,
        kind: enum_from_str(&kind)?,
        feature_slugs: serde_json::from_str(&feature_slugs)?,
        payload: serde_json::from_str(&payload)?,
    })
}
