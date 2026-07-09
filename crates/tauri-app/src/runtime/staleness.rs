//! Index staleness/version status (FZ-2): compute the pure on-disk verdict off
//! the async runtime, and push it to the frontend as an `index:status` event so
//! the UI can prompt a re-index. Never re-indexes — the UI decides.

use std::path::{Path, PathBuf};

use forge_index::IndexStatus;
use serde::Serialize;
use tauri::{AppHandle, Emitter};

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

/// Compute the status for `repo_root` and emit `index:status` `{ repoPath,
/// status }`. Used on context open (FZ-2).
///
/// A probe failure is a named error branch: the repo IS open, so we must not
/// fail the open — we log it and skip the push. The UI can still call the
/// `index_status` command, which surfaces the same error to its caller. There is
/// no fabricated "fresh"/"never" substitute (IndexStatus has no error state).
pub async fn emit_status(app: &AppHandle, repo_root: &Path) {
    match compute(repo_root.to_path_buf()).await {
        Ok(status) => {
            let _ = app.emit(
                INDEX_STATUS_EVENT,
                IndexStatusPayload { repo_path: repo_root, status: &status },
            );
        }
        Err(e) => {
            tracing::error!(repo = %repo_root.display(), "index status probe failed: {e}");
        }
    }
}
