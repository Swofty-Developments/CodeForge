use serde::Serialize;
use tokio::process::Command;

// ── Types ──

#[derive(Debug, Clone, Serialize)]
pub struct GhAuthStatus {
    pub logged_in: bool,
    pub username: Option<String>,
    pub scopes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PullRequest {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub branch: String,
    pub base: String,
    pub url: String,
    pub additions: u64,
    pub deletions: u64,
    pub changed_files: u64,
    pub created_at: String,
    pub updated_at: String,
    pub draft: bool,
    pub labels: Vec<String>,
    pub review_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct Issue {
    pub number: u64,
    pub title: String,
    pub state: String,
    pub author: String,
    pub body: String,
    pub url: String,
    pub labels: Vec<String>,
    pub created_at: String,
    pub comments_count: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct IssueComment {
    pub author: String,
    pub body: String,
    pub created_at: String,
}

// ── Auth ──

#[tauri::command]
pub async fn gh_auth_status() -> Result<GhAuthStatus, String> {
    let output = Command::new("gh")
        .args(["auth", "status", "--hostname", "github.com"])
        .output()
        .await
        .map_err(|e| format!("gh not found: {e}"))?;

    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    if !output.status.success() && !combined.contains("Logged in") {
        return Ok(GhAuthStatus {
            logged_in: false,
            username: None,
            scopes: vec![],
        });
    }

    // Parse username from output like "Logged in to github.com account username"
    let username = combined
        .lines()
        .find(|l| l.contains("Logged in") || l.contains("account"))
        .and_then(|l| {
            // Try "account <username>" pattern
            if let Some(idx) = l.find("account ") {
                let rest = &l[idx + 8..];
                Some(rest.split_whitespace().next().unwrap_or("").trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_').to_string())
            } else {
                None
            }
        })
        .filter(|s| !s.is_empty());

    // Parse scopes
    let scopes = combined
        .lines()
        .find(|l| l.contains("Token scopes") || l.contains("scopes"))
        .map(|l| {
            l.split(':')
                .last()
                .unwrap_or("")
                .split(',')
                .map(|s| s.trim().trim_matches('\'').to_string())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();

    Ok(GhAuthStatus {
        logged_in: true,
        username,
        scopes,
    })
}

#[tauri::command]
pub async fn gh_login() -> Result<String, String> {
    // Start gh auth login in a way that opens the browser
    let output = Command::new("gh")
        .args(["auth", "login", "--hostname", "github.com", "--web", "--scopes", "repo,read:org"])
        .output()
        .await
        .map_err(|e| format!("Failed to start gh auth: {e}"))?;

    if output.status.success() {
        Ok("Authentication successful".to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Authentication failed: {stderr}"))
    }
}

// ── Pull Requests ──

#[tauri::command]
pub async fn list_prs(repo_path: String, state: Option<String>) -> Result<Vec<PullRequest>, String> {
    let st = state.unwrap_or_else(|| "open".to_string());
    let output = Command::new("gh")
        .args([
            "pr", "list",
            "--state", &st,
            "--json", "number,title,state,author,headRefName,baseRefName,url,additions,deletions,changedFiles,createdAt,updatedAt,isDraft,labels,reviewDecision",
            "--limit", "50",
        ])
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to list PRs: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr list failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse PR list: {e}"))?;

    let prs = raw
        .iter()
        .map(|pr| PullRequest {
            number: pr["number"].as_u64().unwrap_or(0),
            title: pr["title"].as_str().unwrap_or("").to_string(),
            state: pr["state"].as_str().unwrap_or("OPEN").to_string(),
            author: pr["author"]["login"].as_str().unwrap_or("").to_string(),
            branch: pr["headRefName"].as_str().unwrap_or("").to_string(),
            base: pr["baseRefName"].as_str().unwrap_or("main").to_string(),
            url: pr["url"].as_str().unwrap_or("").to_string(),
            additions: pr["additions"].as_u64().unwrap_or(0),
            deletions: pr["deletions"].as_u64().unwrap_or(0),
            changed_files: pr["changedFiles"].as_u64().unwrap_or(0),
            created_at: pr["createdAt"].as_str().unwrap_or("").to_string(),
            updated_at: pr["updatedAt"].as_str().unwrap_or("").to_string(),
            draft: pr["isDraft"].as_bool().unwrap_or(false),
            labels: pr["labels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|l| l["name"].as_str().map(String::from)).collect())
                .unwrap_or_default(),
            review_status: pr["reviewDecision"].as_str().unwrap_or("").to_string(),
        })
        .collect();

    Ok(prs)
}

#[tauri::command]
pub async fn get_pr_diff(repo_path: String, pr_number: u64) -> Result<String, String> {
    let output = Command::new("gh")
        .args(["pr", "diff", &pr_number.to_string()])
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get PR diff: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh pr diff failed: {stderr}"));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

// ── Issues ──

#[tauri::command]
pub async fn list_issues(
    repo_path: String,
    state: Option<String>,
    search: Option<String>,
) -> Result<Vec<Issue>, String> {
    let st = state.unwrap_or_else(|| "open".to_string());
    let mut args = vec![
        "issue".to_string(),
        "list".to_string(),
        "--state".to_string(),
        st,
        "--json".to_string(),
        "number,title,state,author,body,url,labels,createdAt,comments".to_string(),
        "--limit".to_string(),
        "30".to_string(),
    ];

    if let Some(q) = search {
        if !q.is_empty() {
            args.push("--search".to_string());
            args.push(q);
        }
    }

    let output = Command::new("gh")
        .args(&args)
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to list issues: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh issue list failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let raw: Vec<serde_json::Value> = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse issues: {e}"))?;

    let issues = raw
        .iter()
        .map(|iss| Issue {
            number: iss["number"].as_u64().unwrap_or(0),
            title: iss["title"].as_str().unwrap_or("").to_string(),
            state: iss["state"].as_str().unwrap_or("OPEN").to_string(),
            author: iss["author"]["login"].as_str().unwrap_or("").to_string(),
            body: iss["body"].as_str().unwrap_or("").to_string(),
            url: iss["url"].as_str().unwrap_or("").to_string(),
            labels: iss["labels"]
                .as_array()
                .map(|arr| arr.iter().filter_map(|l| l["name"].as_str().map(String::from)).collect())
                .unwrap_or_default(),
            created_at: iss["createdAt"].as_str().unwrap_or("").to_string(),
            comments_count: iss["comments"]
                .as_array()
                .map(|a| a.len() as u64)
                .unwrap_or(0),
        })
        .collect();

    Ok(issues)
}

#[tauri::command]
pub async fn get_issue_context(repo_path: String, issue_number: u64) -> Result<String, String> {
    // Fetch issue body + comments as a context block for the AI
    let output = Command::new("gh")
        .args([
            "issue", "view",
            &issue_number.to_string(),
            "--json", "title,body,comments,labels,author,state",
        ])
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get issue: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("gh issue view failed: {stderr}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let iss: serde_json::Value = serde_json::from_str(&stdout)
        .map_err(|e| format!("Failed to parse issue: {e}"))?;

    let title = iss["title"].as_str().unwrap_or("");
    let body = iss["body"].as_str().unwrap_or("");
    let author = iss["author"]["login"].as_str().unwrap_or("");
    let state = iss["state"].as_str().unwrap_or("");
    let labels: Vec<&str> = iss["labels"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|l| l["name"].as_str()).collect())
        .unwrap_or_default();

    let mut context = format!(
        "## Issue #{issue_number}: {title}\n**Author:** {author} | **State:** {state}",
    );
    if !labels.is_empty() {
        context.push_str(&format!(" | **Labels:** {}", labels.join(", ")));
    }
    context.push_str(&format!("\n\n{body}"));

    if let Some(comments) = iss["comments"].as_array() {
        for (i, comment) in comments.iter().enumerate() {
            let c_author = comment["author"]["login"].as_str().unwrap_or("unknown");
            let c_body = comment["body"].as_str().unwrap_or("");
            context.push_str(&format!("\n\n---\n**Comment #{} by {c_author}:**\n{c_body}", i + 1));
        }
    }

    Ok(context)
}

// ── Repo info ──

#[tauri::command]
pub async fn get_repo_info(repo_path: String) -> Result<serde_json::Value, String> {
    let output = Command::new("gh")
        .args(["repo", "view", "--json", "name,owner,url,defaultBranchRef"])
        .current_dir(&repo_path)
        .output()
        .await
        .map_err(|e| format!("Failed to get repo info: {e}"))?;

    if !output.status.success() {
        return Err("Not a GitHub repository".to_string());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).map_err(|e| format!("Failed to parse repo info: {e}"))
}

/// Check if a path is a git repo with a GitHub remote.
#[tauri::command]
pub async fn is_github_repo(path: String) -> Result<bool, String> {
    let output = Command::new("gh")
        .args(["repo", "view", "--json", "name"])
        .current_dir(&path)
        .output()
        .await;

    match output {
        Ok(o) => Ok(o.status.success()),
        Err(_) => Ok(false),
    }
}
