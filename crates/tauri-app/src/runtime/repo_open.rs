//! The "open a path as a repo context" core, factored out of the `open_repo`
//! command so `create_worktree` can reuse it verbatim: a worktree becomes its
//! OWN [`RepoRuntime`] (own daemon / feature index / timeline), keyed by its
//! path in `AppState.repos` (contract W1). Also the symmetric close-context
//! teardown reused by `close_repo` and `remove_worktree`.

use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use forge_core::{RepoState, TimelineEvent};
use forge_daemon::{Daemon, DaemonDeps};
use forge_index::FeatureIndex;
use forge_timeline::TimelineStore;
use serde::Serialize;
use tauri::Emitter;
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;

use crate::db::Database;
use crate::runtime::indexing::{self, ReindexCtx};
use crate::runtime::{mcp, repo_util, staleness};
use crate::state::{AppState, RepoRuntime};
use crate::{events, queries};

/// `timeline:event` payload (FZ-5): the emitting context's canonical repo path
/// plus the event, so the frontend appends a LIVE event only to its active
/// context and never leaks it across worktrees.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct TimelineEventEnvelope<'a> {
    repo_path: &'a Path,
    event: &'a TimelineEvent,
}

/// Open `root` as a repo context: idempotent for an already-open repo, otherwise
/// install the kit, open timeline + index, start the daemon, upsert the repos
/// row, wire the timeline→`timeline:event` forwarder, register the
/// [`RepoRuntime`], kick off cold-start indexing when never indexed, and emit
/// `repo:changed`. Returns the assembled [`RepoState`].
///
/// The caller has already canonicalized `root` and validated it is a git repo.
pub async fn open_context(
    app: tauri::AppHandle,
    state: &AppState,
    root: PathBuf,
) -> Result<RepoState, String> {
    // Canonicalize so the `repos` map key matches what every other command
    // derives (they all canonicalize their `repo_path`). `open_repo` already
    // passes a canonical path (idempotent here); `create_worktree` passes the
    // freshly created worktree path, which this pins to the same key space.
    let root = repo_util::canonical(&root)?;

    // Idempotent: an already-open repo just reports its current state.
    {
        let repos = state.repos.lock().await;
        if let Some(rt) = repos.get(&root) {
            let count = rt.index.read().await.features().len() as u32;
            let port = rt.daemon.port;
            let repo_id = rt.repo_id.clone();
            drop(repos);
            let indexed_at = read_indexed_at(&state.db, &repo_id).await?;
            let mut rs = repo_util::repo_state(&root, count, Some(port));
            rs.indexed_at = indexed_at; // CONTRACT-3: DB is source of truth
            rs.branch = forge_git::current_branch(&root).await.map_err(|e| e.to_string())?;
            rs.project = Some(repo_util::project_name(&root).await?);
            return Ok(rs);
        }
    }

    let mcp_bin = mcp::ensure_mcp_binary().map_err(|e| format!("{e:#}"))?;
    forge_daemon::install_kit(&root, &mcp_bin).map_err(|e| format!("{e}"))?;

    let timeline = Arc::new(TimelineStore::open(&root).map_err(|e| format!("{e}"))?);
    let index = Arc::new(RwLock::new(FeatureIndex::load(&root).map_err(|e| format!("{e}"))?));

    let deps = DaemonDeps {
        repo_root: root.clone(),
        index: index.clone(),
        timeline: timeline.clone(),
    };
    let daemon = Daemon::start(&root, deps).await.map_err(|e| format!("{e}"))?;
    let port = daemon.port;

    // Upsert the repo row and read its indexed_at in one round-trip. CONTRACT-3:
    // `indexed_at IS NULL` ⇔ never indexed — this decides cold-start below.
    let (repo_id, indexed_at) = {
        let db = state.db.clone();
        let path_s = root.to_string_lossy().into_owned();
        let name = repo_util::repo_name(&root);
        tokio::task::spawn_blocking(move || -> Result<(String, Option<String>), String> {
            let db = db.lock().map_err(|e| e.to_string())?;
            let conn = db.conn();
            let id = queries::upsert_repo(conn, &path_s, &name).map_err(|e| e.to_string())?;
            let indexed = queries::get_repo_indexed_at(conn, &id).map_err(|e| e.to_string())?;
            Ok((id, indexed))
        })
        .await
        .map_err(|e| e.to_string())??
    };
    let indexed_at = indexed_at.as_deref().and_then(parse_rfc3339);
    let never_indexed = indexed_at.is_none();

    // Subscribe before spawning so events between open and first poll aren't lost.
    let mut rx = timeline.subscribe();
    let app_fwd = app.clone();
    // FZ-5: tag each live event with THIS context's canonical path so the
    // frontend appends it only to the active context (no cross-worktree leak).
    let fwd_root = root.clone();
    let forwarder_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = app_fwd.emit(
                        events::TIMELINE_EVENT,
                        TimelineEventEnvelope { repo_path: &fwd_root, event: &event },
                    );
                }
                Err(RecvError::Lagged(n)) => tracing::warn!("timeline forwarder lagged {n} events"),
                Err(RecvError::Closed) => break,
            }
        }
    });

    let reindexing = Arc::new(AtomicBool::new(false));
    let features_count = index.read().await.features().len() as u32;

    // FZ-2: staleness poller — checks the on-disk verdict immediately and then
    // every POLL_INTERVAL, emitting `index:status` on change so the UI can
    // prompt a re-index. Never auto-reindexes — the UI decides.
    let staleness_task = staleness::spawn_poller(app.clone(), root.clone(), reindexing.clone());

    {
        let mut repos = state.repos.lock().await;
        repos.insert(
            root.clone(),
            RepoRuntime {
                daemon,
                index: index.clone(),
                timeline: timeline.clone(),
                repo_id: repo_id.clone(),
                forwarder_task,
                staleness_task,
                reindexing: reindexing.clone(),
            },
        );
    }

    // Never indexed (CONTRACT-3: indexed_at IS NULL) → kick off cold-start.
    if never_indexed {
        let ctx = ReindexCtx {
            app: app.clone(),
            db: state.db.clone(),
            repo_root: root.clone(),
            repo_id,
            index,
            timeline,
            reindexing,
            daemon_port: port,
            force: false,
        };
        if let Err(e) = indexing::spawn_reindex(ctx) {
            tracing::warn!("cold-start not started: {e}");
        }
    }

    let mut repo_state = repo_util::repo_state(&root, features_count, Some(port));
    repo_state.indexed_at = indexed_at; // CONTRACT-3: DB is source of truth
    repo_state.branch = forge_git::current_branch(&root).await.map_err(|e| e.to_string())?;
    repo_state.project = Some(repo_util::project_name(&root).await?);
    let _ = app.emit(events::REPO_CHANGED, &repo_state);
    Ok(repo_state)
}

