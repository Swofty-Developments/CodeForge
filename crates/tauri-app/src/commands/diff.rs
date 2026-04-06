use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct ChangedFile {
    pub path: String,
    pub status: String,
    pub insertions: u32,
    pub deletions: u32,
}

#[derive(Debug, Serialize)]
pub struct FileDiff {
    pub path: String,
    pub hunks: Vec<DiffHunk>,
}

#[derive(Debug, Serialize)]
pub struct DiffHunk {
    pub header: String,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Serialize)]
pub struct DiffLine {
    pub line_type: String,
    pub content: String,
    pub old_line: Option<u32>,
    pub new_line: Option<u32>,
}

fn parse_status_char(c: char) -> &'static str {
    match c {
        'M' => "modified",
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        '?' => "added",
        _ => "modified",
    }
}

#[tauri::command]
pub fn get_changed_files(cwd: String) -> Result<Vec<ChangedFile>, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to run git status: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "git status failed: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut files: Vec<ChangedFile> = Vec::new();

    for line in stdout.lines() {
        if line.len() < 3 {
            continue;
        }
        let index_status = line.chars().nth(0).unwrap_or(' ');
        let worktree_status = line.chars().nth(1).unwrap_or(' ');
        let path = line[3..].trim().to_string();

        // Use the most significant status
        let status_char = if index_status != ' ' && index_status != '?' {
            index_status
        } else {
            worktree_status
        };

        let status = parse_status_char(status_char).to_string();
        files.push(ChangedFile {
            path,
            status,
            insertions: 0,
            deletions: 0,
        });
    }

    // Get insertion/deletion stats via git diff --numstat
    let numstat = Command::new("git")
        .args(["diff", "--numstat", "HEAD"])
        .current_dir(&cwd)
        .output();

    if let Ok(ns) = numstat {
        if ns.status.success() {
            let ns_out = String::from_utf8_lossy(&ns.stdout);
            for line in ns_out.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let ins = parts[0].parse::<u32>().unwrap_or(0);
                    let del = parts[1].parse::<u32>().unwrap_or(0);
                    let path = parts[2];
                    if let Some(f) = files.iter_mut().find(|f| f.path == path) {
                        f.insertions = ins;
                        f.deletions = del;
                    }
                }
            }
        }
    }

    // Also try for untracked/new files
    let numstat_cached = Command::new("git")
        .args(["diff", "--numstat", "--cached"])
        .current_dir(&cwd)
        .output();

    if let Ok(ns) = numstat_cached {
        if ns.status.success() {
            let ns_out = String::from_utf8_lossy(&ns.stdout);
            for line in ns_out.lines() {
                let parts: Vec<&str> = line.split('\t').collect();
                if parts.len() >= 3 {
                    let ins = parts[0].parse::<u32>().unwrap_or(0);
                    let del = parts[1].parse::<u32>().unwrap_or(0);
                    let path = parts[2];
                    if let Some(f) = files.iter_mut().find(|f| f.path == path && f.insertions == 0 && f.deletions == 0) {
                        f.insertions = ins;
                        f.deletions = del;
                    }
                }
            }
        }
    }

    Ok(files)
}

fn parse_unified_diff(raw: &str) -> Vec<FileDiff> {
    let mut diffs = Vec::new();
    let mut current_path: Option<String> = None;
    let mut current_hunks: Vec<DiffHunk> = Vec::new();
    let mut current_hunk: Option<DiffHunk> = None;
    let mut old_line: u32 = 0;
    let mut new_line: u32 = 0;

    for line in raw.lines() {
        if line.starts_with("diff --git ") {
            // Flush previous hunk/file
            if let Some(hunk) = current_hunk.take() {
                current_hunks.push(hunk);
            }
            if let Some(path) = current_path.take() {
                diffs.push(FileDiff {
                    path,
                    hunks: std::mem::take(&mut current_hunks),
                });
            }

            // Parse path from "diff --git a/path b/path"
            let rest = &line["diff --git ".len()..];
            if let Some(b_idx) = rest.rfind(" b/") {
                let path = rest[b_idx + 3..].to_string();
                current_path = Some(path);
            }
        } else if line.starts_with("@@ ") {
            // Flush previous hunk
            if let Some(hunk) = current_hunk.take() {
                current_hunks.push(hunk);
            }

            // Parse hunk header: @@ -old_start,old_count +new_start,new_count @@
            let header = line.to_string();
            if let Some(plus_idx) = line.find('+') {
                let after_plus = &line[plus_idx + 1..];
                let num_end = after_plus
                    .find(|c: char| c != '0' && c != '1' && c != '2' && c != '3' && c != '4' && c != '5' && c != '6' && c != '7' && c != '8' && c != '9')
                    .unwrap_or(after_plus.len());
                new_line = after_plus[..num_end].parse().unwrap_or(1);

                // Parse old line number
                if let Some(minus_idx) = line.find('-') {
                    let after_minus = &line[minus_idx + 1..];
                    let num_end = after_minus
                        .find(|c: char| !c.is_ascii_digit())
                        .unwrap_or(after_minus.len());
                    old_line = after_minus[..num_end].parse().unwrap_or(1);
                }
            }

            current_hunk = Some(DiffHunk {
                header,
                lines: Vec::new(),
            });
        } else if let Some(ref mut hunk) = current_hunk {
            if line.starts_with('+') {
                hunk.lines.push(DiffLine {
                    line_type: "add".to_string(),
                    content: line[1..].to_string(),
                    old_line: None,
                    new_line: Some(new_line),
                });
                new_line += 1;
            } else if line.starts_with('-') {
                hunk.lines.push(DiffLine {
                    line_type: "remove".to_string(),
                    content: line[1..].to_string(),
                    old_line: Some(old_line),
                    new_line: None,
                });
                old_line += 1;
            } else if line.starts_with(' ') {
                hunk.lines.push(DiffLine {
                    line_type: "context".to_string(),
                    content: line[1..].to_string(),
                    old_line: Some(old_line),
                    new_line: Some(new_line),
                });
                old_line += 1;
                new_line += 1;
            } else if line.starts_with('\\') {
                // "\ No newline at end of file" — skip
            }
        }
    }

    // Flush remaining
    if let Some(hunk) = current_hunk.take() {
        current_hunks.push(hunk);
    }
    if let Some(path) = current_path.take() {
        diffs.push(FileDiff {
            path,
            hunks: current_hunks,
        });
    }

    diffs
}

