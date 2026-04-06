//! Local git operations: log, branches, checkout, commit, status, stash, merge.

use tokio::process::Command;

use super::{GitBranch, GitLogEntry, GitStatusEntry, RepoStatus};

#[tauri::command]
pub async fn git_log(cwd: String, limit: u32) -> Result<Vec<GitLogEntry>, String> {
    let output = Command::new("git")
        .args(["log", &format!("-{limit}"), "--format=%H\x1f%s\x1f%an\x1f%cr"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git log: {e}"))?;

    if !output.status.success() {
        return Err(format!("git log failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(4, '\x1f').collect();
            (parts.len() >= 4).then(|| GitLogEntry {
                hash: parts[0][..7.min(parts[0].len())].to_string(),
                message: parts[1].to_string(),
                author: parts[2].to_string(),
                date: parts[3].to_string(),
            })
        })
        .collect())
}

#[tauri::command]
pub async fn git_branches(cwd: String) -> Result<Vec<GitBranch>, String> {
    let output = Command::new("git")
        .args(["branch", "-a", "--no-color"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git branch: {e}"))?;

    if !output.status.success() {
        return Err(format!("git branch failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let local_names: Vec<String> = stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("HEAD ->") && !l.contains("remotes/"))
        .map(|line| line.trim_start_matches('*').trim().to_string())
        .collect();

    Ok(stdout
        .lines()
        .filter(|l| !l.is_empty() && !l.contains("HEAD ->"))
        .filter_map(|line| {
            let current = line.starts_with('*');
            let raw = line.trim_start_matches('*').trim().to_string();
            let remote = raw.starts_with("remotes/");
            let name = if remote {
                raw.strip_prefix("remotes/origin/")
                    .unwrap_or(raw.strip_prefix("remotes/").unwrap_or(&raw))
                    .to_string()
            } else {
                raw
            };
            if remote && local_names.contains(&name) {
                return None;
            }
            Some(GitBranch { name, current, remote })
        })
        .collect())
}

#[tauri::command]
pub async fn git_checkout(cwd: String, branch: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", &branch])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout: {e}"))?;

    if output.status.success() {
        return Ok(format!("Switched to branch '{branch}'"));
    }

    // Try tracking the remote branch
    let output2 = Command::new("git")
        .args(["checkout", "-b", &branch, &format!("origin/{branch}")])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout -b: {e}"))?;

    if output2.status.success() {
        return Ok(format!("Created local branch '{branch}' tracking origin/{branch}"));
    }

    Err(format!("git checkout failed: {}", String::from_utf8_lossy(&output2.stderr)))
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
        return Err(format!("git checkout -b failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(format!("Created and switched to branch '{name}'"))
}

#[tauri::command]
pub async fn git_commit(cwd: String, message: String, files: Vec<String>) -> Result<String, String> {
    if files.is_empty() {
        return Err("No files specified for commit".to_string());
    }

    let mut add_args = vec!["add".to_string(), "--".to_string()];
    add_args.extend(files);

    let add_output = Command::new("git")
        .args(&add_args)
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git add: {e}"))?;

    if !add_output.status.success() {
        return Err(format!("git add failed: {}", String::from_utf8_lossy(&add_output.stderr)));
    }

    let commit_output = Command::new("git")
        .args(["commit", "-m", &message])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git commit: {e}"))?;

    if !commit_output.status.success() {
        return Err(format!("git commit failed: {}", String::from_utf8_lossy(&commit_output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&commit_output.stdout);
    Ok(stdout.lines().next().unwrap_or("Committed").to_string())
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
        return Err(format!("git status failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    Ok(stdout
        .lines()
        .filter(|l| l.len() >= 3)
        .map(|line| {
            let index_char = line.chars().next().unwrap_or(' ');
            let worktree_char = line.chars().nth(1).unwrap_or(' ');
            let path = line[3..].trim().to_string();
            let staged = index_char != ' ' && index_char != '?';
            let status_char = if staged { index_char } else { worktree_char };
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
            GitStatusEntry { path, status, staged }
        })
        .collect())
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
            return Err(format!("Branch '{name}' is not fully merged. Use force delete to remove it anyway."));
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
        return Ok(String::from_utf8_lossy(&output.stdout).trim().to_string());
    }

    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
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
        return Err(format!("git stash failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
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
        return Err(format!("git stash pop failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Initialize a git repo in the given directory. Idempotent — safe to call on existing repos.
#[tauri::command]
pub async fn git_init_repo(cwd: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["init"])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git init: {e}"))?;

    if !output.status.success() {
        return Err(format!("git init failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    // Make an initial commit if the repo is brand new (no commits yet)
    let has_commits = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&cwd)
        .output()
        .await;

    if !matches!(has_commits, Ok(ref o) if o.status.success()) {
        let _ = Command::new("git")
            .args(["add", "-A"])
            .current_dir(&cwd)
            .output()
            .await;
        let _ = Command::new("git")
            .args(["commit", "-m", "Initial commit", "--allow-empty"])
            .current_dir(&cwd)
            .output()
            .await;
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Check the git status of a directory: not a repo, local git, or GitHub-connected.
#[tauri::command]
pub async fn git_repo_status(cwd: String) -> Result<RepoStatus, String> {
    // Check if it's a git repo at all
    let is_git = Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(&cwd)
        .output()
        .await;

    if !matches!(is_git, Ok(ref o) if o.status.success()) {
        return Ok(RepoStatus { status: "none".to_string(), branch: None, has_remote: false });
    }

    // Get current branch
    let branch_out = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&cwd)
        .output()
        .await;
    let branch = match branch_out {
        Ok(o) if o.status.success() => {
            let b = String::from_utf8_lossy(&o.stdout).trim().to_string();
            if b.is_empty() { None } else { Some(b) }
        }
        _ => None,
    };

    // Check if origin remote exists
    let remote_out = Command::new("git")
        .args(["remote", "get-url", "origin"])
        .current_dir(&cwd)
        .output()
        .await;
    let has_remote = matches!(remote_out, Ok(ref o) if o.status.success());

    // Check if it's a GitHub repo (has github.com in the remote URL)
    let is_github = if has_remote {
        match remote_out {
            Ok(ref o) if o.status.success() => {
                String::from_utf8_lossy(&o.stdout).contains("github.com")
            }
            _ => false,
        }
    } else {
        false
    };

    let status = if is_github { "github" } else { "git" };
    Ok(RepoStatus { status: status.to_string(), branch, has_remote })
}
