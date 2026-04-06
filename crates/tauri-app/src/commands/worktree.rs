use chrono::Utc;
use serde::Serialize;
use tokio::process::Command;
use tauri::State;

use codeforge_persistence::models::{PrGhState, Worktree, WorktreeStatus};
use codeforge_persistence::queries;
use codeforge_persistence::{ProjectId, ThreadId, WorktreeId};

use crate::state::TauriState;

#[derive(Debug, Serialize)]
pub struct WorktreeInfo {
    pub thread_id: String,
    pub branch: String,
    pub path: String,
    pub active: bool,
    pub pr_number: Option<u32>,
    pub status: String,
    /// Cached from last GitHub poll — "open"/"closed"/"merged"/null.
    pub pr_state: Option<String>,
    /// Cached merge commit SHA (present iff PR was merged).
    pub pr_merge_commit: Option<String>,
    /// Cached PR URL for clickable links without another gh call.
    pub pr_url: Option<String>,
}

impl From<&Worktree> for WorktreeInfo {
    fn from(wt: &Worktree) -> Self {
        let path_exists = std::path::Path::new(&wt.path).exists();
        WorktreeInfo {
            thread_id: wt.thread_id.to_string(),
            branch: wt.branch.clone(),
            path: wt.path.clone(),
            active: wt.status == WorktreeStatus::Active && path_exists,
            pr_number: wt.pr_number,
            status: wt.status.as_str().to_string(),
            pr_state: wt.pr_state.map(|s| s.as_str().to_string()),
            pr_merge_commit: wt.pr_merge_commit.clone(),
            pr_url: wt.pr_url.clone(),
        }
    }
}

#[tauri::command]
pub async fn create_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    thread_title: String,
    project_path: String,
    project_id: String,
) -> Result<WorktreeInfo, String> {
    let sanitized = thread_title
        .to_lowercase()
        .replace(|c: char| !c.is_alphanumeric() && c != '-', "-")
        .trim_matches('-')
        .to_string();
    let id_suffix = if thread_id.len() >= 8 { &thread_id[..8] } else { &thread_id };
    let branch = format!("codeforge/{sanitized}-{id_suffix}");
    let worktree_path = format!("{project_path}/.codeforge-worktrees/{sanitized}");

    // Create the worktree directory parent
    std::fs::create_dir_all(format!("{project_path}/.codeforge-worktrees"))
        .map_err(|e| format!("Failed to create worktree directory: {e}"))?;

    // Create a new branch and worktree
    let output = Command::new("git")
        .args(["worktree", "add", "-b", &branch, &worktree_path])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        // Branch might already exist, try without -b
        let output2 = Command::new("git")
            .args(["worktree", "add", &worktree_path, &branch])
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| format!("Failed to run git: {e}"))?;

        if !output2.status.success() {
            return Err(format!(
                "Failed to create worktree: {}",
                String::from_utf8_lossy(&output2.stderr)
            ));
        }
    }

    let now = Utc::now();
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;
    let pid: ProjectId = project_id.parse().map_err(|e| format!("{e}"))?;
    let wt = Worktree {
        id: WorktreeId::new(),
        thread_id: tid,
        project_id: pid,
        branch: branch.clone(),
        path: worktree_path.clone(),
        pr_number: None,
        status: WorktreeStatus::Active,
        created_at: now,
        updated_at: now,
        pr_state: None,
        pr_merge_commit: None,
        last_seen_comment_count: 0,
        pr_url: None,
    };

    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::insert_worktree(db.conn(), &wt).map_err(|e| format!("{e}"))?;
    }

    Ok(WorktreeInfo::from(&wt))
}

#[tauri::command]
pub fn get_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<Option<WorktreeInfo>, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let wt = queries::get_worktree_by_thread(db.conn(), tid).map_err(|e| format!("{e}"))?;
    Ok(wt.as_ref().map(WorktreeInfo::from))
}

