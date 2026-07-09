//! Worktree IPC commands (contract W3). A worktree is opened as its OWN repo
//! context (contract W1): `create_worktree` reuses the same
//! `repo_open::open_context` core as `open_repo`, and `remove_worktree` reuses
//! `close_context`. The git primitives come from `forge_git` (worktree ops in forge-git).
//! stand-in for `forge_git`; see that module) and the CodeForge-specific
//! setup from `worktree_fs`.

use forge_core::{Actor, EventKind, MergeResult, Worktree};
use forge_timeline::NewEvent;
use tauri::State;

use crate::runtime::{repo_open, repo_util, worktree_fs};
use crate::state::AppState;

/// All worktrees of the repo `repo_path` belongs to, base first. Derives the
/// base first (contract W4: `repo_path` may itself be a worktree).
#[tauri::command]
pub async fn list_worktrees(repo_path: String) -> Result<Vec<Worktree>, String> {
    let root = repo_util::canonical(&repo_path)?;
    let base = forge_git::base_repo_root(&root).await.map_err(|e| format!("{e}"))?;
    forge_git::list_worktrees(&base).await.map_err(|e| format!("{e}"))
}

/// Create a worktree on a new branch `name` off `base_ref` (default: the base
/// repo's current HEAD), have it inherit the base's feature model, keep
/// `.codeforge-worktrees/` ignored, then open it as its own context.
#[tauri::command]
pub async fn create_worktree(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    repo_path: String,
    name: String,
    base_ref: Option<String>,
) -> Result<Worktree, String> {
    let root = repo_util::canonical(&repo_path)?;
    let base = forge_git::base_repo_root(&root).await.map_err(|e| format!("{e}"))?;

    let worktree = forge_git::create_worktree(&base, &name, base_ref.as_deref())
        .await
        .map_err(|e| format!("{e}"))?;

    // Inherit features immediately (a base with no features.json yet is a real
    // "not indexed" state, not an error), and keep the worktrees dir ignored.
    worktree_fs::inherit_features(&base, &worktree.path)?;
    worktree_fs::ensure_worktrees_ignored(&base)?;

    // Open the worktree as its own RepoRuntime (own daemon / index / timeline).
    repo_open::open_context(app, &state, worktree.path.clone()).await?;
    Ok(worktree)
}

/// Remove a worktree: close its context if open (graceful daemon shutdown + stop
/// its sessions), then remove the git worktree. `force` removes even a dirty
/// worktree.
#[tauri::command]
pub async fn remove_worktree(
    state: State<'_, AppState>,
    repo_path: String,
    worktree_path: String,
    force: bool,
) -> Result<(), String> {
    let root = repo_util::canonical(&repo_path)?;
    let base = forge_git::base_repo_root(&root).await.map_err(|e| format!("{e}"))?;
    let worktree_root = repo_util::canonical(&worktree_path)?;

    // Close the context first (no-op if it was never open) so no daemon/session
    // is left running against a path that is about to disappear.
    repo_open::close_context(&state, &worktree_root).await?;
    forge_git::remove_worktree(&base, &worktree_root, force)
        .await
        .map_err(|e| format!("{e}"))
}

/// Merge a worktree's branch into the base repo's current branch, then record a
/// System note on the BASE context's timeline summarizing the outcome. The base
/// must be open (its timeline is the audit target) — a named precondition.
#[tauri::command]
pub async fn merge_worktree(
    state: State<'_, AppState>,
    base_repo_path: String,
    worktree_path: String,
) -> Result<MergeResult, String> {
    let base = repo_util::canonical(&base_repo_path)?;
    let worktree_root = repo_util::canonical(&worktree_path)?;

    // Precondition: the base context is open, so the merge note lands on its live
    // timeline (and reaches the UI via the existing forwarder).
    let timeline = state.timeline(&base).await.ok_or("base repo is not open")?;

    let result = forge_git::merge_worktree(&base, &worktree_root)
        .await
        .map_err(|e| format!("{e}"))?;

    let event = NewEvent {
        session_id: None,
        actor: Actor::System,
        kind: EventKind::Note,
        feature_slugs: Vec::new(),
        payload: serde_json::json!({
            "kind": "worktree_merge",
            "merged": result.merged,
            "aborted": result.aborted,
            "conflicts": result.conflicts,
            "sourceBranch": result.source_branch,
            "targetBranch": result.target_branch,
            "message": result.message,
        }),
    };
    timeline
        .append(event)
        .map_err(|e| format!("merge completed but timeline note failed: {e}"))?;

    Ok(result)
}
