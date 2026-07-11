//! Re-index orchestration: run the headless indexer, merge (pinned survive),
//! save, write docs, bookend with `IndexStarted`/`IndexCompleted` timeline
//! events, and stream `index:progress`. One reindex per repo at a time.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use forge_core::{Actor, EventKind, IndexProgress};
use forge_index::{FeatureIndex, IndexState, Indexer};
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

    // Every stage-level failure funnels here as a named state; only the fully
    // successful path below writes indexed_at and emits a clean IndexCompleted.
    let outcome = run_indexed(&ctx, &progress_tx).await;
    drop(progress_tx);
    let _ = progress_task.await;

    match outcome {
        Ok(Completion { report, indexed_at, incremental }) => {
            let count = ctx.index.read().await.features().len() as u32;
            append(
                &ctx.timeline,
                EventKind::IndexCompleted,
                serde_json::json!({
                    "features": count,
                    "docsWritten": report.written,
                    "docsFailed": report.failed.len(),
                    "incremental": incremental,
                }),
            );
            let mut state = repo_util::repo_state(&ctx.repo_root, count, Some(ctx.daemon_port));
            // repo_util no longer derives indexed_at (the DB owns it); stamp the
            // timestamp we just recorded so the event carries the real value.
            state.indexed_at = Some(indexed_at);
            state.branch = match forge_git::current_branch(&ctx.repo_root).await {
                Ok(b) => b,
                Err(e) => {
                    tracing::warn!("could not read git branch for repo:changed: {e}");
                    None
                }
            };
            state.project = match repo_util::project_name(&ctx.repo_root).await {
                Ok(p) => Some(p),
                Err(e) => {
                    tracing::warn!("could not resolve project name for repo:changed: {e}");
                    None
                }
            };
            let _ = ctx.app.emit(events::REPO_CHANGED, &state);
        }
        Err(e) => {
            // The reindex did NOT cleanly complete. Leave the existing index +
            // indexed_at untouched, record the failure on the timeline, and push
            // an explicit error stage to the UI (never a clean IndexCompleted).
            tracing::error!("reindex failed: {e}");
            append(&ctx.timeline, EventKind::IndexCompleted, serde_json::json!({ "error": e }));
            emit_error(&ctx.app, &e);
        }
    }
}

/// The reindex happy path. Any `Err` short-circuits before `indexed_at` is
/// written and before a clean completion is emitted, so a failed decomposition,
/// a failed index save, or a failed `indexed_at` write can never masquerade as
/// success. The existing on-disk index survives a failed reindex verbatim.
///
/// `force: false` first attempts the incremental path (stale index, changed
/// files attributable to existing features → prune + refresh only those docs,
/// no LLM decomposition); `force: true` — and any case incremental can't
/// handle — runs the full cold start.
async fn run_indexed(
    ctx: &ReindexCtx,
    progress_tx: &mpsc::Sender<IndexProgress>,
) -> Result<Completion, String> {
    if !ctx.force {
        if let Some(done) = try_incremental(ctx, progress_tx).await? {
            return Ok(done);
        }
    }

    let features = Indexer::cold_start(&ctx.repo_root, progress_tx.clone())
        .await
        .map_err(|e| e.to_string())?;

    // Merge (pinned survive) then persist. A failed save means the reindex is not
    // durable — surface it rather than proceeding to a clean completion. The
    // content-hash manifest (FZ-2) is rewritten over the MERGED set so a fresh
    // index reports `fresh`; a failed meta write is a non-durable reindex too.
    {
        let mut index = ctx.index.write().await;
        index.merge_reindex(features.clone());
        index
            .save()
            .map_err(|e| format!("failed to save feature index: {e}"))?;
        index
            .write_meta()
            .map_err(|e| format!("failed to write index meta: {e}"))?;
    }

    let report = Indexer::write_feature_docs(&ctx.repo_root, &features, progress_tx.clone())
        .await
        .map_err(|e| format!("doc pass failed: {e}"))?;
    if !report.failed.is_empty() {
        tracing::warn!(failed = ?report.failed, "some feature docs failed to generate");
    }

    // CONTRACT-3: recording indexed_at is a mandatory completion step. If it
    // fails the repo has not been cleanly indexed — treat it as a failure.
    let indexed_at = persist_indexed_at(&ctx.db, &ctx.repo_id)
        .await
        .map_err(|e| format!("failed to record indexed_at: {e}"))?;

    Ok(Completion { report, indexed_at, incremental: false })
}