#[tauri::command]
pub async fn merge_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<String, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_active_worktree_by_thread(db.conn(), tid)
            .map_err(|e| format!("{e}"))?
            .ok_or("No active worktree found for this thread")?
    };

    let branch = &wt.branch;
    let worktree_path = &wt.path;

    // Get current branch name
    let current_branch = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get current branch: {e}"))?;
    let main_branch = String::from_utf8_lossy(&current_branch.stdout)
        .trim()
        .to_string();
    let main_branch = if main_branch.is_empty() { "main".to_string() } else { main_branch };

    if let Some(pr_num) = wt.pr_number {
        // PR mode: commit any pending changes, check for divergence, then push
        // the worktree branch to the PR's remote branch. We keep the worktree
        // alive — the user can continue making changes and pushing again.
        // The thread is only locked when the PR is merged on GitHub.
        let pr_num_str = pr_num.to_string();
        let pr_branch_output = Command::new("gh")
            .args(["pr", "view", &pr_num_str, "--json", "headRefName", "--jq", ".headRefName"])
            .current_dir(&project_path)
            .output()
            .await;

        let remote_branch = match pr_branch_output {
            Ok(out) if out.status.success() => {
                let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if b.is_empty() { branch.clone() } else { b }
            }
            _ => branch.clone(),
        };

        // 1) Commit any uncommitted changes.
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(worktree_path)
            .output()
            .await
            .map_err(|e| format!("Failed to check status: {e}"))?;

        if !String::from_utf8_lossy(&status.stdout).trim().is_empty() {
            let _ = Command::new("git").args(["add", "-A"]).current_dir(worktree_path).output().await;
            let _ = Command::new("git").args(["commit", "-m", "Changes from CodeForge"]).current_dir(worktree_path).output().await;
        }

        // 2) Fetch latest remote state so the divergence check is accurate.
        let _ = Command::new("git")
            .args(["fetch", "origin", &remote_branch, "--quiet"])
            .current_dir(worktree_path)
            .output()
            .await;

        // 3) Check ahead/behind between our branch and origin's.
        let (ahead, behind) = ahead_behind(worktree_path, "HEAD", &format!("origin/{remote_branch}")).await;

        // 4) If the remote has commits we don't, refuse to push. Return a typed
        // error (prefix "DIVERGED:") so the frontend can recognize and offer a
        // targeted "rebase & retry" flow instead of dumping a generic error
        // into the AI prompt.
        if behind > 0 {
            return Err(format!(
                "DIVERGED: ahead={ahead} behind={behind} branch={remote_branch} pr={pr_num}"
            ));
        }

        // 5) Safe to push. Use --force-with-lease to protect against any race
        // where remote moved between fetch and push; this will only succeed if
        // the remote still matches what we just fetched.
        let lease = format!("{remote_branch}:origin/{remote_branch}");
        let refspec = format!("{branch}:{remote_branch}");
        let push = Command::new("git")
            .args([
                "push",
                "origin",
                &refspec,
                "--force-with-lease",
                &format!("--force-with-lease={lease}"),
            ])
            .current_dir(worktree_path)
            .output()
            .await
            .map_err(|e| format!("Failed to push: {e}"))?;

        if !push.status.success() {
            let stderr = String::from_utf8_lossy(&push.stderr).to_string();
            // Differentiate lease rejection (remote moved) from generic failure.
            if stderr.contains("stale info") || stderr.contains("rejected") {
                return Err(format!("DIVERGED: lease_rejected branch={remote_branch} pr={pr_num}"));
            }
            return Err(format!("Push failed: {stderr}"));
        }

        Ok(format!("Pushed to PR #{pr_num} ({remote_branch})"))
    } else {
        // Normal mode: merge worktree branch into main
        let merge = Command::new("git")
            .args(["merge", branch, "--no-edit"])
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| format!("Failed to merge: {e}"))?;

        if !merge.status.success() {
            let stderr = String::from_utf8_lossy(&merge.stderr).to_string();

            let _ = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(&project_path)
                .output()
                .await;

            return Err(format!(
                "Merge has conflicts. Resolve them manually in the worktree at {worktree_path} then try again.\n\nConflict details: {stderr}"
            ));
        }

        // Remove worktree
        let _ = Command::new("git")
            .args(["worktree", "remove", worktree_path, "--force"])
            .current_dir(&project_path)
            .output()
            .await;

        // Delete the branch
        let _ = Command::new("git")
            .args(["branch", "-d", branch])
            .current_dir(&project_path)
            .output()
            .await;

        // Mark as merged in DB
        {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = queries::update_worktree_status(db.conn(), wt.id, WorktreeStatus::Merged);
        }

        Ok(format!("Merged {branch} into {main_branch}"))
    }
}

/// Commit all changes in the worktree, push the branch, and create a GitHub PR.
/// Returns the PR URL on success.
#[tauri::command]
pub async fn create_pr_from_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
    title: String,
    body: String,
) -> Result<String, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_active_worktree_by_thread(db.conn(), tid)
            .map_err(|e| format!("{e}"))?
            .ok_or("No active worktree found for this thread")?
    };

    let branch = &wt.branch;
    let worktree_path = &wt.path;

    // Commit any uncommitted changes
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(worktree_path)
        .output()
        .await
        .map_err(|e| format!("Failed to check status: {e}"))?;

    if !String::from_utf8_lossy(&status.stdout).trim().is_empty() {
        let _ = Command::new("git").args(["add", "-A"]).current_dir(worktree_path).output().await;
        let _ = Command::new("git").args(["commit", "-m", &title]).current_dir(worktree_path).output().await;
    }

    // Push the branch
    let push = Command::new("git")
        .args(["push", "-u", "origin", branch])
        .current_dir(worktree_path)
        .output()
        .await
        .map_err(|e| format!("Failed to push: {e}"))?;

    if !push.status.success() {
        return Err(format!("Push failed: {}", String::from_utf8_lossy(&push.stderr)));
    }

    // Determine base branch
    let base_out = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&project_path)
        .output()
        .await;
    let base = match base_out {
        Ok(o) if o.status.success() => {
            let b = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if b.is_empty() { "main".to_string() } else { b }
        }
        _ => "main".to_string(),
    };

    // Create the PR
    let pr = Command::new("gh")
        .args(["pr", "create", "--title", &title, "--body", &body, "--head", branch, "--base", &base])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to create PR: {e}"))?;

    if !pr.status.success() {
        return Err(format!("gh pr create failed: {}", String::from_utf8_lossy(&pr.stderr)));
    }

    let pr_url = String::from_utf8_lossy(&pr.stdout).trim().to_string();

    // Extract PR number from URL and update the worktree record
    if let Some(num_str) = pr_url.rsplit('/').next() {
        if let Ok(num) = num_str.parse::<u32>() {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = queries::update_worktree_pr(db.conn(), tid, num);
        }
    }

    Ok(pr_url)
}

// ---------------------------------------------------------------------------
// PR status reconciler (the one place that talks to GitHub and updates DB)
// ---------------------------------------------------------------------------

