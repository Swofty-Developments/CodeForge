use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct SkillInfo {
    pub name: String,
    pub source: String,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MarketplaceSource {
    pub url: String,
}

/// List installed plugins/skills for a given provider.
#[tauri::command]
pub async fn list_skills(provider: String) -> Result<Vec<SkillInfo>, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        _ => "claude",
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
        _ => "claude",
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
        _ => "claude",
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
        _ => "claude",
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
        _ => "claude",
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
        _ => "claude",
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
        _ => "claude",
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

/// Parse `claude plugin list` output into structured skill info.
fn parse_skills_list(output: &str) -> Vec<SkillInfo> {
    let mut skills = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("Plugin")
            || line.starts_with("---")
            || line.starts_with("No ")
            || line.starts_with("Installed")
        {
            continue;
        }

        // Lines typically look like:
        // "- skill-name@marketplace (enabled)"
        // "- skill-name (disabled)"
        // "  skill-name   marketplace-source   enabled"
        let clean = line.trim_start_matches(['-', '*', ' '].as_ref()).trim();
        if clean.is_empty() {
            continue;
        }

        let enabled = !clean.contains("disabled");

        if let Some((name_part, rest)) = clean.split_once('@') {
            let name = name_part.trim().to_string();
            let source = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(')')
                .to_string();
            skills.push(SkillInfo {
                name,
                source,
                enabled,
            });
        } else if let Some((name_part, _)) = clean.split_once(char::is_whitespace) {
            let name = name_part.trim().to_string();
            if !name.is_empty() && !name.starts_with('(') {
                skills.push(SkillInfo {
                    name,
                    source: String::new(),
                    enabled,
                });
            }
        } else {
            // Single word — just a name
            let name = clean
                .trim_end_matches("(enabled)")
                .trim_end_matches("(disabled)")
                .trim()
                .to_string();
            if !name.is_empty() {
                skills.push(SkillInfo {
                    name,
                    source: String::new(),
                    enabled,
                });
            }
        }
    }

    skills
}

/// Parse `claude plugin marketplace list` output.
fn parse_marketplaces(output: &str) -> Vec<MarketplaceSource> {
    let mut sources = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty()
            || line.starts_with("Marketplace")
            || line.starts_with("---")
            || line.starts_with("No ")
            || line.starts_with("Configured")
        {
            continue;
        }

        let clean = line.trim_start_matches(['-', '*', ' '].as_ref()).trim();
        if !clean.is_empty() {
            sources.push(MarketplaceSource {
                url: clean.to_string(),
            });
        }
    }

    sources
}
