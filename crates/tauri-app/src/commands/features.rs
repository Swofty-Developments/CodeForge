use forge_core::{Feature, FeaturePatch};
use tauri::State;

use crate::state::AppState;

/// All features of an open repo (sidebar feature tree).
#[tauri::command]
pub async fn get_features(state: State<'_, AppState>, repo_path: String) -> Result<Vec<Feature>, String> {
    let _ = (state, repo_path);
    // IMPLEMENT(agent): state.repos[repo_path].index.read().await.features().to_vec()
    Err("not implemented: get_features".into())
}

/// One feature by slug (feature detail view).
#[tauri::command]
pub async fn get_feature(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
) -> Result<Feature, String> {
    let _ = (state, repo_path, slug);
    // IMPLEMENT(agent): index.get(slug) cloned, Err on unknown slug.
    Err("not implemented: get_feature".into())
}

/// Pin/unpin a feature (pinned features survive re-index verbatim). Appends a
/// FeaturePinned timeline event and persists the index.
#[tauri::command]
pub async fn pin_feature(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
    pinned: bool,
) -> Result<(), String> {
    let _ = (state, repo_path, slug, pinned);
    // IMPLEMENT(agent): index.write().await.pin(...) + save + timeline append.
    Err("not implemented: pin_feature".into())
}

/// Apply a human edit to a feature (implies pinning). Appends a FeatureEdited
/// timeline event, persists, and returns the updated feature.
#[tauri::command]
pub async fn update_feature(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
    patch: FeaturePatch,
) -> Result<Feature, String> {
    let _ = (state, repo_path, slug, patch);
    // IMPLEMENT(agent): index.write().await.apply_patch(...) + save + timeline append.
    Err("not implemented: update_feature".into())
}
