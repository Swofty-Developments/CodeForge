use forge_core::RepoState;
use serde::Serialize;
use tauri::State;

use crate::runtime::indexing::{self, ReindexCtx};
use crate::runtime::{repo_open, repo_util};
use crate::state::AppState;

/// Returned by `daemon_status`.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DaemonStatus {
    pub running: bool,
    pub port: Option<u16>,
}

/// Open a repository as a context: install the integration kit, open timeline +
/// index, start the daemon, kick off cold-start indexing if it has never been
/// indexed, and emit `repo:changed`. A worktree opens through the SAME core
/// (`repo_open::open_context`, contract W1). This command only validates the
/// path first; a worktree's `.git` is a file, which `is_git_repo` accepts.
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
        // Machine-matchable prefix: the frontend offers `init_repo` on exactly this.
        return Err(format!("not_a_git_repo: {}", root.display()));
    }
    repo_open::open_context(app, &state, root).await
}

/// `git init -b main` in a plain folder, then open it through the SAME flow as
/// `open_repo`. Offered by the UI when `open_repo` rejects with the
/// "not_a_git_repo:" prefix. An already-initialized repo is a named error —
/// the caller should open it instead.
#[tauri::command]
pub async fn init_repo(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    path: String,
) -> Result<RepoState, String> {
    let root = repo_util::canonical(&path)?;
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()));
    }
    if repo_util::is_git_repo(&root) {
        return Err(format!("{} is already a git repository", root.display()));
    }
    forge_git::init_repo(&root).await.map_err(|e| e.to_string())?;
    repo_open::open_context(app, &state, root).await
}

/// Close a repository context: tear down every session rooted here (a session
/// can't outlive its repo), shut down its daemon, drop the runtime. An
/// already-closed repo is a named error.
#[tauri::command]
pub async fn close_repo(state: State<'_, AppState>, path: String) -> Result<(), String> {
    let root = repo_util::canonical(&path)?;
    if !repo_open::close_context(&state, &root).await? {
        return Err("repo is not open".into());
    }
    Ok(())
}

/// Re-run the indexer. `force: false` takes the incremental path when the
/// index is merely stale (refresh affected features only, no decomposition),
/// falling back to a full cold start; `force: true` always cold-starts.
/// Both merge by slug and preserve pinned features.
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
