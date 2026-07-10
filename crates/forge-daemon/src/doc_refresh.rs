//! Living-doc auto-refresh worker — the product flywheel: when a Stop hook
//! ends an agent turn (a `SessionEnded` event), refresh the living doc of every
//! feature that turn's file edits touched, then record the outcome as a
//! `DocUpdated` timeline event. Success AND failure are named states on the
//! timeline; the store broadcast makes both live in the UI.

use std::collections::HashSet;
use std::sync::Arc;

use forge_core::{Actor, EventKind, TimelineEvent, TimelineFilter};
use forge_timeline::NewEvent;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::{broadcast, Mutex, Semaphore};
use tokio::task::JoinSet;

use crate::{DaemonDeps, Error, Result};

/// Recent-event window scanned to reconstruct one turn. A turn longer than
/// this folds in a truncated slice (logged), never a wrong-session one.
const SESSION_SLICE_LIMIT: u32 = 1000;

/// Serializes (semaphore of 1 — each refresh shells out to headless `claude`
/// for up to minutes) and dedupes (a slug already queued is not double-queued)
/// living-doc refreshes for one repo.
pub struct DocRefresher {
    deps: DaemonDeps,
    /// Slugs currently waiting on the semaphore.
    queued: Mutex<HashSet<String>>,
    sem: Semaphore,
}

impl DocRefresher {
    pub fn new(deps: DaemonDeps) -> Self {
        Self { deps, queued: Mutex::new(HashSet::new()), sem: Semaphore::new(1) }
    }

    /// React to one `SessionEnded` event: slice that session's turn out of the
    /// timeline, then refresh each touched feature's doc (awaits the whole
    /// backlog). Public so tests drive the worker deterministically without
    /// the broadcast listener.
    pub async fn on_session_ended(&self, session_id: &str, end_event_id: i64) {
        let recent = match self.recent_events().await {
            Ok(events) => events,
            Err(e) => {
                tracing::warn!(session = session_id, "doc refresh skipped — timeline query failed: {e}");
                return;
            }
        };
        if recent.len() as u32 == SESSION_SLICE_LIMIT {
            tracing::warn!(session = session_id, limit = SESSION_SLICE_LIMIT, "turn slice window full — oldest events of a very long turn may be missing");
        }
        let slice = turn_slice(&recent, session_id, end_event_id);
        for slug in touched_slugs(&slice) {
            self.refresh_one(&slug, session_id, &slice).await;
        }
    }

    /// Newest-first recent events, fetched off the async runtime (rusqlite).
    async fn recent_events(&self) -> Result<Vec<TimelineEvent>> {
        let store = self.deps.timeline.clone();
        let filter = TimelineFilter { limit: Some(SESSION_SLICE_LIMIT), ..Default::default() };
        tokio::task::spawn_blocking(move || store.query(&filter))
            .await
            .map_err(|e| Error::Other(format!("turn slice task failed: {e}")))?
            .map_err(Error::Timeline)
    }

    /// Queue-dedupe, serialize, refresh, and record the outcome for one slug.
    async fn refresh_one(&self, slug: &str, session_id: &str, slice: &[TimelineEvent]) {
        {
            let mut queued = self.queued.lock().await;
            if !queued.insert(slug.to_string()) {
                tracing::debug!(slug, "doc refresh already queued — not double-queued");
                return;
            }
        }
        // The semaphore is never closed; Err is unreachable by construction.
        let Ok(_permit) = self.sem.acquire().await else { return };
        // Processing starts: a turn ending DURING this refresh may queue the
        // slug again with its newer slice — exactly the flywheel semantics.
        self.queued.lock().await.remove(slug);

        // Snapshot the feature under a short read lock — never held across the
        // minutes-long claude call. Pinned docs are human-owned: same skip rule
        // as Indexer::write_feature_docs.
        let feature = {
            let index = self.deps.index.read().await;
            match index.get(slug) {
                Some(f) if f.pinned => {
                    tracing::debug!(slug, "pinned feature — doc is human-owned, refresh skipped");
                    return;
                }
                Some(f) => f.clone(),
                None => {
                    tracing::warn!(slug, "touched feature no longer in index — doc refresh skipped");
                    return;
                }
            }
        };

        let refreshed =
            forge_index::refresh_feature_doc(&self.deps.repo_root, &feature, slice).await;
        let payload = match &refreshed {
            Ok(()) => serde_json::json!({
                "slug": slug,
                "outcome": "updated",
                "detail": format!("{} events folded in", slice.len()),
            }),
            Err(e) => {
                tracing::warn!(slug, error = %e, "living-doc refresh failed");
                serde_json::json!({ "slug": slug, "outcome": "failed", "detail": e.to_string() })
            }
        };

        let event = NewEvent {
            session_id: Some(session_id.to_string()),
            actor: Actor::System,
            kind: EventKind::DocUpdated,
            feature_slugs: vec![slug.to_string()],
            payload,
        };
        let store = self.deps.timeline.clone();
        match tokio::task::spawn_blocking(move || store.append(event)).await {
            Ok(Ok(_)) => {}
            Ok(Err(e)) => tracing::warn!(slug, "DocUpdated event append failed: {e}"),
            Err(e) => tracing::warn!(slug, "DocUpdated append task failed: {e}"),
        }
    }
}

/// Spawn the timeline-broadcast listener that drives the refresher. Each
/// `SessionEnded` becomes a detached job so a minutes-long refresh never
/// stalls the receiver into lagging. Aborting the returned task drops the
/// internal `JoinSet`, aborting in-flight jobs (`kill_on_drop` in forge-index
/// reaps a running claude child).
pub(crate) fn spawn_listener(
    refresher: Arc<DocRefresher>,
    rx: broadcast::Receiver<TimelineEvent>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(listen(refresher, rx))
}