/// The fully-resolved lifecycle state of a thread — computed here, stored
/// verbatim on the frontend. The frontend never derives lifecycle from loose
/// booleans; it just renders whatever variant this is.
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LifecycleState {
    /// No PR linked yet, or worktree exists without a PR.
    Working,
    /// PR is open and actively tracked.
    PrOpen {
        pr: PrSnapshot,
        ci: String,             // "success" | "failure" | "pending" | "none" | "unknown"
        review: String,         // "approved" | "changes_requested" | "commented" | "none"
        unread_comments: u32,
    },
    /// PR branch has diverged from its upstream (local & remote both have commits).
    PrOpenDiverged {
        pr: PrSnapshot,
        ahead: u32,
        behind: u32,
    },
    /// PR was closed without merging. Thread is read-only.
    PrClosed { pr: PrSnapshot },
    /// PR was merged and the merge commit is reachable on base. Thread is read-only.
    PrMerged {
        pr: PrSnapshot,
        merge_commit: String,
    },
    /// PR was merged but the merge commit is no longer reachable from base
    /// — someone reverted the merge on GitHub. Thread is editable.
    PrReverted { pr: PrSnapshot },
    /// Worktree directory missing from disk but the DB row remains.
    WorktreeMissing { branch: String, path: String },
    /// Git's worktree list doesn't know about this worktree anymore.
    WorktreeOrphaned { branch: String, path: String },
}

#[derive(Debug, Serialize, Clone)]
pub struct PrSnapshot {
    pub number: u32,
    pub url: String,
    pub state: String, // "open" | "closed" | "merged"
}

/// What the backend emits from `get_pr_status`. This is consumed by the
/// frontend poller: it stores `lifecycle` on the thread, surfaces any
/// transitions via system events, and appends new review comments.
#[derive(Debug, Serialize)]
pub struct PrStatus {
    pub pr_number: u32,
    pub ci_status: String,
    pub review_status: String,
    pub comment_count: u32,
    /// Fully-resolved lifecycle state — frontend stores this as-is.
    pub lifecycle: LifecycleState,
    /// The worktree's previous status before this reconcile tick, so the
    /// frontend can emit transition events only on change.
    pub previous_status: String,
    /// Count of review comments the user hasn't seen yet (since last poll).
    pub new_comment_count: u32,
    /// True if this reconcile tick detected a revert (merged→revert) and the
    /// thread should be unlocked. One-shot signal — the lifecycle handles
    /// the persistent part.
    pub revert_detected: bool,
    /// True if the previous worktree status was `merged` but the PR is now
    /// open — PR was reopened. Thread should be unlocked.
    pub reopen_detected: bool,
    /// True when the PR no longer exists on GitHub (404). The worktree's
    /// PR linkage has been cleared.
    pub pr_missing: bool,
}

/// Parse the merged JSON blob from one `gh pr view` call.
///
/// We combine CI (statusCheckRollup), review decision, state, merge commit,
/// URL, and comment counts into a single subprocess invocation instead of
/// three separate ones.
struct GhPrView {
    state: String,            // "OPEN" | "CLOSED" | "MERGED"
    review_decision: String,
    url: String,
    merge_commit_oid: Option<String>,
    base_ref_name: String,
    comment_count: u32,
    ci_status: String,        // "success" | "failure" | "pending" | "none" | "unknown"
}

fn parse_gh_pr_view(raw: &str) -> Option<GhPrView> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;

    let state = v.get("state")?.as_str()?.to_string();
    let review_decision = v.get("reviewDecision")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .to_string();
    let url = v.get("url").and_then(|x| x.as_str()).unwrap_or("").to_string();
    let base_ref_name = v.get("baseRefName").and_then(|x| x.as_str()).unwrap_or("main").to_string();
    let merge_commit_oid = v.get("mergeCommit")
        .and_then(|x| x.get("oid"))
        .and_then(|x| x.as_str())
        .map(String::from)
        .filter(|s| !s.is_empty());

    let comments_len = v.get("comments").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
    let reviews_len = v.get("reviews").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
    let comment_count = (comments_len + reviews_len) as u32;

    // Parse CI rollup — proper JSON walk, no substring hacks.
    let ci_status = match v.get("statusCheckRollup").and_then(|x| x.as_array()) {
        Some(checks) if !checks.is_empty() => {
            let mut has_failure = false;
            let mut has_pending = false;
            let mut has_success = false;
            for check in checks {
                // GitHub returns either StatusContext (state: SUCCESS/ERROR/PENDING)
                // or CheckRun (conclusion: SUCCESS/FAILURE/...; status: COMPLETED/IN_PROGRESS).
                let state = check.get("state").and_then(|x| x.as_str()).unwrap_or("");
                let status = check.get("status").and_then(|x| x.as_str()).unwrap_or("");
                let conclusion = check.get("conclusion").and_then(|x| x.as_str()).unwrap_or("");

                let combined = format!("{state} {status} {conclusion}").to_ascii_uppercase();
                if combined.contains("FAILURE") || combined.contains("ERROR") || combined.contains("TIMED_OUT") || combined.contains("CANCELLED") || combined.contains("ACTION_REQUIRED") {
                    has_failure = true;
                } else if combined.contains("IN_PROGRESS") || combined.contains("QUEUED") || combined.contains("PENDING") || combined.contains("EXPECTED") || (status == "IN_PROGRESS") {
                    has_pending = true;
                } else if combined.contains("SUCCESS") || combined.contains("NEUTRAL") || combined.contains("SKIPPED") {
                    has_success = true;
                }
            }
            if has_failure { "failure".to_string() }
            else if has_pending { "pending".to_string() }
            else if has_success { "success".to_string() }
            else { "unknown".to_string() }
        }
        _ => "none".to_string(),
    };

    Some(GhPrView { state, review_decision, url, merge_commit_oid, base_ref_name, comment_count, ci_status })
}

