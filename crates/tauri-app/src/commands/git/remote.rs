//! Remote git operations: fetch, push, pull, PR creation, remote update checks.

use tokio::process::Command;

use super::{GitDiffStat, RemoteUpdate};

// ── Helpers ──

/// Resolve the remote branch to check/pull. PR number → `gh pr view`,
/// else explicit branch, else current branch.
async fn resolve_remote_branch(
    cwd: &str,
    branch: &Option<String>,
    pr_number: &Option<String>,
) -> Result<String, String> {
    if let Some(pr) = pr_number {
        let out = Command::new("gh")
            .args(["pr", "view", pr, "--json", "headRefName", "--jq", ".headRefName"])
            .current_dir(cwd)
            .output()
            .await;
        if let Ok(o) = out {
            if o.status.success() {
                let b = String::from_utf8_lossy(&o.stdout).trim().to_string();
                if !b.is_empty() {
                    return Ok(b);
                }
            }
        }
    }

    match branch {
        Some(b) if !b.is_empty() => Ok(b.clone()),
        _ => {
            let out = Command::new("git")
                .args(["branch", "--show-current"])
                .current_dir(cwd)
                .output()
                .await
                .map_err(|e| format!("Failed to get branch: {e}"))?;
            Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
        }
    }
}

// ── Commands ──

#[tauri::command]
pub async fn git_fetch(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["fetch", "--all", "--prune"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git fetch: {e}"))?;

    if !output.status.success() {
        return Err(format!("git fetch failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    Ok(if !stderr.trim().is_empty() { stderr.trim().to_string() } else { "Fetch successful".to_string() })
}

#[tauri::command]
pub async fn git_pull(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["pull"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git pull: {e}"))?;

    if !output.status.success() {
        return Err(format!("git pull failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub async fn git_push(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["push"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git push: {e}"))?;

    if !output.status.success() {
        return Err(format!("git push failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        "Push successful".to_string()
    })
}

#[tauri::command]
pub async fn git_push_force(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["push", "--force-with-lease"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git push --force-with-lease: {e}"))?;

    if !output.status.success() {
        return Err(format!("git push --force-with-lease failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        "Force push successful".to_string()
    })
}

#[tauri::command]
pub async fn git_create_pr(
    cwd: String,
    title: String,
    body: String,
    branch: String,
    base: String,
) -> Result<String, String> {
    let output = Command::new("gh")
        .args(["pr", "create", "--title", &title, "--body", &body, "--head", &branch, "--base", &base])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run gh pr create: {e}"))?;

    if !output.status.success() {
        return Err(format!("gh pr create failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

#[tauri::command]
pub async fn git_diff_branches(cwd: String, branch1: String, branch2: String) -> Result<Vec<GitDiffStat>, String> {
    let range = format!("{branch1}...{branch2}");
    let output = Command::new("git")
        .args(["diff", &range, "--numstat"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !output.status.success() {
        return Err(format!("git diff failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            (parts.len() >= 3).then(|| GitDiffStat {
                file: parts[2].to_string(),
                insertions: parts[0].parse().unwrap_or(0),
                deletions: parts[1].parse().unwrap_or(0),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn git_check_remote(
    cwd: String,
    branch: Option<String>,
    pr_number: Option<String>,
) -> Result<Option<RemoteUpdate>, String> {
    let _ = Command::new("git")
        .args(["fetch", "--quiet"])
        .current_dir(&cwd)
        .output()
        .await;

    let check_branch = resolve_remote_branch(&cwd, &branch, &pr_number).await?;
    if check_branch.is_empty() {
        return Ok(None);
    }

    let remote_ref = format!("origin/{check_branch}");
    let rev_list = Command::new("git")
        .args(["rev-list", "--count", &format!("HEAD..{remote_ref}")])
        .current_dir(&cwd)
        .output()
        .await;

    let behind = match rev_list {
        Ok(out) if out.status.success() => {
            String::from_utf8_lossy(&out.stdout).trim().parse::<u32>().unwrap_or(0)
        }
        _ => return Ok(None),
    };

    if behind == 0 {
        return Ok(None);
    }

    let log = Command::new("git")
        .args(["log", &remote_ref, "-1", "--format=%s"])
        .current_dir(&cwd)
        .output()
        .await;

    let latest_message = match log {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    };

    Ok(Some(RemoteUpdate { branch: check_branch, behind, latest_message }))
}

#[tauri::command]
pub async fn git_pull_branch(
    cwd: String,
    branch: Option<String>,
    pr_number: Option<String>,
) -> Result<String, String> {
    let remote_branch = resolve_remote_branch(&cwd, &branch, &pr_number).await?;
    if remote_branch.is_empty() {
        return Err("Could not determine remote branch".into());
    }

    let output = Command::new("git")
        .args(["pull", "origin", &remote_branch])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git pull: {e}"))?;

    if !output.status.success() {
        return Err(format!("git pull failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}
