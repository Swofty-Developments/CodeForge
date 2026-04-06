use serde::Serialize;
use tokio::process::Command;
use tauri::State;

use crate::state::TauriState;

#[derive(Debug, Serialize)]
pub struct WorktreeInfo {
    pub thread_id: String,
    pub branch: String,
    pub path: String,
    pub active: bool,
}

#[tauri::command]
pub async fn create_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    thread_title: String,
    project_path: String,
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
        let _stderr = String::from_utf8_lossy(&output.stderr);
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

    // Store the mapping
    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        let _ = codeforge_persistence::queries::set_setting(
            db.conn(),
            &format!("worktree:{thread_id}"),
            &format!("{branch}|{worktree_path}"),
        );
    }

    Ok(WorktreeInfo {
        thread_id,
        branch,
        path: worktree_path,
        active: true,
    })
}

#[tauri::command]
pub fn get_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
) -> Result<Option<WorktreeInfo>, String> {
    let db = state.db.lock().map_err(|e| format!("{e}"))?;
    let setting = codeforge_persistence::queries::get_setting(
        db.conn(),
        &format!("worktree:{thread_id}"),
    )
    .map_err(|e| format!("{e}"))?;

    match setting {
        Some(val) => {
            let parts: Vec<&str> = val.splitn(2, '|').collect();
            if parts.len() != 2 {
                return Ok(None);
            }
            let path_exists = std::path::Path::new(parts[1]).exists();
            Ok(Some(WorktreeInfo {
                thread_id,
                branch: parts[0].to_string(),
                path: parts[1].to_string(),
                active: path_exists,
            }))
        }
        None => Ok(None),
    }
}