/// Count commits each side has that the other doesn't, relative to a pair of
/// refs. Returns `(ahead, behind)` where `ahead` is "local has these",
/// `behind` is "remote has these". Zeros on any git failure.
async fn ahead_behind(cwd: &str, local_ref: &str, remote_ref: &str) -> (u32, u32) {
    let out = Command::new("git")
        .args([
            "rev-list",
            "--left-right",
            "--count",
            &format!("{local_ref}...{remote_ref}"),
        ])
        .current_dir(cwd)
        .output()
        .await;

    match out {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            let parts: Vec<&str> = raw.split_whitespace().collect();
            let ahead = parts.first().and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            let behind = parts.get(1).and_then(|s| s.parse::<u32>().ok()).unwrap_or(0);
            (ahead, behind)
        }
        _ => (0, 0),
    }
}

/// Check whether a commit is reachable from the tip of a branch.
///
/// Used for revert detection: a merged PR whose merge commit is no longer
/// reachable from `origin/<base>` was reverted.
async fn is_commit_reachable(cwd: &str, commit: &str, base_ref: &str) -> bool {
    // Make sure we actually have the commit and the base locally before the
    // reachability check. A missing fetch would give us a false negative.
    let _ = Command::new("git")
        .args(["fetch", "origin", base_ref, "--quiet"])
        .current_dir(cwd)
        .output()
        .await;

    let output = Command::new("git")
        .args(["merge-base", "--is-ancestor", commit, &format!("origin/{base_ref}")])
        .current_dir(cwd)
        .output()
        .await;

    matches!(output, Ok(o) if o.status.success())
}

/// Fetch PR state from GitHub and reconcile it against the DB.
///
/// This is the **single source of truth** for PR lifecycle transitions.
/// It's called by the frontend poller (every ~60 s) and by any code that
/// explicitly wants to refresh a thread's PR state (e.g. after pushing).
///
/// Responsibilities:
///   1. One `gh pr view` subprocess call with all needed fields.
///   2. Detect the desired lifecycle (`working`, `pr_open`, `pr_merged`, etc).
///   3. Persist the new state into the worktree row (pr_state, pr_merge_commit, pr_url).
///   4. Update `last_seen_comment_count` so the next poll can compute a real delta.
///   5. Return everything the frontend needs in one trip — including the
///      previous status so it can emit transition events idempotently.
#[tauri::command]
pub async fn get_pr_status(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<Option<PrStatus>, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    // Snapshot current DB state
    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_worktree_by_thread(db.conn(), tid).map_err(|e| format!("{e}"))?
    };
    let Some(wt) = wt else { return Ok(None); };
    let Some(pr_num) = wt.pr_number else { return Ok(None); };
    let previous_status = wt.status.as_str().to_string();

    // One subprocess call
    let out = Command::new("gh")
        .args([
            "pr", "view", &pr_num.to_string(),
            "--json", "state,reviewDecision,url,mergeCommit,baseRefName,comments,reviews,statusCheckRollup",
        ])
        .current_dir(&project_path)
        .output()
        .await;

    let parsed = match out {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout);
            parse_gh_pr_view(&raw)
        }
        // Detect 404 / PR-not-found: the PR was deleted or transferred.
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr).to_ascii_lowercase();
            if stderr.contains("could not resolve") || stderr.contains("not found") || stderr.contains("no pull requests") {
                // Clear the PR linkage; keep the worktree.
                let db = state.db.lock().map_err(|e| format!("{e}"))?;
                let _ = queries::clear_worktree_pr(db.conn(), wt.id);
                return Ok(Some(PrStatus {
                    pr_number: pr_num,
                    ci_status: "none".to_string(),
                    review_status: "none".to_string(),
                    comment_count: 0,
                    lifecycle: LifecycleState::Working,
                    previous_status,
                    new_comment_count: 0,
                    revert_detected: false,
                    reopen_detected: false,
                    pr_missing: true,
                }));
            }
            // Network/auth error: keep existing state, return None so poller leaves things alone.
            return Ok(None);
        }
        Err(_) => return Ok(None),
    };

    let Some(view) = parsed else { return Ok(None); };

    // Compute the new desired worktree status from GitHub's state.
    let gh_state: PrGhState = view.state.parse().unwrap_or(PrGhState::Unknown);
    let mut revert_detected = false;

    let (new_status, lifecycle) = match gh_state {
        PrGhState::Open => {
            let snap = PrSnapshot { number: pr_num, url: view.url.clone(), state: "open".to_string() };
            let review = if view.review_decision.is_empty() {
                "none".to_string()
            } else {
                view.review_decision.to_ascii_lowercase()
            };
            // new comment count, bounded below by what we've seen
            let new_comments = view.comment_count.saturating_sub(wt.last_seen_comment_count);
            (
                WorktreeStatus::Active,
                LifecycleState::PrOpen { pr: snap, ci: view.ci_status.clone(), review, unread_comments: new_comments },
            )
        }
        PrGhState::Merged => {
            let snap = PrSnapshot { number: pr_num, url: view.url.clone(), state: "merged".to_string() };
            match view.merge_commit_oid.as_deref() {
                Some(mc) => {
                    // Reachability check — did someone revert the merge?
                    let reachable = is_commit_reachable(&project_path, mc, &view.base_ref_name).await;
                    if reachable {
                        (WorktreeStatus::Merged, LifecycleState::PrMerged { pr: snap, merge_commit: mc.to_string() })
                    } else {
                        revert_detected = true;
                        // Thread remains editable — keep status Active.
                        (WorktreeStatus::Active, LifecycleState::PrReverted { pr: snap })
                    }
                }
                None => (WorktreeStatus::Merged, LifecycleState::PrMerged { pr: snap, merge_commit: String::new() }),
            }
        }
        PrGhState::Closed => {
            let snap = PrSnapshot { number: pr_num, url: view.url.clone(), state: "closed".to_string() };
            (WorktreeStatus::Closed, LifecycleState::PrClosed { pr: snap })
        }
        PrGhState::Unknown => {
            // Unknown state — leave the DB as-is.
            return Ok(None);
        }
    };

    // Detect reopen: was previously merged/closed in DB, now open upstream.
    let reopen_detected = matches!(
        (wt.status, gh_state),
        (WorktreeStatus::Merged, PrGhState::Open) | (WorktreeStatus::Closed, PrGhState::Open)
    );

    // Compute new comment delta against the persisted high-water mark.
    let new_comment_count = view.comment_count.saturating_sub(wt.last_seen_comment_count);

    // Persist the new state.
    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        // 1) status (only write on change)
        if wt.status != new_status {
            let _ = queries::update_worktree_status(db.conn(), wt.id, new_status);
        }
        // 2) pr_state + merge_commit + url (always write — they're cheap)
        let _ = queries::update_worktree_pr_state(
            db.conn(),
            wt.id,
            gh_state,
            view.merge_commit_oid.as_deref(),
            if view.url.is_empty() { None } else { Some(&view.url) },
        );
        // 3) comment count high-water mark
        if view.comment_count != wt.last_seen_comment_count {
            let _ = queries::update_worktree_comment_count(db.conn(), wt.id, view.comment_count);
        }
    }

    let review_status = if view.review_decision.is_empty() {
        "none".to_string()
    } else {
        view.review_decision.to_ascii_lowercase()
    };

    Ok(Some(PrStatus {
        pr_number: pr_num,
        ci_status: view.ci_status,
        review_status,
        comment_count: view.comment_count,
        lifecycle,
        previous_status,
        new_comment_count,
        revert_detected,
        reopen_detected,
        pr_missing: false,
    }))
}

