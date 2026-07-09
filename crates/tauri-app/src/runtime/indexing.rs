//! Re-index orchestration: run the headless indexer, merge (pinned survive),
//! save, write docs, bookend with `IndexStarted`/`IndexCompleted` timeline
//! events, and stream `index:progress`. One reindex per repo at a time.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use forge_core::{Actor, EventKind, IndexProgress};
use forge_index::{FeatureIndex, Indexer};
use forge_timeline::{NewEvent, TimelineStore};
use tauri::{AppHandle, Emitter};
use tokio::sync::{mpsc, RwLock};

use crate::db::Database;
use crate::runtime::repo_util;
use crate::{events, queries};

/// Resets the per-repo "reindex in progress" flag on drop (covers task panic).
struct ReindexGuard(Arc<AtomicBool>);

impl Drop for ReindexGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::SeqCst);
    }
}

/// Deps a reindex task owns (cloned out of `RepoRuntime` so no state lock is held).
pub struct ReindexCtx {
    pub app: AppHandle,
    pub db: Arc<Mutex<Database>>,
    pub repo_root: PathBuf,
    pub repo_id: String,
    pub index: Arc<RwLock<FeatureIndex>>,
    pub timeline: Arc<TimelineStore>,
    pub reindexing: Arc<AtomicBool>,
    pub daemon_port: u16,
    pub force: bool,
}

/// Claim the reindex slot and spawn the task. `Err` when one is already running.
pub fn spawn_reindex(ctx: ReindexCtx) -> Result<(), String> {
    if ctx.reindexing.swap(true, Ordering::SeqCst) {
        return Err("reindex already in progress for this repo".into());
    }
    tokio::spawn(async move {
        let _guard = ReindexGuard(ctx.reindexing.clone());
        run(ctx).await;
    });
    Ok(())
}

async fn run(ctx: ReindexCtx) {
    append(&ctx.timeline, EventKind::IndexStarted, serde_json::json!({ "force": ctx.force }));

    let (progress_tx, mut progress_rx) = mpsc::channel::<IndexProgress>(64);
    let app_prog = ctx.app.clone();
    let progress_task = tokio::spawn(async move {
        while let Some(p) = progress_rx.recv().await {
            let _ = app_prog.emit(events::INDEX_PROGRESS, &p);
        }
    });

    let features = match Indexer::cold_start(&ctx.repo_root, progress_tx.clone()).await {
        Ok(features) => features,
        Err(e) => {
            tracing::error!("cold_start failed: {e}");
            append(&ctx.timeline, EventKind::IndexCompleted, serde_json::json!({ "error": e.to_string() }));
            drop(progress_tx);
            let _ = progress_task.await;
            return;
        }
    };

    {
        let mut index = ctx.index.write().await;
        index.merge_reindex(features.clone());
        if let Err(e) = index.save() {
            tracing::error!("failed to save feature index: {e}");
        }
    }

    if let Err(e) = Indexer::write_feature_docs(&ctx.repo_root, &features, progress_tx.clone()).await {
        tracing::warn!("write_feature_docs failed: {e}");
    }
    drop(progress_tx);
    let _ = progress_task.await;

    persist_indexed_at(&ctx.db, &ctx.repo_id).await;

    let count = ctx.index.read().await.features().len() as u32;
    append(&ctx.timeline, EventKind::IndexCompleted, serde_json::json!({ "features": count }));

    let state = repo_util::repo_state(&ctx.repo_root, count, Some(ctx.daemon_port));
    let _ = ctx.app.emit(events::REPO_CHANGED, &state);
}

async fn persist_indexed_at(db: &Arc<Mutex<Database>>, repo_id: &str) {
    let db = db.clone();
    let repo_id = repo_id.to_string();
    let now = chrono::Utc::now().to_rfc3339();
    let joined = tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        queries::set_repo_indexed_at(db.conn(), &repo_id, &now)
    })
    .await;
    match joined {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("failed to record indexed_at: {e}"),
        Err(e) => tracing::warn!("indexed_at task join error: {e}"),
    }
}

fn append(timeline: &TimelineStore, kind: EventKind, payload: serde_json::Value) {
    let event = NewEvent {
        session_id: None,
        actor: Actor::System,
        kind,
        feature_slugs: Vec::new(),
        payload,
    };
    if let Err(e) = timeline.append(event) {
        tracing::warn!("timeline append failed: {e}");
    }
}