#[tauri::command]
pub async fn merge_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<String, String> {
    // Get worktree info
    let (branch, worktree_path) = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        let setting = codeforge_persistence::queries::get_setting(
            db.conn(),
            &format!("worktree:{thread_id}"),
        )
        .map_err(|e| format!("{e}"))?
        .ok_or("No worktree found for this thread")?;

        let parts: Vec<&str> = setting.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err("Invalid worktree setting".into());
        }
        (parts[0].to_string(), parts[1].to_string())
    };

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
    let main_branch = if main_branch.is_empty() {
        "main".to_string()
    } else {
        main_branch
    };

    // Check if this thread is linked to a PR
    let pr_number = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        codeforge_persistence::queries::get_setting(
            db.conn(),
            &format!("pr:{thread_id}"),
        )
        .ok()
        .flatten()
    };

    if let Some(pr_num) = &pr_number {
        // PR mode: commit and push the worktree branch to the PR's remote branch
        // First, find the PR's actual remote branch via `gh pr view`
        let pr_branch_output = Command::new("gh")
            .args(["pr", "view", pr_num, "--json", "headRefName", "--jq", ".headRefName"])
            .current_dir(&project_path)
            .output()
            .await;

        let remote_branch = match pr_branch_output {
            Ok(out) if out.status.success() => {
                let b = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if b.is_empty() { branch.clone() } else { b }
            }
            _ => branch.clone(), // Fallback to local branch name
        };

        // Commit any uncommitted changes in the worktree
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| format!("Failed to check status: {e}"))?;

        let has_changes = !String::from_utf8_lossy(&status.stdout).trim().is_empty();

        if has_changes {
            let _ = Command::new("git")
                .args(["add", "-A"])
                .current_dir(&worktree_path)
                .output()
                .await;

            let _ = Command::new("git")
                .args(["commit", "-m", "Changes from CodeForge"])
                .current_dir(&worktree_path)
                .output()
                .await;
        }

        // Push local worktree branch to the PR's remote branch
        let refspec = format!("{branch}:{remote_branch}");
        let push = Command::new("git")
            .args(["push", "origin", &refspec])
            .current_dir(&worktree_path)
            .output()
            .await
            .map_err(|e| format!("Failed to push: {e}"))?;

        if !push.status.success() {
            let stderr = String::from_utf8_lossy(&push.stderr);
            return Err(format!("Push failed: {stderr}"));
        }

        // Remove worktree but keep branch (PR is still open)
        let _ = Command::new("git")
            .args(["worktree", "remove", &worktree_path, "--force"])
            .current_dir(&project_path)
            .output()
            .await;

        // Remove worktree setting
        {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = codeforge_persistence::queries::delete_setting(
                db.conn(),
                &format!("worktree:{thread_id}"),
            );
        }

        Ok(format!("Pushed to PR #{pr_num} ({remote_branch})"))
    } else {
        // Normal mode: merge worktree branch into main
        let merge = Command::new("git")
            .args(["merge", &branch, "--no-edit"])
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| format!("Failed to merge: {e}"))?;

        if !merge.status.success() {
            let stderr = String::from_utf8_lossy(&merge.stderr).to_string();

            // Abort the failed merge to leave repo in clean state
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
            .args(["worktree", "remove", &worktree_path, "--force"])
            .current_dir(&project_path)
            .output()
            .await;

        // Delete the branch
        let _ = Command::new("git")
            .args(["branch", "-d", &branch])
            .current_dir(&project_path)
            .output()
            .await;

        // Remove setting
        {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = codeforge_persistence::queries::delete_setting(
                db.conn(),
                &format!("worktree:{thread_id}"),
            );
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
    // Get worktree info
    let (branch, worktree_path) = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        let setting = codeforge_persistence::queries::get_setting(
            db.conn(),
            &format!("worktree:{thread_id}"),
        )
        .map_err(|e| format!("{e}"))?
        .ok_or("No worktree found for this thread")?;

        let parts: Vec<&str> = setting.splitn(2, '|').collect();
        if parts.len() != 2 {
            return Err("Invalid worktree setting".into());
        }
        (parts[0].to_string(), parts[1].to_string())
    };

    // Commit any uncommitted changes
    let status = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&worktree_path)
        .output()
        .await
        .map_err(|e| format!("Failed to check status: {e}"))?;

    if !String::from_utf8_lossy(&status.stdout).trim().is_empty() {
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&worktree_path)
            .output()
            .await;

        let _ = Command::new("git")
            .args(["commit", "-m", &title])
            .current_dir(&worktree_path)
            .output()
            .await;
    }

    // Push the branch
    let push = Command::new("git")
        .args(["push", "-u", "origin", &branch])
        .current_dir(&worktree_path)
        .output()
        .await
        .map_err(|e| format!("Failed to push: {e}"))?;

    if !push.status.success() {
        let stderr = String::from_utf8_lossy(&push.stderr);
        return Err(format!("Push failed: {stderr}"));
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
        .args([
            "pr", "create",
            "--title", &title,
            "--body", &body,
            "--head", &branch,
            "--base", &base,
        ])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to create PR: {e}"))?;

    if !pr.status.success() {
        let stderr = String::from_utf8_lossy(&pr.stderr);
        return Err(format!("gh pr create failed: {stderr}"));
    }

    let pr_url = String::from_utf8_lossy(&pr.stdout).trim().to_string();

    // Extract PR number from URL and store the mapping
    if let Some(num_str) = pr_url.rsplit('/').next() {
        if let Ok(num) = num_str.parse::<i64>() {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = codeforge_persistence::queries::set_setting(
                db.conn(),
                &format!("pr:{thread_id}"),
                &num.to_string(),
            );
        }
    }

    Ok(pr_url)
}

