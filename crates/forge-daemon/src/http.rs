//! HTTP surface of the per-repo daemon. Route table is the frozen contract:
//!
//! - `POST /hooks/event`          — Claude Code hook payloads (fire-and-forget)
//! - `GET  /api/features`         — all features
//! - `GET  /api/features/{slug}`  — one feature
//! - `GET  /api/timeline`         — query params: `feature`, `since`, `limit`
//! - `GET  /api/health`           — liveness + repo identity
//! - `POST /api/notes`            — `{text, featureSlugs}` → appended Note event
//! - `GET  /api/classify`         — `?path=` → feature slugs (serves forge-mcp `which_features`)

use std::path::PathBuf;
use std::sync::Arc;

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::{DateTime, Utc};
use forge_core::{Actor, EventKind, Feature, TimelineEvent, TimelineFilter};
use forge_timeline::{NewEvent, TimelineStore};
use serde::Deserialize;
use tower_http::cors::CorsLayer;

use crate::{ingest, DaemonDeps, ReindexQueue};

/// Router state: shared deps + runtime identity for `/api/health`.
#[derive(Clone)]
struct AppState {
    deps: DaemonDeps,
    port: u16,
    started_at: DateTime<Utc>,
    /// Durable queue of edits that classified to no feature (see [`ReindexQueue`]).
    reindex_queue: Arc<ReindexQueue>,
}

/// Query params for `GET /api/timeline`.
#[derive(Debug, Deserialize)]
pub struct TimelineParams {
    pub feature: Option<String>,
    /// RFC3339 timestamp.
    pub since: Option<String>,
    pub limit: Option<u32>,
}

/// Query params for `GET /api/classify`.
#[derive(Debug, Deserialize)]
pub struct ClassifyParams {
    pub path: String,
}

/// Body for `POST /api/notes`.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NoteBody {
    pub text: String,
    #[serde(default)]
    pub feature_slugs: Vec<String>,
}

/// Build the daemon router (also used by tests without binding a port). The
/// caller supplies the [`ReindexQueue`] so it can be shared with spool draining.
pub fn build_router(deps: DaemonDeps, reindex_queue: Arc<ReindexQueue>) -> Router {
    build_router_with_info(deps, 0, Utc::now(), reindex_queue)
}

/// Same router, carrying the bound port + start time for `/api/health`.
pub(crate) fn build_router_with_info(
    deps: DaemonDeps,
    port: u16,
    started_at: DateTime<Utc>,
    reindex_queue: Arc<ReindexQueue>,
) -> Router {
    let state = AppState {
        deps,
        port,
        started_at,
        reindex_queue,
    };
    Router::new()
        .route("/hooks/event", post(hooks_event))
        .route("/api/features", get(list_features))
        .route("/api/features/{slug}", get(get_feature))
        .route("/api/timeline", get(get_timeline))
        .route("/api/health", get(health))
        .route("/api/notes", post(post_note))
        .route("/api/classify", get(classify))
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// Append an event on the blocking pool (rusqlite behind a sync Mutex).
async fn append_event(
    store: Arc<TimelineStore>,
    event: NewEvent,
) -> Result<TimelineEvent, StatusCode> {
    tokio::task::spawn_blocking(move || store.append(event))
        .await
        .map_err(|e| {
            tracing::error!("timeline append task failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map_err(|e| {
            tracing::error!("timeline append failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

/// Classify paths against the index, relativized to the repo root.
async fn classify_paths(deps: &DaemonDeps, paths: &[PathBuf]) -> Vec<String> {
    let relative = ingest::relativize(&deps.repo_root, paths);
    deps.index.read().await.classify_paths(&relative)
}

/// Receives raw Claude Code hook JSON. Always replies 200 `{}` fast; the hook
/// script must never block Claude. Unrecognized events are logged at debug.
async fn hooks_event(
    State(state): State<AppState>,
    Json(payload): Json<serde_json::Value>,
) -> Json<serde_json::Value> {
    ingest::process_hook(&state.deps, &state.reindex_queue, payload).await;
    Json(serde_json::json!({}))
}

async fn list_features(State(state): State<AppState>) -> Json<Vec<Feature>> {
    Json(state.deps.index.read().await.features().to_vec())
}

async fn get_feature(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<Feature>, StatusCode> {
    state
        .deps
        .index
        .read()
        .await
        .get(&slug)
        .cloned()
        .map(Json)
        .ok_or(StatusCode::NOT_FOUND)
}

async fn get_timeline(
    State(state): State<AppState>,
    Query(params): Query<TimelineParams>,
) -> Result<Json<Vec<TimelineEvent>>, StatusCode> {
    let since = match params.since.as_deref() {
        Some(s) => Some(
            DateTime::parse_from_rfc3339(s)
                .map(|dt| dt.with_timezone(&Utc))
                .map_err(|_| StatusCode::BAD_REQUEST)?,
        ),
        None => None,
    };
    let filter = TimelineFilter {
        feature_slug: params.feature,
        actor: None,
        kinds: None,
        since,
        limit: params.limit,
    };
    let store = state.deps.timeline.clone();
    tokio::task::spawn_blocking(move || store.query(&filter))
        .await
        .map_err(|e| {
            tracing::error!("timeline query task failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?
        .map(Json)
        .map_err(|e| {
            tracing::error!("timeline query failed: {e}");
            StatusCode::INTERNAL_SERVER_ERROR
        })
}

async fn health(State(state): State<AppState>) -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "repo": state.deps.repo_root,
        "port": state.port,
        "uptime": (Utc::now() - state.started_at).num_seconds(),
    }))
}

async fn post_note(
    State(state): State<AppState>,
    Json(body): Json<NoteBody>,
) -> Result<Json<TimelineEvent>, StatusCode> {
    let event = NewEvent {
        session_id: None,
        actor: Actor::Agent,
        kind: EventKind::Note,
        feature_slugs: body.feature_slugs,
        payload: serde_json::json!({ "text": body.text }),
    };
    append_event(state.deps.timeline.clone(), event).await.map(Json)
}

async fn classify(
    State(state): State<AppState>,
    Query(params): Query<ClassifyParams>,
) -> Json<serde_json::Value> {
    let slugs = classify_paths(&state.deps, &[PathBuf::from(&params.path)]).await;
    Json(serde_json::json!({ "path": params.path, "featureSlugs": slugs }))
}