/// Fetch new PR review comments and return them as text.
#[tauri::command]
pub async fn get_pr_review_comments(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<Vec<PrComment>, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let pr_num = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_worktree_by_thread(db.conn(), tid)
            .ok()
            .flatten()
            .and_then(|wt| wt.pr_number)
    };

    let pr_num = match pr_num {
        Some(n) => n.to_string(),
        None => return Ok(vec![]),
    };

    let out = Command::new("gh")
        .args([
            "pr", "view", &pr_num,
            "--json", "reviews",
            "--jq", ".reviews[] | {author: .author.login, state: .state, body: .body}",
        ])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get reviews: {e}"))?;

    if !out.status.success() {
        return Ok(vec![]);
    }

    let raw = String::from_utf8_lossy(&out.stdout);
    let mut comments = Vec::new();

    for line in raw.lines() {
        if line.trim().is_empty() { continue; }
        if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(line) {
            comments.push(PrComment {
                author: parsed["author"].as_str().unwrap_or("unknown").to_string(),
                state: parsed["state"].as_str().unwrap_or("").to_string(),
                body: parsed["body"].as_str().unwrap_or("").to_string(),
            });
        }
    }

    Ok(comments)
}

#[derive(Debug, Serialize)]
pub struct PrComment {
    pub author: String,
    pub state: String,
    pub body: String,
}

#[derive(Debug, Serialize)]
pub struct OpenPr {
    pub number: u32,
    pub title: String,
    pub branch: String,
    pub author: String,
    pub url: String,
}

/// List open PRs for the repo.
#[tauri::command]
pub async fn list_open_prs(project_path: String) -> Result<Vec<OpenPr>, String> {
    let out = Command::new("gh")
        .args(["pr", "list", "--json", "number,title,headRefName,author,url", "--limit", "30"])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to list PRs: {e}"))?;

    if !out.status.success() {
        return Err(format!("gh pr list failed: {}", String::from_utf8_lossy(&out.stderr)));
    }

    let raw = String::from_utf8_lossy(&out.stdout);
    let parsed: Vec<serde_json::Value> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed to parse PR list: {e}"))?;

    Ok(parsed.iter().map(|pr| OpenPr {
        number: pr["number"].as_u64().unwrap_or(0) as u32,
        title: pr["title"].as_str().unwrap_or("").to_string(),
        branch: pr["headRefName"].as_str().unwrap_or("").to_string(),
        author: pr["author"].get("login").and_then(|v| v.as_str()).unwrap_or("").to_string(),
        url: pr["url"].as_str().unwrap_or("").to_string(),
    }).collect())
}

/// Check if a PR is already linked to any thread. Returns the thread ID if so.
#[tauri::command]
pub fn find_thread_for_pr(
    state: State<'_, TauriState>,
    pr_number: u32,
) -> Result<Option<String>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let tid = queries::find_thread_for_pr_number(db.conn(), pr_number)
        .map_err(|e| format!("{e}"))?;
    Ok(tid.map(|id| id.to_string()))
}