/// The incremental path: when the index is merely *stale* (manifest files
/// changed or vanished) and every changed file belongs to an existing feature,
/// skip the LLM decomposition entirely — prune dead paths, rewrite the
/// manifest, and refresh only the affected features' docs. Returns `Ok(None)`
/// when a full cold start is required instead (index `never`/`outdated`/
/// `fresh`, or changed files no feature owns — new territory needs
/// decomposition to discover features).
async fn try_incremental(
    ctx: &ReindexCtx,
    progress_tx: &mpsc::Sender<IndexProgress>,
) -> Result<Option<Completion>, String> {
    // index_status re-hashes the manifest — blocking IO off the async thread.
    let root = ctx.repo_root.clone();
    let status = tokio::task::spawn_blocking(move || forge_index::index_status(&root))
        .await
        .map_err(|e| format!("index status task join error: {e}"))?
        .map_err(|e| e.to_string())?;
    if status.state != IndexState::Stale {
        return Ok(None);
    }

    let affected: Vec<forge_core::Feature> = {
        let mut index = ctx.index.write().await;
        let slugs = index.owners_of(&status.changed_files);
        if slugs.is_empty() {
            return Ok(None);
        }
        let removed = index.prune_missing_files(&slugs);
        if !removed.is_empty() {
            tracing::info!(?removed, "incremental reindex removed features with no surviving files");
        }
        index
            .save()
            .map_err(|e| format!("failed to save feature index: {e}"))?;
        index
            .write_meta()
            .map_err(|e| format!("failed to write index meta: {e}"))?;
        slugs
            .iter()
            .filter(|s| !removed.contains(s))
            .filter_map(|s| index.get(s).cloned())
            .collect()
    };

    let _ = progress_tx
        .send(IndexProgress {
            stage: "incremental".to_string(),
            detail: format!(
                "{} changed file{} → refreshing {} feature{}",
                status.changed_files.len(),
                if status.changed_files.len() == 1 { "" } else { "s" },
                affected.len(),
                if affected.len() == 1 { "" } else { "s" },
            ),
            done: 0,
            total: affected.len() as u32,
        })
        .await;

    let report = Indexer::write_feature_docs(&ctx.repo_root, &affected, progress_tx.clone())
        .await
        .map_err(|e| format!("incremental doc pass failed: {e}"))?;
    if !report.failed.is_empty() {
        tracing::warn!(failed = ?report.failed, "some feature docs failed to refresh incrementally");
    }

    let indexed_at = persist_indexed_at(&ctx.db, &ctx.repo_id)
        .await
        .map_err(|e| format!("failed to record indexed_at: {e}"))?;

    Ok(Some(Completion { report, indexed_at, incremental: true }))
}

/// The successful reindex result: the doc report plus the `indexed_at` that was
/// just written to the DB (the single source of truth, carried through so the
/// `repo:changed` event never reports a null index time right after indexing).
struct Completion {
    report: forge_index::DocReport,
    indexed_at: chrono::DateTime<chrono::Utc>,
    /// True when the cheap stale-only path ran (no LLM decomposition).
    incremental: bool,
}

/// Push an explicit `error` stage on `index:progress` so the UI shows the
/// failure instead of silently staying on the last good stage.
fn emit_error(app: &AppHandle, detail: &str) {
    let _ = app.emit(
        events::INDEX_PROGRESS,
        &IndexProgress {
            stage: "error".to_string(),
            detail: detail.to_string(),
            done: 0,
            total: 0,
        },
    );
}

/// Write `indexed_at = now` for the repo and return the timestamp written.
async fn persist_indexed_at(
    db: &Arc<Mutex<Database>>,
    repo_id: &str,
) -> anyhow::Result<chrono::DateTime<chrono::Utc>> {
    let db = db.clone();
    let repo_id = repo_id.to_string();
    let now = chrono::Utc::now();
    let now_str = now.to_rfc3339();
    tokio::task::spawn_blocking(move || -> anyhow::Result<()> {
        let db = db.lock().map_err(|_| anyhow::anyhow!("app db mutex poisoned"))?;
        queries::set_repo_indexed_at(db.conn(), &repo_id, &now_str)
    })
    .await
    .map_err(|e| anyhow::anyhow!("indexed_at task join error: {e}"))??;
    Ok(now)
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