async fn listen(refresher: Arc<DocRefresher>, mut rx: broadcast::Receiver<TimelineEvent>) {
    let mut jobs: JoinSet<()> = JoinSet::new();
    loop {
        tokio::select! {
            received = rx.recv() => match received {
                Ok(event) => {
                    if event.kind != EventKind::SessionEnded {
                        continue;
                    }
                    match event.session_id.clone() {
                        Some(session_id) => {
                            let refresher = Arc::clone(&refresher);
                            jobs.spawn(async move {
                                refresher.on_session_ended(&session_id, event.id).await;
                            });
                        }
                        // Named state: a session-less end cannot be sliced to a turn.
                        None => tracing::debug!(event = event.id, "session_ended without session id — no doc refresh"),
                    }
                }
                Err(RecvError::Lagged(n)) => {
                    tracing::warn!(missed = n, "doc-refresh listener lagged; a turn's Stop event may have been dropped");
                }
                Err(RecvError::Closed) => break,
            },
            Some(finished) = jobs.join_next(), if !jobs.is_empty() => {
                if let Err(e) = finished {
                    tracing::warn!("doc refresh job panicked: {e}");
                }
            }
        }
    }
}

/// One turn's slice from a NEWEST-FIRST event list: events strictly between
/// the session's previous boundary (its nearest `SessionEnded`/`SessionStarted`
/// before `end_event_id`) and the terminating `SessionEnded`. Keeps what the
/// refresh prompt renders: the session's `FileEdited` + `CommandRun`, plus
/// `Note` events in the window (`/api/notes` appends notes session-less).
/// Returned chronologically.
fn turn_slice(
    newest_first: &[TimelineEvent],
    session_id: &str,
    end_event_id: i64,
) -> Vec<TimelineEvent> {
    let mut slice = Vec::new();
    for event in newest_first.iter().filter(|e| e.id < end_event_id) {
        let same_session = event.session_id.as_deref() == Some(session_id);
        if same_session && matches!(event.kind, EventKind::SessionEnded | EventKind::SessionStarted)
        {
            break;
        }
        let keep = match event.kind {
            EventKind::FileEdited | EventKind::CommandRun => same_session,
            EventKind::Note => true,
            _ => false,
        };
        if keep {
            slice.push(event.clone());
        }
    }
    slice.reverse();
    slice
}

/// Deduped feature slugs of the slice's `FileEdited` events, first-touched order.
fn touched_slugs(slice: &[TimelineEvent]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut slugs = Vec::new();
    for event in slice.iter().filter(|e| e.kind == EventKind::FileEdited) {
        for slug in &event.feature_slugs {
            if seen.insert(slug.clone()) {
                slugs.push(slug.clone());
            }
        }
    }
    slugs
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;

    fn ev(id: i64, session: Option<&str>, kind: EventKind, slugs: &[&str]) -> TimelineEvent {
        TimelineEvent {
            id,
            ts: Utc::now(),
            session_id: session.map(str::to_owned),
            actor: Actor::Agent,
            kind,
            feature_slugs: slugs.iter().map(|s| s.to_string()).collect(),
            payload: serde_json::json!({}),
        }
    }

    /// Interleaved sessions: only the ENDING session's edits (plus session-less
    /// notes in the window) are in its turn slice.
    #[test]
    fn turn_slice_keeps_only_the_ending_sessions_turn() {
        let mut events = vec![
            ev(1, Some("s1"), EventKind::SessionStarted, &[]),
            ev(2, Some("s1"), EventKind::FileEdited, &["auth"]),
            ev(3, Some("s2"), EventKind::SessionStarted, &[]),
            ev(4, Some("s2"), EventKind::FileEdited, &["billing"]),
            ev(5, None, EventKind::Note, &[]),
            ev(6, Some("s1"), EventKind::CommandRun, &[]),
            ev(7, Some("s2"), EventKind::SessionEnded, &[]),
        ];
        events.reverse(); // store queries return newest first

        let slice = turn_slice(&events, "s2", 7);
        assert_eq!(slice.iter().map(|e| e.id).collect::<Vec<_>>(), vec![4, 5]);
        assert_eq!(touched_slugs(&slice), vec!["billing".to_string()]);
    }

    /// A session's SECOND turn slices back only to its previous SessionEnded.
    #[test]
    fn turn_slice_stops_at_the_sessions_previous_boundary() {
        let mut events = vec![
            ev(1, Some("s1"), EventKind::SessionStarted, &[]),
            ev(2, Some("s1"), EventKind::FileEdited, &["auth"]),
            ev(3, Some("s1"), EventKind::SessionEnded, &[]),
            ev(4, Some("s1"), EventKind::FileEdited, &["timeline"]),
            ev(5, Some("s1"), EventKind::SessionEnded, &[]),
        ];
        events.reverse();

        let slice = turn_slice(&events, "s1", 5);
        assert_eq!(slice.iter().map(|e| e.id).collect::<Vec<_>>(), vec![4]);
        assert_eq!(touched_slugs(&slice), vec!["timeline".to_string()]);
    }

    #[test]
    fn touched_slugs_dedupes_in_first_touch_order() {
        let slice = vec![
            ev(1, Some("s"), EventKind::FileEdited, &["b", "a"]),
            ev(2, Some("s"), EventKind::FileEdited, &["a", "c"]),
            ev(3, Some("s"), EventKind::CommandRun, &["ignored-kind"]),
        ];
        assert_eq!(touched_slugs(&slice), vec!["b", "a", "c"]);
    }
}