/// Checkout a PR's branch into a new worktree for a thread.
#[tauri::command]
pub async fn checkout_pr_into_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    pr_number: u32,
    project_path: String,
    project_id: String,
) -> Result<WorktreeInfo, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;
    let pid: ProjectId = project_id.parse().map_err(|e| format!("{e}"))?;

    // Get the PR's branch name
    let pr_out = Command::new("gh")
        .args(["pr", "view", &pr_number.to_string(), "--json", "headRefName,title", "--jq", ".headRefName + \"\\n\" + .title"])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get PR info: {e}"))?;

    if !pr_out.status.success() {
        return Err(format!("Failed to get PR #{pr_number}: {}", String::from_utf8_lossy(&pr_out.stderr)));
    }

    let raw_output = String::from_utf8_lossy(&pr_out.stdout).trim().to_string();
    let pr_branch = raw_output.lines().next().unwrap_or("").to_string();
    if pr_branch.is_empty() {
        return Err("Could not determine PR branch".into());
    }

    // Fetch the branch
    let _ = Command::new("git")
        .args(["fetch", "origin", &pr_branch])
        .current_dir(&project_path)
        .output()
        .await;

    // Create worktree from the PR branch
    let sanitized = pr_branch.replace('/', "-");
    let worktree_path = format!("{project_path}/.codeforge-worktrees/{sanitized}");

    std::fs::create_dir_all(format!("{project_path}/.codeforge-worktrees"))
        .map_err(|e| format!("Failed to create worktree directory: {e}"))?;

    // Always create a local tracking branch to avoid detached HEAD.
    // Try -b first (creates new local branch tracking the remote).
    // If the local branch already exists, use it directly.
    let output = Command::new("git")
        .args(["worktree", "add", "-b", &pr_branch, &worktree_path, &format!("origin/{pr_branch}")])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to create worktree: {e}"))?;

    if !output.status.success() {
        // Local branch already exists — use it
        let output2 = Command::new("git")
            .args(["worktree", "add", &worktree_path, &pr_branch])
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| format!("Failed to create worktree: {e}"))?;

        if !output2.status.success() {
            return Err(format!("Failed to checkout PR branch: {}", String::from_utf8_lossy(&output2.stderr)));
        }

        // Make sure the local branch is up to date with the remote
        let _ = Command::new("git")
            .args(["reset", "--hard", &format!("origin/{pr_branch}")])
            .current_dir(&worktree_path)
            .output()
            .await;
    }

    // Set upstream tracking so push/pull work without extra args
    let _ = Command::new("git")
        .args(["branch", "--set-upstream-to", &format!("origin/{pr_branch}"), &pr_branch])
        .current_dir(&worktree_path)
        .output()
        .await;

    let now = Utc::now();
    let wt = Worktree {
        id: WorktreeId::new(),
        thread_id: tid,
        project_id: pid,
        branch: pr_branch,
        path: worktree_path,
        pr_number: Some(pr_number),
        status: WorktreeStatus::Active,
        created_at: now,
        updated_at: now,
        pr_state: None,
        pr_merge_commit: None,
        last_seen_comment_count: 0,
        pr_url: None,
    };

    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::insert_worktree(db.conn(), &wt).map_err(|e| format!("{e}"))?;
    }

    Ok(WorktreeInfo::from(&wt))
}

/// Link an existing PR to a thread without touching git.
///
/// Three cases:
///   1. Thread has no worktree → checkout the PR's branch into a new worktree
///      (delegates to `checkout_pr_into_worktree`).
///   2. Thread has an active worktree with no PR → stamp this PR number onto
///      the existing worktree row.
///   3. Thread has an active worktree already linked to a *different* PR →
///      return an error so the UI can prompt.
///
/// Returns the up-to-date `WorktreeInfo`.
#[tauri::command]
pub async fn link_pr_to_thread(
    state: State<'_, TauriState>,
    thread_id: String,
    pr_number: u32,
    project_path: String,
    project_id: String,
) -> Result<WorktreeInfo, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    // What do we have?
    let existing = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_active_worktree_by_thread(db.conn(), tid).map_err(|e| format!("{e}"))?
    };

    match existing {
        // Case 3: already linked to a different PR
        Some(wt) if wt.pr_number.is_some() && wt.pr_number != Some(pr_number) => {
            Err(format!(
                "Thread is already linked to PR #{}. Unlink it first or use a different thread.",
                wt.pr_number.unwrap()
            ))
        }
        // Case 2: active worktree, no PR — stamp it on
        Some(wt) => {
            {
                let db = state.db.lock().map_err(|e| format!("{e}"))?;
                queries::update_worktree_pr(db.conn(), tid, pr_number)
                    .map_err(|e| format!("{e}"))?;
            }
            // Fetch fresh PR state so the lifecycle is accurate immediately.
            let _ = get_pr_status(state.clone(), thread_id.clone(), project_path).await;
            // Return the updated row
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let updated = queries::get_active_worktree_by_thread(db.conn(), tid)
                .map_err(|e| format!("{e}"))?
                .ok_or("Worktree vanished after update")?;
            Ok(WorktreeInfo {
                pr_number: Some(pr_number),
                ..WorktreeInfo::from(&updated)
            })
        }
        // Case 1: no worktree — do a full checkout
        None => {
            checkout_pr_into_worktree(state, thread_id, pr_number, project_path, project_id).await
        }
    }
}

// ---------------------------------------------------------------------------
// 1.2 — Worktree Health Monitoring
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct WorktreeHealth {
    pub thread_id: String,
    pub status: String, // "healthy", "missing", "orphaned", "detached_head"
    pub branch: String,
    pub path: String,
}

