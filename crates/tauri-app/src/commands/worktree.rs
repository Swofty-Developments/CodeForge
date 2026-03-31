use serde::Serialize;
use std::process::Command;
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
pub fn create_worktree(
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
    let branch = format!("codeforge/{sanitized}-{}", &thread_id[..8]);
    let worktree_path = format!("{project_path}/.codeforge-worktrees/{sanitized}");

    // Create the worktree directory parent
    std::fs::create_dir_all(format!("{project_path}/.codeforge-worktrees"))
        .map_err(|e| format!("Failed to create worktree directory: {e}"))?;

    // Create a new branch and worktree
    let output = Command::new("git")
        .args(["worktree", "add", "-b", &branch, &worktree_path])
        .current_dir(&project_path)
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        // Branch might already exist, try without -b
        let output2 = Command::new("git")
            .args(["worktree", "add", &worktree_path, &branch])
            .current_dir(&project_path)
            .output()
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
pub fn merge_worktree(
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

    if let Some(_pr_num) = &pr_number {
        // PR mode: commit and push the worktree branch (don't merge to main)
        // First, commit any uncommitted changes in the worktree
        let status = Command::new("git")
            .args(["status", "--porcelain"])
            .current_dir(&worktree_path)
            .output()
            .map_err(|e| format!("Failed to check status: {e}"))?;

        let has_changes = !String::from_utf8_lossy(&status.stdout).trim().is_empty();

        if has_changes {
            let _ = Command::new("git")
                .args(["add", "-A"])
                .current_dir(&worktree_path)
                .output();

            let _ = Command::new("git")
                .args(["commit", "-m", "Changes from CodeForge"])
                .current_dir(&worktree_path)
                .output();
        }

        // Push the branch
        let push = Command::new("git")
            .args(["push", "origin", &branch])
            .current_dir(&worktree_path)
            .output()
            .map_err(|e| format!("Failed to push: {e}"))?;

        if !push.status.success() {
            let stderr = String::from_utf8_lossy(&push.stderr);
            return Err(format!("Push failed: {stderr}"));
        }

        // Remove worktree but keep branch (PR is still open)
        let _ = Command::new("git")
            .args(["worktree", "remove", &worktree_path, "--force"])
            .current_dir(&project_path)
            .output();

        // Remove worktree setting
        {
            let db = state.db.lock().map_err(|e| format!("{e}"))?;
            let _ = codeforge_persistence::queries::delete_setting(
                db.conn(),
                &format!("worktree:{thread_id}"),
            );
        }

        Ok(format!("Pushed {branch} to origin"))
    } else {
        // Normal mode: merge worktree branch into main
        let merge = Command::new("git")
            .args(["merge", &branch, "--no-edit"])
            .current_dir(&project_path)
            .output()
            .map_err(|e| format!("Failed to merge: {e}"))?;

        if !merge.status.success() {
            let stderr = String::from_utf8_lossy(&merge.stderr).to_string();

            // Abort the failed merge to leave repo in clean state
            let _ = Command::new("git")
                .args(["merge", "--abort"])
                .current_dir(&project_path)
                .output();

            return Err(format!(
                "Merge has conflicts. Resolve them manually in the worktree at {worktree_path} then try again.\n\nConflict details: {stderr}"
            ));
        }

        // Remove worktree
        let _ = Command::new("git")
            .args(["worktree", "remove", &worktree_path, "--force"])
            .current_dir(&project_path)
            .output();

        // Delete the branch
        let _ = Command::new("git")
            .args(["branch", "-d", &branch])
            .current_dir(&project_path)
            .output();

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
