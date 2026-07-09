use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};

use chrono::{DateTime, Utc};
use forge_core::RepoState;
use forge_daemon::{Daemon, DaemonDeps};
use forge_index::FeatureIndex;
use forge_timeline::TimelineStore;
use serde::Serialize;
use tauri::{Emitter, State};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::RwLock;

use crate::db::Database;
use crate::runtime::indexing::{self, ReindexCtx};
use crate::runtime::{mcp, repo_util};
use crate::state::{AppState, RepoRuntime};
use crate::{events, queries};

/// Returned by `daemon_status`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatus {
    pub running: bool,
    pub port: Option<u16>,
}

/// Open a repository: upsert the repos row, install the integration kit, open
/// timeline + index, start the daemon, kick off cold-start indexing if the repo
/// has never been indexed, and emit `repo:changed`.
#[tauri::command]
pub async fn open_repo(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<RepoState, String> {
    let root = repo_util::canonical(&path)?;
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    if !repo_util::is_git_repo(&root) {
        return Err(format!("{} is not a git repository", root.display()));
    }

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
    // `indexed_at IS NULL` ⇔ never indexed — this decides cold-start below, NOT
    // features_count.
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
    let forwarder_task = tokio::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(event) => {
                    let _ = app_fwd.emit(events::TIMELINE_EVENT, &event);
                }
                Err(RecvError::Lagged(n)) => tracing::warn!("timeline forwarder lagged {n} events"),
                Err(RecvError::Closed) => break,
            }
        }
    });

    let reindexing = Arc::new(AtomicBool::new(false));
    let features_count = index.read().await.features().len() as u32;

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
                reindexing: reindexing.clone(),
            },
        );
    }

    // Never indexed (CONTRACT-3: indexed_at IS NULL) → kick off cold-start
    // (progress + completion arrive as events).
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
    let _ = app.emit(events::REPO_CHANGED, &repo_state);
    Ok(repo_state)
}

/// Parse a stored RFC3339 `indexed_at` into a UTC timestamp.
fn parse_rfc3339(s: &str) -> Option<DateTime<Utc>> {
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

/// Close a repository: tear down every session rooted here (a session can't
/// outlive its repo), shut down its daemon, drop the runtime.
#[tauri::command]
pub async fn close_repo(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = repo_util::canonical(&path)?;
    let runtime = {
        let mut repos = state.repos.lock().await;
        repos.remove(&root)
    };
    let RepoRuntime { daemon, forwarder_task, .. } = runtime.ok_or("repo is not open")?;

    // Fix 4: `session_repos` is read here — stop every session mapped to this
    // repo root, so no sidecar keeps running against a closed repo.
    let session_ids: Vec<String> = {
        let map = state.session_repos.lock().await;
        map.iter().filter(|&(_, r)| r == &root).map(|(id, _)| id.clone()).collect()
    };
    if !session_ids.is_empty() {
        let mut mgr = state.sessions.lock().await;
        for id in &session_ids {
            if let Err(e) = mgr.stop(id).await {
                tracing::warn!(session = %id, "stopping session on repo close failed: {e}");
            }
        }
        let mut map = state.session_repos.lock().await;
        for id in &session_ids {
            map.remove(id);
        }
    }

    forwarder_task.abort();
    daemon.shutdown().await;
    Ok(())
}

/// Re-run the indexer. `force: false` preserves pinned features and merges by
/// slug; `force: true` discards unpinned features first.
#[tauri::command]
pub async fn reindex_repo(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    force: bool,
) -> Result<(), String> {
    let root = repo_util::canonical(&repo_path)?;
    let ctx = {
        let repos = state.repos.lock().await;
        let rt = repos.get(&root).ok_or("repo is not open")?;
        ReindexCtx {
            app,
            db: state.db.clone(),
            repo_root: root.clone(),
            repo_id: rt.repo_id.clone(),
            index: rt.index.clone(),
            timeline: rt.timeline.clone(),
            reindexing: rt.reindexing.clone(),
            daemon_port: rt.daemon.port,
            force,
        }
    };
    indexing::spawn_reindex(ctx)
}

/// Daemon liveness for the status bar.
#[tauri::command]
pub async fn daemon_status(state: State<'_, AppState>, repo_path: String) -> Result<DaemonStatus, String> {
    let root = repo_util::canonical(&repo_path)?;
    let repos = state.repos.lock().await;
    let status = repos
        .get(&root)
        .map(|rt| DaemonStatus { running: true, port: Some(rt.daemon.port) })
        .unwrap_or(DaemonStatus { running: false, port: None });
    Ok(status)
}