#[tauri::command]
pub fn get_session_diff(cwd: String) -> Result<Vec<FileDiff>, String> {
    // Get both staged and unstaged diffs
    let unstaged = Command::new("git")
        .args(["diff", "HEAD"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !unstaged.status.success() {
        // Fallback: might be an initial commit with no HEAD
        let staged = Command::new("git")
            .args(["diff", "--cached"])
            .current_dir(&cwd)
            .output()
            .map_err(|e| format!("Failed to run git diff: {e}"))?;

        let raw = String::from_utf8_lossy(&staged.stdout);
        return Ok(parse_unified_diff(&raw));
    }

    let raw = String::from_utf8_lossy(&unstaged.stdout);
    Ok(parse_unified_diff(&raw))
}

#[tauri::command]
pub fn get_file_diff(cwd: String, file_path: String) -> Result<String, String> {
    let output = Command::new("git")
        .args(["diff", "HEAD", "--", &file_path])
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !output.status.success() {
        // Fallback for staged-only
        let output2 = Command::new("git")
            .args(["diff", "--cached", "--", &file_path])
            .current_dir(&cwd)
            .output()
            .map_err(|e| format!("Failed to run git diff: {e}"))?;

        return Ok(String::from_utf8_lossy(&output2.stdout).to_string());
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

#[tauri::command]
pub fn get_file_content(cwd: String, file_path: String, version: String) -> Result<String, String> {
    match version.as_str() {
        "head" | "HEAD" => {
            let output = Command::new("git")
                .args(["show", &format!("HEAD:{file_path}")])
                .current_dir(&cwd)
                .output()
                .map_err(|e| format!("Failed to run git show: {e}"))?;

            if !output.status.success() {
                return Ok(String::new()); // File might be new
            }

            Ok(String::from_utf8_lossy(&output.stdout).to_string())
        }
        "working" | "worktree" => {
            let full_path = std::path::Path::new(&cwd).join(&file_path);
            std::fs::read_to_string(&full_path)
                .map_err(|e| format!("Failed to read file: {e}"))
        }
        _ => Err(format!("Unknown version: {version}. Use 'HEAD' or 'working'.")),
    }
}

/// Get diff between a base commit and current HEAD (per-turn diff).
#[tauri::command]
pub fn get_turn_diff(cwd: String, base_commit: String) -> Result<Vec<FileDiff>, String> {
    let output = Command::new("git")
        .args(["diff", &base_commit, "HEAD"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to run git diff: {e}"))?;

    if !output.status.success() {
        return Err(format!("git diff failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    Ok(parse_unified_diff(&raw))
}

/// Get changed files between a base commit and HEAD with stats.
#[tauri::command]
pub fn get_turn_changed_files(cwd: String, base_commit: String) -> Result<Vec<ChangedFile>, String> {
    let output = Command::new("git")
        .args(["diff", "--numstat", &base_commit, "HEAD"])
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("{e}"))?;

    if !output.status.success() {
        return Ok(vec![]);
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    let mut files = Vec::new();

    for line in raw.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let ins = parts[0].parse::<u32>().unwrap_or(0);
            let del = parts[1].parse::<u32>().unwrap_or(0);
            let path = parts[2].to_string();
            let is_binary = parts[0] == "-" && parts[1] == "-";
            files.push(ChangedFile {
                status: if is_binary { "binary".to_string() } else { "modified".to_string() },
                path,
                insertions: ins,
                deletions: del,
            });
        }
    }

    Ok(files)
}