/// Validate all active worktrees for a project against actual git state.
#[tauri::command]
pub async fn validate_worktrees(
    state: State<'_, TauriState>,
    project_path: String,
    project_id: String,
) -> Result<Vec<WorktreeHealth>, String> {
    let pid: ProjectId = project_id.parse().map_err(|e| format!("{e}"))?;

    let worktrees = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_worktrees_by_project(db.conn(), pid).map_err(|e| format!("{e}"))?
    };

    let git_list = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(&project_path)
        .output()
        .await
        .ok();

    let git_paths: Vec<String> = git_list
        .map(|o| {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.strip_prefix("worktree ").map(String::from))
                .collect()
        })
        .unwrap_or_default();

    let mut results = Vec::new();

    for wt in &worktrees {
        if wt.status != WorktreeStatus::Active { continue; }

        let path_exists = std::path::Path::new(&wt.path).exists();
        let in_git_list = git_paths.iter().any(|p| p == &wt.path);

        let mut health = if !path_exists {
            "missing"
        } else if !in_git_list {
            "orphaned"
        } else {
            let head_check = Command::new("git")
                .args(["symbolic-ref", "HEAD"])
                .current_dir(&wt.path)
                .output()
                .await;
            match head_check {
                Ok(o) if o.status.success() => "healthy",
                _ => "detached_head",
            }
        };

        if health == "missing" || health == "orphaned" {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = queries::update_worktree_status(db.conn(), wt.id, WorktreeStatus::Orphaned);
        }

        // For linked PRs, verify the PR still exists on GitHub. If `gh pr view`
        // returns a "not found" error, the PR was deleted / transferred /
        // repo renamed. Clear the linkage so the worktree becomes "branch only".
        if let Some(pr_num) = wt.pr_number {
            let out = Command::new("gh")
                .args(["pr", "view", &pr_num.to_string(), "--json", "state"])
                .current_dir(&project_path)
                .output()
                .await;
            if let Ok(o) = out {
                if !o.status.success() {
                    let stderr = String::from_utf8_lossy(&o.stderr).to_ascii_lowercase();
                    if stderr.contains("could not resolve") || stderr.contains("not found") || stderr.contains("no pull requests") {
                        let db = state.db.lock().map_err(|e| format!("{e}"))?;
                        let _ = queries::clear_worktree_pr(db.conn(), wt.id);
                        // Mark the health so the UI can surface it distinctly
                        // from a healthy active worktree.
                        if health == "healthy" { health = "pr_missing"; }
                    }
                }
            }
        }

        results.push(WorktreeHealth {
            thread_id: wt.thread_id.to_string(),
            status: health.to_string(),
            branch: wt.branch.clone(),
            path: wt.path.clone(),
        });
    }

    Ok(results)
}

/// Repair a worktree — either recreate it or detach it.
#[tauri::command]
pub async fn repair_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
    action: String,
) -> Result<WorktreeInfo, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_worktree_by_thread(db.conn(), tid)
            .map_err(|e| format!("{e}"))?
            .ok_or("No worktree record found for this thread")?
    };

    if action == "detach" {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::update_worktree_status(db.conn(), wt.id, WorktreeStatus::Deleted).map_err(|e| format!("{e}"))?;
        let mut info = WorktreeInfo::from(&wt);
        info.status = "deleted".to_string();
        info.active = false;
        return Ok(info);
    }

    let _ = Command::new("git").args(["worktree", "prune"]).current_dir(&project_path).output().await;

    let branch_exists = Command::new("git")
        .args(["rev-parse", "--verify", &wt.branch])
        .current_dir(&project_path)
        .output()
        .await
        .map(|o| o.status.success())
        .unwrap_or(false);

    std::fs::create_dir_all(format!("{project_path}/.codeforge-worktrees")).map_err(|e| format!("{e}"))?;

    let output = if branch_exists {
        Command::new("git").args(["worktree", "add", &wt.path, &wt.branch]).current_dir(&project_path).output().await
    } else {
        Command::new("git").args(["worktree", "add", "-b", &wt.branch, &wt.path]).current_dir(&project_path).output().await
    }.map_err(|e| format!("Failed to recreate worktree: {e}"))?;

    if !output.status.success() {
        return Err(format!("Failed to recreate: {}", String::from_utf8_lossy(&output.stderr)));
    }

    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::update_worktree_status(db.conn(), wt.id, WorktreeStatus::Active).map_err(|e| format!("{e}"))?;
    }

    let mut info = WorktreeInfo::from(&wt);
    info.status = "active".to_string();
    info.active = true;
    Ok(info)
}

/// Prune orphaned worktrees and clean up.
#[tauri::command]
pub async fn cleanup_worktrees(
    state: State<'_, TauriState>,
    project_path: String,
) -> Result<u32, String> {
    let _ = Command::new("git").args(["worktree", "prune"]).current_dir(&project_path).output().await;

    let all_wts = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_all_active_worktrees(db.conn()).map_err(|e| format!("{e}"))?
    };

    let mut cleaned = 0u32;
    for wt in &all_wts {
        if !std::path::Path::new(&wt.path).exists() {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = queries::update_worktree_status(db.conn(), wt.id, WorktreeStatus::Orphaned);
            cleaned += 1;
        }
    }
    Ok(cleaned)
}

// ---------------------------------------------------------------------------
// 1.3 — Merge Conflict Resolution
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ConflictFile {
    pub path: String,
    pub ours: String,
    pub theirs: String,
    pub base: String,
}

