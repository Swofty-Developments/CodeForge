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
    let branches = stdout
        .lines()
        .filter(|l| !l.is_empty())
        .filter(|l| !l.contains("HEAD ->"))
        .map(|line| {
            let current = line.starts_with('*');
            let name = line
                .trim_start_matches('*')
                .trim()
                .to_string();
            let remote = name.starts_with("remotes/");
            let name = if remote {
                name.trim_start_matches("remotes/").to_string()
            } else {
                name
            };
            GitBranch {
                name,
                current,
                remote,
            }
        })
        .collect();

    Ok(branches)
}

#[tauri::command]
pub async fn git_checkout(cwd: String, branch: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["checkout", &branch])
        .current_dir(&cwd)
        .output()
        .await
        .map_err(|e| format!("Failed to run git checkout: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("git checkout failed: {stderr}"));
    }

    Ok(format!("Switched to branch '{branch}'"))
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
        // git push writes progress to stderr even on success, check exit code
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