/// Tear down the context rooted at `root`: stop every session mapped to it (a
/// session can't outlive its context), abort the timeline forwarder and the
/// staleness poller, shut the daemon down, drop the [`RepoRuntime`]. Returns
/// `true` if a context was open
/// (and is now closed), `false` if nothing was open. Reused by `close_repo`
/// (which turns `false` into a named error) and `remove_worktree` (for which a
/// not-open worktree is a legitimate no-op before the git removal).
pub async fn close_context(state: &AppState, root: &Path) -> Result<bool, String> {
    let runtime = {
        let mut repos = state.repos.lock().await;
        repos.remove(root)
    };
    let Some(RepoRuntime { daemon, forwarder_task, staleness_task, .. }) = runtime else {
        return Ok(false);
    };

    // Stop every session mapped to this root so no sidecar keeps running against
    // a closed context.
    let session_ids: Vec<String> = {
        let map = state.session_repos.lock().await;
        map.iter().filter(|&(_, r)| r == root).map(|(id, _)| id.clone()).collect()
    };
    if !session_ids.is_empty() {
        let mut mgr = state.sessions.lock().await;
        for id in &session_ids {
            if let Err(e) = mgr.stop(id).await {
                tracing::warn!(session = %id, "stopping session on context close failed: {e}");
            }
        }
        let mut map = state.session_repos.lock().await;
        for id in &session_ids {
            map.remove(id);
        }
    }

    forwarder_task.abort();
    staleness_task.abort();
    daemon.shutdown().await;
    Ok(true)
}

/// Parse a stored RFC3339 `indexed_at` into a UTC timestamp.
pub(crate) fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
}

/// Read `repos.indexed_at` for an open repo (CONTRACT-3 source of truth).
async fn read_indexed_at(
    db: &Arc<Mutex<Database>>,
    repo_id: &str,
) -> Result<Option<DateTime<Utc>>, String> {
    let db = db.clone();
    let repo_id = repo_id.to_string();
    let raw = tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
        let db = db.lock().map_err(|e| e.to_string())?;
        queries::get_repo_indexed_at(db.conn(), &repo_id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(raw.as_deref().and_then(parse_rfc3339))
}
