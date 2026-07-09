use forge_core::RepoState;
use serde::Serialize;
use tauri::State;

use crate::state::AppState;

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
    let _ = (app, state, path);
    // IMPLEMENT(agent): canonicalize path; Database upsert_repo (spawn_blocking);
    // forge_daemon::install_kit; TimelineStore::open + FeatureIndex::load;
    // Daemon::start -> RepoRuntime into state.repos; spawn Indexer::cold_start
    // when features.json is empty (forward IndexProgress to events::INDEX_PROGRESS);
    // emit events::REPO_CHANGED; return RepoState.
    Err("not implemented: open_repo".into())
}

/// Close a repository: shut down its daemon, drop the runtime, emit `repo:changed`.
#[tauri::command]
pub async fn close_repo(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let _ = (state, path);
    // IMPLEMENT(agent): remove from state.repos, DaemonHandle::shutdown().
    Err("not implemented: close_repo".into())
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
    let _ = (app, state, repo_path, force);
    // IMPLEMENT(agent): Indexer::cold_start + merge into FeatureIndex + save +
    // write_feature_docs; stream index:progress; append IndexStarted/IndexCompleted
    // timeline events.
    Err("not implemented: reindex_repo".into())
}

/// Daemon liveness for the status bar.
#[tauri::command]
pub async fn daemon_status(state: State<'_, AppState>, repo_path: String) -> Result<DaemonStatus, String> {
    let repos = state.repos.lock().await;
    let status = repos
        .get(std::path::Path::new(&repo_path))
        .map(|rt| DaemonStatus { running: true, port: Some(rt.daemon.port) })
        .unwrap_or(DaemonStatus { running: false, port: None });
    Ok(status)
}
