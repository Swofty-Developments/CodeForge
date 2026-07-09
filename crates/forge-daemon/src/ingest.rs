//! Turning raw Claude Code hook payloads into timeline events, plus draining the
//! `forward.sh` spool on daemon start. Shared by the live HTTP handler and the
//! startup replay so both name the "unclassified edit" state identically.

use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;

use forge_core::Actor;
use forge_timeline::NewEvent;

use crate::hooks::parse_hook_event;
use crate::{DaemonDeps, ReindexQueue};

/// Relativize absolute hook paths to the repo root (features store repo-relative
/// paths; hooks send absolute ones). Paths outside the repo are kept as-is.
pub(crate) fn relativize(repo_root: &FsPath, paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .iter()
        .map(|p| {
            p.strip_prefix(repo_root)
                .map(FsPath::to_path_buf)
                .unwrap_or_else(|_| p.clone())
        })
        .collect()
}

/// Turn one raw hook payload into a timeline event, distinguishing three states
/// explicitly rather than collapsing them into "empty slugs":
///
/// - not file-scoped (Stop/SessionStart/Bash) → empty slugs is correct;
/// - file edit that classified to features → tagged normally;
/// - file edit that classified to *nothing* (new/renamed file or stale index) →
///   the event is marked `unclassified: true` and the paths are queued on the
///   [`ReindexQueue`] so a later reindex can resolve the file→feature link.
pub(crate) async fn process_hook(
    deps: &DaemonDeps,
    queue: &Arc<ReindexQueue>,
    payload: serde_json::Value,
) {
    let Some(parsed) = parse_hook_event(&payload) else {
        tracing::debug!(payload = %payload, "ignoring unrecognized hook event");
        return;
    };

    let relative = relativize(&deps.repo_root, &parsed.edited_paths);
    let feature_slugs = deps.index.read().await.classify_paths(&relative);
    let file_scoped = !parsed.edited_paths.is_empty();
    let unclassified = file_scoped && feature_slugs.is_empty();

    let mut event_payload = parsed.payload;
    if unclassified {
        if let Some(obj) = event_payload.as_object_mut() {
            // Named state the timeline UI shows as "edit to an unindexed file —
            // re-index pending".
            obj.insert("unclassified".into(), serde_json::Value::Bool(true));
        }
    }

    let event = NewEvent {
        session_id: parsed.session_id.clone(),
        actor: Actor::Agent,
        kind: parsed.kind,
        feature_slugs,
        payload: event_payload,
    };

    // append (which broadcasts — the live-update path) and the follow-up enqueue
    // both hit rusqlite; run them off the async runtime.
    let timeline = deps.timeline.clone();
    let queue = queue.clone();
    let session_id = parsed.session_id;
    let rel_paths: Vec<String> = relative.iter().map(|p| p.to_string_lossy().into_owned()).collect();
    let joined = tokio::task::spawn_blocking(move || -> Result<(), String> {
        let stored = timeline.append(event).map_err(|e| format!("timeline append failed: {e}"))?;
        if unclassified {
            for path in &rel_paths {
                queue
                    .enqueue(path, stored.id, session_id.as_deref())
                    .map_err(|e| format!("reindex enqueue failed for {path}: {e}"))?;
            }
        }
        Ok(())
    })
    .await;
    match joined {
        Ok(Ok(())) => {}
        Ok(Err(e)) => tracing::warn!("hook event dropped: {e}"),
        Err(e) => tracing::warn!("hook processing task failed: {e}"),
    }
}

/// Replay hook payloads that `forward.sh` spooled to
/// `.featureforge/runtime/spool/` while the daemon was down, then delete each on
/// success. Payloads that aren't valid JSON are quarantined under
/// `spool/rejected/` (the lost-data state is named, not silently dropped).
pub(crate) async fn drain_spool(deps: &DaemonDeps, queue: &Arc<ReindexQueue>) {
    let spool = deps.repo_root.join(".featureforge").join("runtime").join("spool");
    let read = match std::fs::read_dir(&spool) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return,
        Err(e) => {
            tracing::warn!(dir = %spool.display(), "cannot read spool dir: {e}");
            return;
        }
    };
    let mut files: Vec<PathBuf> =
        read.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.is_file()).collect();
    if files.is_empty() {
        return;
    }
    files.sort();
    tracing::info!(count = files.len(), "draining spooled hook payloads");
    for file in files {
        let text = match std::fs::read_to_string(&file) {
            Ok(text) => text,
            Err(e) => {
                tracing::warn!(path = %file.display(), "cannot read spooled payload: {e}");
                continue;
            }
        };
        match serde_json::from_str::<serde_json::Value>(&text) {
            Ok(payload) => {
                process_hook(deps, queue, payload).await;
                if let Err(e) = std::fs::remove_file(&file) {
                    tracing::warn!(path = %file.display(), "drained spooled payload but could not remove it: {e}");
                }
            }
            Err(e) => {
                tracing::warn!(path = %file.display(), "spooled payload is not valid JSON; quarantining: {e}");
                quarantine_spooled(&spool, &file);
            }
        }
    }
}

/// Move an unparseable spool file to `spool/rejected/` so it is preserved for
/// inspection rather than reprocessed forever; delete it only if the move fails.
fn quarantine_spooled(spool: &FsPath, file: &FsPath) {
    let rejected = spool.join("rejected");
    if std::fs::create_dir_all(&rejected).is_ok() {
        if let Some(name) = file.file_name() {
            if std::fs::rename(file, rejected.join(name)).is_ok() {
                return;
            }
        }
    }
    if let Err(e) = std::fs::remove_file(file) {
        tracing::warn!(path = %file.display(), "could not quarantine or remove rejected spool file: {e}");
    }
}
