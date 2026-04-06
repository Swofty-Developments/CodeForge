use serde::Serialize;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct BlameLine {
    pub line_number: u32,
    pub commit_hash: String,
    pub author: String,
    pub date: String,
    pub content: String,
}

/// Get blame information for a file.
#[tauri::command]
pub fn get_file_blame(
    cwd: String,
    file_path: String,
    revision: Option<String>,
) -> Result<Vec<BlameLine>, String> {
    let mut args = vec!["blame", "--porcelain"];
    if let Some(ref rev) = revision {
        args.push(rev);
    }
    args.push("--");
    args.push(&file_path);

    let output = Command::new("git")
        .args(&args)
        .current_dir(&cwd)
        .output()
        .map_err(|e| format!("Failed to run git blame: {e}"))?;

    if !output.status.success() {
        return Err(format!("git blame failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let raw = String::from_utf8_lossy(&output.stdout);
    parse_porcelain_blame(&raw)
}

fn parse_porcelain_blame(raw: &str) -> Result<Vec<BlameLine>, String> {
    let mut lines = Vec::new();
    let mut current_hash = String::new();
    let mut current_author = String::new();
    let mut current_date = String::new();
    let mut current_line_number: u32 = 0;

    for line in raw.lines() {
        if line.len() >= 40 && line.chars().take(40).all(|c| c.is_ascii_hexdigit()) {
            // This is a commit line: <hash> <orig_line> <final_line> [<num_lines>]
            let parts: Vec<&str> = line.split_whitespace().collect();
            current_hash = parts.first().unwrap_or(&"").to_string();
            current_line_number = parts.get(2).and_then(|s| s.parse().ok()).unwrap_or(0);
        } else if let Some(author) = line.strip_prefix("author ") {
            current_author = author.to_string();
        } else if let Some(time) = line.strip_prefix("author-time ") {
            // Unix timestamp -> date string
            if let Ok(ts) = time.parse::<i64>() {
                current_date = chrono::DateTime::from_timestamp(ts, 0)
                    .map(|dt| dt.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
            }
        } else if let Some(content) = line.strip_prefix('\t') {
            lines.push(BlameLine {
                line_number: current_line_number,
                commit_hash: if current_hash.len() > 8 { current_hash[..8].to_string() } else { current_hash.clone() },
                author: current_author.clone(),
                date: current_date.clone(),
                content: content.to_string(),
            });
        }
    }

    Ok(lines)
}