/// Fetch CI check status and PR review status for a thread's linked PR.
#[tauri::command]
pub async fn get_pr_status(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<Option<PrStatus>, String> {
    // Look up PR number
    let pr_number = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        codeforge_persistence::queries::get_setting(
            db.conn(),
            &format!("pr:{thread_id}"),
        )
        .ok()
        .flatten()
    };

    let pr_num = match pr_number {
        Some(n) => n,
        None => return Ok(None),
    };

    // Get CI checks
    let checks_out = Command::new("gh")
        .args(["pr", "checks", &pr_num, "--json", "name,state,conclusion", "--jq", "[.[] | {name, state, conclusion}]"])
        .current_dir(&project_path)
        .output()
        .await;

    let ci_status = match checks_out {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if raw.contains("FAILURE") || raw.contains("failure") {
                "failure".to_string()
            } else if raw.contains("PENDING") || raw.contains("pending") || raw.contains("IN_PROGRESS") {
                "pending".to_string()
            } else if raw.contains("SUCCESS") || raw.contains("success") {
                "success".to_string()
            } else if raw.is_empty() || raw == "[]" {
                "none".to_string()
            } else {
                "unknown".to_string()
            }
        }
        _ => "none".to_string(),
    };

    // Get review status
    let review_out = Command::new("gh")
        .args(["pr", "view", &pr_num, "--json", "reviewDecision", "--jq", ".reviewDecision"])
        .current_dir(&project_path)
        .output()
        .await;

    let review_status = match review_out {
        Ok(o) if o.status.success() => {
            let raw = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if raw.is_empty() { "none".to_string() } else { raw.to_lowercase() }
        }
        _ => "none".to_string(),
    };

    // Get review comments count
    let comments_out = Command::new("gh")
        .args(["pr", "view", &pr_num, "--json", "comments,reviews", "--jq", "(.comments | length) + (.reviews | length)"])
        .current_dir(&project_path)
        .output()
        .await;

    let comment_count = match comments_out {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout).trim().parse::<u32>().unwrap_or(0)
        }
        _ => 0,
    };

    Ok(Some(PrStatus {
        pr_number: pr_num.parse().unwrap_or(0),
        ci_status,
        review_status,
        comment_count,
    }))
}

#[derive(Debug, Serialize)]
pub struct PrStatus {
    pub pr_number: u32,
    pub ci_status: String,
    pub review_status: String,
    pub comment_count: u32,
}

/// Fetch new PR review comments and return them as text.
#[tauri::command]
pub async fn get_pr_review_comments(
    state: State<'_, TauriState>,
    thread_id: String,
    project_path: String,
) -> Result<Vec<PrComment>, String> {
    let pr_number = {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        codeforge_persistence::queries::get_setting(
            db.conn(),
            &format!("pr:{thread_id}"),
        )
        .ok()
        .flatten()
    };

    let pr_num = match pr_number {
        Some(n) => n,
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

    // Parse NDJSON lines from jq output
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
    // Scan settings for pr:<thread_id> = <pr_number>
    let mut stmt = db.conn()
        .prepare("SELECT key, value FROM settings WHERE key LIKE 'pr:%'")
        .map_err(|e| format!("{e}"))?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    }).map_err(|e| format!("{e}"))?;

    for row in rows {
        if let Ok((key, val)) = row {
            if val == pr_number.to_string() {
                // key is "pr:<thread_id>"
                return Ok(Some(key.strip_prefix("pr:").unwrap_or(&key).to_string()));
            }
        }
    }
    Ok(None)
}

/// Checkout a PR's branch into a new worktree for a thread.
#[tauri::command]
pub async fn checkout_pr_into_worktree(
    state: State<'_, TauriState>,
    thread_id: String,
    pr_number: u32,
    project_path: String,
) -> Result<WorktreeInfo, String> {
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

    let output = Command::new("git")
        .args(["worktree", "add", &worktree_path, &format!("origin/{pr_branch}")])
        .current_dir(&project_path)
        .output()
        .await
        .map_err(|e| format!("Failed to create worktree: {e}"))?;

    if !output.status.success() {
        // Try with -b to create a local tracking branch
        let output2 = Command::new("git")
            .args(["worktree", "add", "-b", &pr_branch, &worktree_path, &format!("origin/{pr_branch}")])
            .current_dir(&project_path)
            .output()
            .await
            .map_err(|e| format!("Failed to create worktree: {e}"))?;

        if !output2.status.success() {
            return Err(format!("Failed to checkout PR branch: {}", String::from_utf8_lossy(&output2.stderr)));
        }
    }

    // Store worktree mapping
    {
        let db = state.db.lock().map_err(|e| format!("{e}"))?;
        let _ = codeforge_persistence::queries::set_setting(
            db.conn(),
            &format!("worktree:{thread_id}"),
            &format!("{pr_branch}|{worktree_path}"),
        );
        // Link PR to thread
        let _ = codeforge_persistence::queries::set_setting(
            db.conn(),
            &format!("pr:{thread_id}"),
            &pr_number.to_string(),
        );
    }

    Ok(WorktreeInfo {
        thread_id,
        branch: pr_branch,
        path: worktree_path,
        active: true,
    })
}
