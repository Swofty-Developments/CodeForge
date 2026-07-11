//! Index staleness/version status (FZ-2): compute the pure on-disk verdict off
//! the async runtime, and push it to the frontend as an `index:status` event so
//! the UI can prompt a re-index. Never re-indexes — the UI decides.
//!
//! A background poller per open repo re-checks every [`POLL_INTERVAL`] and
//! emits only when the verdict changes, so edits made outside our hooks (e.g.
//! `git pull`, an external editor) surface on screen without reopening the repo.

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use forge_index::IndexStatus;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

/// How often the background poller re-hashes the manifest against
/// `.codeforge/index-meta.json`.
const POLL_INTERVAL: Duration = Duration::from_secs(10);

/// Tauri event carrying an index staleness verdict for one repo (FZ-2). Payload
/// is [`IndexStatusPayload`] (`{ repoPath, status }`, camelCase). The frontend
/// filters by `repoPath` and shows the re-index modal.
const INDEX_STATUS_EVENT: &str = "index:status";

/// `index:status` payload: the emitting repo's canonical path plus the verdict.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct IndexStatusPayload<'a> {
    repo_path: &'a Path,
    status: &'a IndexStatus,
}

/// Compute the on-disk index status for `repo_root` off the async runtime
/// (sha256 hashing is blocking IO). Pure — never spawns `claude`, never
/// re-indexes. Surfaces both the join error and the index error to the caller.
pub async fn compute(repo_root: PathBuf) -> Result<IndexStatus, String> {
    tokio::task::spawn_blocking(move || forge_index::index_status(&repo_root))
        .await
        .map_err(|e| format!("index_status task join error: {e}"))?
        .map_err(|e| format!("{e}"))
}

/// Spawn the per-repo staleness poller: check immediately on open, then every
/// [`POLL_INTERVAL`], emitting `index:status` `{ repoPath, status }` only when
/// the verdict CHANGED since the last emit — a steady state (fresh or already-
/// reported stale) stays silent so the UI isn't re-prompted every tick.
///
/// While `reindexing` is set the poll is skipped entirely: mid-reindex the
/// manifest is being rewritten, so a verdict against the old meta is noise.
/// `last` is also reset so the first post-reindex verdict is always re-emitted
/// (normally back to `fresh`, clearing the status-bar dot).
///
/// A probe failure is a named error branch: the repo IS open, so we log and
/// keep polling. The UI can still call the `index_status` command, which
/// surfaces the same error to its caller. There is no fabricated
/// "fresh"/"never" substitute (IndexStatus has no error state).
///
/// The returned handle is owned by the `RepoRuntime` and aborted on context
/// close — the poller must not outlive its repo.
pub fn spawn_poller(
    app: AppHandle,
    repo_root: PathBuf,
    reindexing: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut last: Option<IndexStatus> = None;
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        // First tick fires immediately (covers the on-open push); if a slow
        // hash overruns the interval, just resume the cadence — don't burst.
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
        loop {
            ticker.tick().await;
            if reindexing.load(Ordering::SeqCst) {
                last = None;
                continue;
            }
            match compute(repo_root.clone()).await {
                Ok(status) => {
                    if last.as_ref() != Some(&status) {
                        let _ = app.emit(
                            INDEX_STATUS_EVENT,
                            IndexStatusPayload { repo_path: &repo_root, status: &status },
                        );
                        last = Some(status);
                    }
                }
                Err(e) => {
                    tracing::error!(repo = %repo_root.display(), "index status probe failed: {e}");
                }
            }
        }
    })
}