#[tauri::command]
pub async fn get_conflict_files(cwd: String) -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=U"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("{e}"))?;

    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect())
}

#[tauri::command]
pub async fn get_conflict_markers(cwd: String, file_path: String) -> Result<ConflictFile, String> {
    let full_path = std::path::Path::new(&cwd).join(&file_path);
    let content = std::fs::read_to_string(&full_path).map_err(|e| format!("{e}"))?;

    let mut ours = String::new();
    let mut theirs = String::new();
    let mut base = String::new();
    let mut section = "none";

    for line in content.lines() {
        if line.starts_with("<<<<<<<") { section = "ours"; }
        else if line.starts_with("|||||||") { section = "base"; }
        else if line.starts_with("=======") { section = "theirs"; }
        else if line.starts_with(">>>>>>>") { section = "none"; }
        else {
            match section {
                "ours" => { ours.push_str(line); ours.push('\n'); }
                "theirs" => { theirs.push_str(line); theirs.push('\n'); }
                "base" => { base.push_str(line); base.push('\n'); }
                _ => { ours.push_str(line); ours.push('\n'); theirs.push_str(line); theirs.push('\n'); base.push_str(line); base.push('\n'); }
            }
        }
    }

    Ok(ConflictFile { path: file_path, ours, theirs, base })
}

#[tauri::command]
pub async fn resolve_conflict(cwd: String, file_path: String, resolution: String) -> Result<(), String> {
    match resolution.as_str() {
        "ours" => { Command::new("git").args(["checkout", "--ours", "--", &file_path]).current_dir(&cwd).output().await.map_err(|e| format!("{e}"))?; }
        "theirs" => { Command::new("git").args(["checkout", "--theirs", "--", &file_path]).current_dir(&cwd).output().await.map_err(|e| format!("{e}"))?; }
        content => {
            let full_path = std::path::Path::new(&cwd).join(&file_path);
            std::fs::write(&full_path, content).map_err(|e| format!("{e}"))?;
        }
    }
    Command::new("git").args(["add", &file_path]).current_dir(&cwd).output().await.map_err(|e| format!("{e}"))?;
    Ok(())
}

#[tauri::command]
pub async fn finalize_merge(cwd: String) -> Result<String, String> {
    let output = Command::new("git").args(["commit", "--no-edit"]).current_dir(&cwd).output().await.map_err(|e| format!("{e}"))?;
    if !output.status.success() {
        return Err(format!("Commit failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok("Merge completed successfully".to_string())
}

#[tauri::command]
pub async fn abort_merge(cwd: String) -> Result<(), String> {
    Command::new("git").args(["merge", "--abort"]).current_dir(&cwd).output().await.map_err(|e| format!("{e}"))?;
    Ok(())
}

// ---------------------------------------------------------------------------
// 3.2 — Turn-level undo
// ---------------------------------------------------------------------------

/// Reset worktree to a specific commit and return the result.
#[tauri::command]
pub async fn undo_to_commit(
    state: State<'_, TauriState>,
    thread_id: String,
    commit_sha: String,
) -> Result<String, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_active_worktree_by_thread(db.conn(), tid)
            .map_err(|e| format!("{e}"))?
            .ok_or("No active worktree for this thread")?
    };

    // Verify commit exists
    let verify = Command::new("git")
        .args(["cat-file", "-t", &commit_sha])
        .current_dir(&wt.path)
        .output()
        .await
        .map_err(|e| format!("{e}"))?;

    if !verify.status.success() {
        return Err(format!("Commit {commit_sha} not found in worktree"));
    }

    let output = Command::new("git")
        .args(["reset", "--hard", &commit_sha])
        .current_dir(&wt.path)
        .output()
        .await
        .map_err(|e| format!("{e}"))?;

    if !output.status.success() {
        return Err(format!("Reset failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    Ok(format!("Reset to {}", &commit_sha[..8.min(commit_sha.len())]))
}

/// Get the current HEAD commit SHA for a worktree.
#[tauri::command]
pub async fn get_head_commit(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("{e}"))?;

    if !output.status.success() {
        return Err("Not a git repository or no commits".to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check if a worktree branch has unpushed commits or uncommitted changes.
/// Returns: "clean" (nothing to push, can merge), "dirty" (uncommitted changes),
/// "ahead" (has unpushed commits), "diverged" (ahead and behind).
#[tauri::command]
pub async fn check_worktree_sync_status(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<String, String> {
    let tid: ThreadId = thread_id.parse().map_err(|e| format!("{e}"))?;

    let wt = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        queries::get_active_worktree_by_thread(db.conn(), tid)
            .map_err(|e| format!("{e}"))?
            .ok_or("No active worktree")?
    };

    // Check for uncommitted changes
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&wt.path)
        .output()
        .await
        .map_err(|e| format!("{e}"))?;

    let has_changes = !String::from_utf8_lossy(&status.stdout).trim().is_empty();
    if has_changes {
        return Ok("dirty".to_string());
    }

    // Fetch latest remote state, then compute ahead/behind via the shared helper.
    let _ = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(&wt.path)
        .output()
        .await;

    let (ahead, behind) = ahead_behind(&wt.path, "HEAD", &format!("origin/{}", wt.branch)).await;

    if ahead > 0 && behind > 0 {
        return Ok("diverged".to_string());
    }
    if ahead > 0 {
        return Ok("ahead".to_string());
    }
    if behind > 0 {
        return Ok("behind".to_string());
    }
    Ok("clean".to_string())
}
