use forge_core::{Actor, EventKind, Feature, FeaturePatch};
use forge_timeline::NewEvent;
use tauri::State;

use crate::runtime::{feature_color, repo_util};
use crate::state::AppState;

/// All features of an open repo (sidebar feature tree).
#[tauri::command]
pub async fn get_features(state: State<'_, AppState>, repo_path: String) -> Result<Vec<Feature>, String> {
    let root = repo_util::canonical(&repo_path)?;
    let index = state.index(&root).await.ok_or("repo is not open")?;
    let features = index.read().await.features().to_vec();
    Ok(features)
}

/// One feature by slug (feature detail view).
#[tauri::command]
pub async fn get_feature(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
) -> Result<Feature, String> {
    let root = repo_util::canonical(&repo_path)?;
    let index = state.index(&root).await.ok_or("repo is not open")?;
    let guard = index.read().await;
    guard
        .get(&slug)
        .cloned()
        .ok_or_else(|| format!("unknown feature: {slug}"))
}

/// The living doc markdown for a feature (`.featureforge/docs/<slug>.md`), or
/// `None` if it has not been generated yet. Feature detail renders this above
/// the description.
#[tauri::command]
pub async fn get_feature_doc(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
) -> Result<Option<String>, String> {
    let root = repo_util::canonical(&repo_path)?;
    let index = state.index(&root).await.ok_or("repo is not open")?;
    let guard = index.read().await;
    guard.read_doc(&slug).map_err(|e| format!("{e}"))
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
    let root = repo_util::canonical(&repo_path)?;
    let (index, timeline) = state.index_and_timeline(&root).await.ok_or("repo is not open")?;

    {
        let mut guard = index.write().await;
        guard.pin(&slug, pinned).map_err(|e| format!("{e}"))?;
        guard.save().map_err(|e| format!("{e}"))?;
    }

    let event = NewEvent {
        session_id: None,
        actor: Actor::Human,
        kind: EventKind::FeaturePinned,
        feature_slugs: vec![slug],
        payload: serde_json::json!({ "pinned": pinned }),
    };
    // The timeline is the durable audit log; a lost append is a real failure,
    // surfaced to the caller rather than reduced to a log line.
    timeline
        .append(event)
        .map_err(|e| format!("pin persisted but timeline append failed: {e}"))?;
    Ok(())
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
    let root = repo_util::canonical(&repo_path)?;
    let (index, timeline) = state.index_and_timeline(&root).await.ok_or("repo is not open")?;

    let feature = {
        let mut guard = index.write().await;
        let feature = guard.apply_patch(&slug, patch).map_err(|e| format!("{e}"))?;
        guard.save().map_err(|e| format!("{e}"))?;
        feature
    };

    let event = NewEvent {
        session_id: None,
        actor: Actor::Human,
        kind: EventKind::FeatureEdited,
        feature_slugs: vec![slug],
        payload: serde_json::json!({ "name": feature.name, "tags": feature.tags }),
    };
    // The timeline is the durable audit log; a lost append is a real failure,
    // surfaced to the caller rather than reduced to a log line.
    timeline
        .append(event)
        .map_err(|e| format!("edit persisted but timeline append failed: {e}"))?;
    Ok(feature)
}

/// Set (or clear, with `color: null`) a feature's graph node color. Like
/// `update_feature` this implies a pin, persists the index, appends a
/// FeatureEdited timeline event, and returns the updated feature.
#[tauri::command]
pub async fn set_feature_color(
    state: State<'_, AppState>,
    repo_path: String,
    slug: String,
    color: Option<String>,
) -> Result<Feature, String> {
    let root = repo_util::canonical(&repo_path)?;
    let (index, timeline) = state.index_and_timeline(&root).await.ok_or("repo is not open")?;

    let feature = {
        let mut guard = index.write().await;
        feature_color::set_color(&mut guard, &slug, color)?
    };

    let event = NewEvent {
        session_id: None,
        actor: Actor::Human,
        kind: EventKind::FeatureEdited,
        feature_slugs: vec![slug],
        payload: serde_json::json!({ "color": feature.color }),
    };
    // The timeline is the durable audit log; a lost append is a real failure,
    // surfaced to the caller rather than reduced to a log line.
    timeline
        .append(event)
        .map_err(|e| format!("color persisted but timeline append failed: {e}"))?;
    Ok(feature)
}
