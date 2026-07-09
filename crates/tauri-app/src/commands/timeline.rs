use forge_core::{DiffByFeature, TimelineEvent, TimelineFilter};
use tauri::State;

use crate::runtime::repo_util;
use crate::state::AppState;

/// Query the per-repo timeline, newest first (timeline view + feature detail slice).
#[tauri::command]
pub async fn get_timeline(
    state: State<'_, AppState>,
    repo_path: String,
    filter: TimelineFilter,
) -> Result<Vec<TimelineEvent>, String> {
    let root = repo_util::canonical(&repo_path)?;
    let timeline = state.timeline(&root).await.ok_or("repo is not open")?;
    // rusqlite is sync — keep it off the async runtime thread.
    tokio::task::spawn_blocking(move || timeline.query(&filter))
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| format!("{e}"))
}

/// The pending diff grouped by feature (diff review view).
#[tauri::command]
pub async fn get_diff_by_feature(
    state: State<'_, AppState>,
    repo_path: String,
) -> Result<DiffByFeature, String> {
    let root = repo_util::canonical(&repo_path)?;
    let index = state.index(&root).await.ok_or("repo is not open")?;
    let guard = index.read().await;
    forge_git::diff_by_feature(&root, &guard)
        .await
        .map_err(|e| format!("{e}"))
}
