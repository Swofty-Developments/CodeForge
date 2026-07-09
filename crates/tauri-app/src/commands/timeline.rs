use forge_core::{DiffByFeature, TimelineEvent, TimelineFilter};
use tauri::State;

use crate::state::AppState;

/// Query the per-repo timeline, newest first (timeline view + feature detail slice).
#[tauri::command]
pub async fn get_timeline(
    state: State<'_, AppState>,
    repo_path: String,
    filter: TimelineFilter,
) -> Result<Vec<TimelineEvent>, String> {
    let _ = (state, repo_path, filter);
    // IMPLEMENT(agent): state.repos[repo_path].timeline.query(&filter)
    // (spawn_blocking — rusqlite is sync).
    Err("not implemented: get_timeline".into())
}

/// The pending diff grouped by feature (diff review view).
#[tauri::command]
pub async fn get_diff_by_feature(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<DiffByFeature, String> {
    let _ = (state, repo_path);
    // IMPLEMENT(agent): forge_git::diff_by_feature(repo_root, &*index.read().await)
    Err("not implemented: get_diff_by_feature".into())
}
