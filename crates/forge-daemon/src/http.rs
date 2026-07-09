//! HTTP surface of the per-repo daemon. Route table is the frozen contract:
//!
//! - `POST /hooks/event`          — Claude Code hook payloads (fire-and-forget)
//! - `GET  /api/features`         — all features
//! - `GET  /api/features/{slug}`  — one feature
//! - `GET  /api/timeline`         — query params: `feature`, `since`, `limit`
//! - `GET  /api/health`           — liveness + repo identity
//! - `POST /api/notes`            — `{text, featureSlugs}` → appended Note event

use axum::extract::{Path, Query, State};
use axum::routing::{get, post};
use axum::{Json, Router};
use forge_core::{Feature, TimelineEvent};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::DaemonDeps;

/// Query params for `GET /api/timeline`.
#[derive(Debug, Deserialize)]
pub struct TimelineParams {
    pub feature: Option<String>,
    /// RFC3339 timestamp.
    pub since: Option<String>,
    pub limit: Option<u32>,
}

/// Body for `POST /api/notes`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteBody {
    pub text: String,
    #[serde(default)]
    pub feature_slugs: Vec<String>,
}

/// Build the daemon router (also used by tests without binding a port).
pub fn build_router(deps: DaemonDeps) -> Router {
    Router::new()
        .route("/hooks/event", post(hooks_event))
        .route("/api/features", get(list_features))
        .route("/api/features/{slug}", get(get_feature))
        .route("/api/timeline", get(get_timeline))
        .route("/api/health", get(health))
        .route("/api/notes", post(post_note))
        .layer(CorsLayer::permissive())
        .with_state(deps)
}

/// Receives raw Claude Code hook JSON. Must return 200 fast — parse, append the
/// raw TimelineEvent, then classify + broadcast asynchronously.
async fn hooks_event(State(deps): State<DaemonDeps>, Json(payload): Json<serde_json::Value>) {
    let _ = (deps, payload);
    // IMPLEMENT(agent): map hook_event_name (PostToolUse/Stop/SessionStart) to
    // EventKind, append NewEvent, spawn async classification of touched paths.
    todo!("daemon hooks_event")
}

async fn list_features(State(deps): State<DaemonDeps>) -> Json<Vec<Feature>> {
    let _ = deps;
    // IMPLEMENT(agent): deps.index.read().await.features().to_vec()
    todo!("daemon list_features")
}

async fn get_feature(
    State(deps): State<DaemonDeps>,
    Path(slug): Path<String>,
) -> Result<Json<Feature>, axum::http::StatusCode> {
    let _ = (deps, slug);
    // IMPLEMENT(agent): 404 on unknown slug.
    todo!("daemon get_feature")
}

async fn get_timeline(
    State(deps): State<DaemonDeps>,
    Query(params): Query<TimelineParams>,
) -> Json<Vec<TimelineEvent>> {
    let _ = (deps, params);
    // IMPLEMENT(agent): map params into forge_core::TimelineFilter, query store.
    todo!("daemon get_timeline")
}

async fn health(State(deps): State<DaemonDeps>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "ok": true,
        "repo": deps.repo_root,
        "pid": std::process::id(),
    }))
}

async fn post_note(State(deps): State<DaemonDeps>, Json(body): Json<NoteBody>) -> Json<TimelineEvent> {
    let _ = (deps, body);
    // IMPLEMENT(agent): append Note event (actor Agent when session header
    // present, else System) and return it.
    todo!("daemon post_note")
}
