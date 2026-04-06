use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub source: String,
    pub enabled: bool,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceSource {
    pub name: String,
    pub source: String,
}

/// List installed plugins/skills for a given provider.
#[tauri::command]
pub async fn list_skills(provider: String) -> Result<Vec<SkillInfo>, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "list"])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin list: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_skills_list(&stdout))
}

/// Install a plugin by name.
#[tauri::command]
pub async fn install_skill(provider: String, name: String) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "install", &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin install: {e}"))?;

    if output.status.success() {
        Ok(format!("Installed plugin '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to install plugin: {stderr}"))
    }
}

/// Uninstall a plugin by name.
#[tauri::command]
pub async fn uninstall_skill(provider: String, name: String) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "uninstall", &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin uninstall: {e}"))?;

    if output.status.success() {
        Ok(format!("Uninstalled plugin '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to uninstall plugin: {stderr}"))
    }
}

/// Enable a plugin by name.
#[tauri::command]
pub async fn enable_skill(provider: String, name: String) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "enable", &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin enable: {e}"))?;

    if output.status.success() {
        Ok(format!("Enabled plugin '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to enable plugin: {stderr}"))
    }
}

/// Disable a plugin by name.
#[tauri::command]
pub async fn disable_skill(provider: String, name: String) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "disable", &name])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin disable: {e}"))?;

    if output.status.success() {
        Ok(format!("Disabled plugin '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to disable plugin: {stderr}"))
    }
}

/// List configured marketplace sources.
#[tauri::command]
pub async fn list_marketplaces(provider: String) -> Result<Vec<MarketplaceSource>, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "marketplace", "list"])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin marketplace list: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    Ok(parse_marketplaces(&stdout))
}

/// Add a marketplace source.
#[tauri::command]
pub async fn add_marketplace(provider: String, source: String) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        "claude" | "claude_code" => "claude",
        other => return Err(format!("Unknown provider: {other}")),
    };

    let output = Command::new(binary)
        .args(["plugin", "marketplace", "add", &source])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin marketplace add: {e}"))?;

    if output.status.success() {
        Ok(format!("Added marketplace '{source}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to add marketplace: {stderr}"))
    }
}

/// Parse `claude plugin list` output.
///
/// Format:
/// ```text
/// Installed plugins:
///
///   ❯ frontend-design@claude-plugins-official
///     Version: unknown
///     Scope: user
///     Status: ✔ enabled
///
///   ❯ rust-analyzer-lsp@claude-plugins-official
///     Version: 1.0.0
///     Scope: user
///     Status: ✔ enabled
/// ```
fn parse_skills_list(output: &str) -> Vec<SkillInfo> {
    let mut skills = Vec::new();
    let mut current_name = String::new();
    let mut current_source = String::new();
    let mut current_enabled = true;
    let mut current_version = String::new();
    let mut in_entry = false;

    for line in output.lines() {
        let trimmed = line.trim();

        // Entry header: "❯ name@source" or "❯ name"
        if trimmed.starts_with('❯') || trimmed.starts_with('>') {
            // Save previous entry
            if in_entry && !current_name.is_empty() {
                skills.push(SkillInfo {
                    name: current_name.clone(),
                    source: current_source.clone(),
                    enabled: current_enabled,
                    version: if current_version.is_empty() { None } else { Some(current_version.clone()) },
                });
            }

            let entry = trimmed.trim_start_matches('❯').trim_start_matches('>').trim();
            if let Some((name, source)) = entry.split_once('@') {
                current_name = name.trim().to_string();
                current_source = source.trim().to_string();
            } else {
                current_name = entry.to_string();
                current_source = String::new();
            }
            current_enabled = true;
            current_version = String::new();
            in_entry = true;
        } else if in_entry {
            if let Some(val) = trimmed.strip_prefix("Status:") {
                let val = val.trim();
                current_enabled = val.contains("enabled") || val.contains("✔");
            } else if let Some(val) = trimmed.strip_prefix("Version:") {
                current_version = val.trim().to_string();
            }
        }
    }

    // Don't forget the last entry
    if in_entry && !current_name.is_empty() {
        skills.push(SkillInfo {
            name: current_name,
            source: current_source,
            enabled: current_enabled,
            version: if current_version.is_empty() { None } else { Some(current_version) },
        });
    }

    skills
}

/// Parse `claude plugin marketplace list` output.
///
/// Format:
/// ```text
/// Configured marketplaces:
///
///   ❯ claude-plugins-official
///     Source: GitHub (anthropics/claude-plugins-official)
///
///   ❯ rust-skills
///     Source: GitHub (actionbook/rust-skills)
/// ```
fn parse_marketplaces(output: &str) -> Vec<MarketplaceSource> {
    let mut sources = Vec::new();
    let mut current_name = String::new();
    let mut current_source = String::new();
    let mut in_entry = false;

    for line in output.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with('❯') || trimmed.starts_with('>') {
            if in_entry && !current_name.is_empty() {
                sources.push(MarketplaceSource {
                    name: current_name.clone(),
                    source: current_source.clone(),
                });
            }
            current_name = trimmed.trim_start_matches('❯').trim_start_matches('>').trim().to_string();
            current_source = String::new();
            in_entry = true;
        } else if in_entry {
            if let Some(val) = trimmed.strip_prefix("Source:") {
                current_source = val.trim().to_string();
            }
        }
    }

    if in_entry && !current_name.is_empty() {
        sources.push(MarketplaceSource {
            name: current_name,
            source: current_source,
        });
    }

    sources
}
