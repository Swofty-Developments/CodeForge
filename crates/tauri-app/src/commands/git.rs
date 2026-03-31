use serde::Serialize;
use tokio::process::Command;

// ── Types ──

#[derive(Debug, Clone, Serialize)]
pub struct GitLogEntry {
    pub hash: String,
    pub message: String,
    pub author: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitBranch {
    pub name: String,
    pub current: bool,
    pub remote: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitStatusEntry {
    pub path: String,
    pub status: String,
    pub staged: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GitDiffStat {
    pub file: String,
    pub insertions: u32,
    pub deletions: u32,
}

// ── Commands ──

#[tauri::command]
pub async fn git_log(cwd: String, limit: u32) -> Result<Vec<GitLogEntry>, String> {
    let output = Command::new("git")
        .args([
            "log",
            &format!("-{limit}"),
            "--format=%H\x1f%s\x1f%an\x1f%cr",
        ])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git log: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git log failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            if parts.len() >= 4 {
                Some(GitLogEntry {
                    hash: parts[0][..7.min(parts[0].len())].to_string(),
                    message: parts[1].to_string(),
                    author: parts[2].to_string(),
                    date: parts[3].to_string(),
                })
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}

#[tauri::command]
pub async fn git_branches(cwd: String) -> Result<Vec<GitBranch>, String> {
    // Fetch first to ensure we have up-to-date remote refs
    let _ = Command::new("git")
        .args(["fetch", "--all", "--prune"])
        .current_dir(&cwd)
        .output()
        .await;

    let output = Command::new("git")
        .args(["branch", "-a", "--no-color"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git branch: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git branch failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Collect local branch names so we can deduplicate remotes
    let local_names: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("HEAD ->") && !l.contains("remotes/"))
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .collect();

    let branches = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| !l.contains("HEAD ->"))
        .filter_map(|line| {
            let current = line.starts_with('*');
            let raw = line.trim_start_matches('*').trim().to_string();
            let remote = raw.starts_with("remotes/");
            let name = if remote {
                // Strip remotes/origin/ prefix
                raw.strip_prefix("remotes/origin/")
                    .unwrap_or(raw.strip_prefix("remotes/").unwrap_or(&raw))
                    .to_string()
            } else {
                raw
            };
            // Skip remote branches that already have a local counterpart
            if remote && local_names.contains(&name) {
                return None;
            }
            Some(GitBranch {
                name,
                current,
                remote,
            })
        })
        .collect();

    Ok(branches)
}

#[tauri::command]
pub async fn git_checkout(cwd: String, branch: String) -> Result<String, String> {
    // Fetch origin first so remote branches are available
    let _ = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(&cwd)
        .output()
        .await;

    let output = Command::new("git")
        .args(["checkout", &branch])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout: {e}"))?;

    if output.status.success() {
        return Ok(format!("Switched to branch '{branch}'"));
    }

    // Local checkout failed — try tracking the remote branch
    let output2 = Command::new("git")
        .args(["checkout", "-b", &branch, &format!("origin/{branch}")])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout -b: {e}"))?;

    if output2.status.success() {
        return Ok(format!("Created local branch '{branch}' tracking origin/{branch}"));
    }

    let stderr = String::from_utf8_lossy(&output2.stderr);
    Err(format!("git checkout failed: {stderr}"))
}

#[tauri::command]
pub async fn git_create_branch(cwd: String, name: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", "-b", &name])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout -b: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git checkout -b failed: {stderr}"));
    }

    Ok(format!("Created and switched to branch '{name}'"))
}

#[tauri::command]
pub async fn git_commit(cwd: String, message: String, files: Vec<String>) -> Result<String, String> {
    if files.is_empty() {
        return Err("No files specified for commit".to_string());
    }

    // Stage files
    let mut add_args = vec!["add".to_string(), "--".to_string()];
    add_args.extend(files.clone());

    let add_output = Command::new("git")
        .args(&add_args)
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git add: {e}"))?;

    if !add_output.status.success() {
        let stderr = String::from_utf8_lossy(&add_output.stderr);
        return Err(format!("git add failed: {stderr}"));
    }

    // Commit
    let commit_output = Command::new("git")
        .args(["commit", "-m", &message])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git commit: {e}"))?;

    if !commit_output.status.success() {
        let stderr = String::from_utf8_lossy(&commit_output.stderr);
        return Err(format!("git commit failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&commit_output.stdout);
    Ok(stdout.lines().next().unwrap_or("Committed").to_string())
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git push failed: {stderr}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        "Push successful".to_string()
    };

    Ok(msg)
}

#[tauri::command]
pub async fn git_fetch(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["fetch", "--all", "--prune"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git fetch: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git fetch failed: {stderr}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let msg = if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        "Fetch successful".to_string()
    };

    Ok(msg)
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git pull failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git push --force-with-lease failed: {stderr}"));
    }

    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let msg = if !stdout.trim().is_empty() {
        stdout.trim().to_string()
    } else if !stderr.trim().is_empty() {
        stderr.trim().to_string()
    } else {
        "Force push successful".to_string()
    };

    Ok(msg)
}

#[tauri::command]
pub async fn git_delete_branch(cwd: String, name: String, force: bool) -> Result<String, String> {
    let flag = if force { "-D" } else { "-d" };
    let output = Command::new("git")
        .args(["branch", flag, &name])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git branch {flag}: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if !force && stderr.contains("not fully merged") {
            return Err(format!(
                "Branch '{name}' is not fully merged. Use force delete to remove it anyway."
            ));
        }
        return Err(format!("git branch {flag} failed: {stderr}"));
    }

    Ok(format!("Deleted branch '{name}'"))
}

#[tauri::command]
pub async fn git_merge_branch(cwd: String, branch: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["merge", &branch, "--no-edit"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git merge: {e}"))?;

    if output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Ok(stdout.trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    // Abort the failed merge to leave repo in clean state
    let _ = Command::new("git")
        .args(["merge", "--abort"])
        .current_dir(&cwd)
        .output()
        .await;

    Err(format!("Merge conflicts detected, merge aborted:\n{stderr}"))
}

#[tauri::command]
pub async fn git_stash(cwd: String, message: Option<String>) -> Result<String, String> {
    let mut args = vec!["stash", "push"];
    let msg;
    if let Some(ref m) = message {
        args.push("-m");
        msg = m.clone();
        args.push(&msg);
    }

    let output = Command::new("git")
        .args(&args)
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git stash: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git stash failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

#[tauri::command]
pub async fn git_stash_pop(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["stash", "pop"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git stash pop: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git stash pop failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
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
        .args([
            "pr", "create",
            "--title", &title,
            "--body", &body,
            "--head", &branch,
            "--base", &base,
        ])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run gh pr create: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr create failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout.trim().to_string())
}

#[tauri::command]
pub async fn git_diff_branches(
    cwd: String,
    branch1: String,
    branch2: String,
) -> Result<Vec<GitDiffStat>, String> {
    let range = format!("{branch1}...{branch2}");
    let output = Command::new("git")
        .args(["diff", &range, "--numstat"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git diff failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stats = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let insertions = parts[0].parse::<u32>().unwrap_or(0);
                let deletions = parts[1].parse::<u32>().unwrap_or(0);
                Some(GitDiffStat {
                    file: parts[2].to_string(),
                    insertions,
                    deletions,
                })
            } else {
                None
            }
        })
        .collect();

    Ok(stats)
}

#[tauri::command]
pub async fn git_status(cwd: String) -> Result<Vec<GitStatusEntry>, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git status: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git status failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let entries = stdout
        .lines()
        .filter(|l| l.len() >= 3)
        .map(|line| {
            let index_char = line.chars().next().unwrap_or(' ');
            let worktree_char = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].trim().to_string();

            let staged = index_char != ' ' && index_char != '?';

            let status_char = if index_char != ' ' && index_char != '?' {
                index_char
            } else {
                worktree_char
            };

            let status = match status_char {
                'M' => "modified",
                'A' => "added",
                'D' => "deleted",
                'R' => "renamed",
                'C' => "copied",
                '?' => "untracked",
                _ => "modified",
            }
            .to_string();

            GitStatusEntry {
                path,
                status,
                staged,
            }
        })
        .collect();

    Ok(entries)
}
