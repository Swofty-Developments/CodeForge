use serde::Serialize;
use tokio::process::Command;

#[derive(Debug, Clone, Serialize)]
pub struct McpServer {
    pub name: String,
    pub url_or_command: String,
    pub transport: String,
    pub scope: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
    pub source: String,
}

/// List MCP servers configured for a given provider.
#[tauri::command]
pub async fn mcp_list_servers(provider: String) -> Result<Vec<McpServer>, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        _ => "claude",
    };

    // Try CLI first
    let output = Command::new(binary)
        .args(["mcp", "list"])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} mcp list: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut servers = parse_mcp_list(&stdout);

    // If CLI returned nothing, try reading config files directly
    if servers.is_empty() && provider != "codex" {
        let home = std::env::var("HOME").unwrap_or_default();
        // Check ~/.claude/.mcp.json and project .mcp.json
        for (path, scope) in [
            (format!("{home}/.claude/.mcp.json"), "user"),
            (".mcp.json".to_string(), "project"),
        ] {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(obj) = json.get("mcpServers").and_then(|s| s.as_object())
                        .or_else(|| json.as_object()) {
                        for (name, config) in obj {
                            let url_or_cmd = config.get("url")
                                .or_else(|| config.get("command"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let transport = if url_or_cmd.starts_with("http") { "http" }
                                else if url_or_cmd.contains("sse") { "sse" }
                                else { "stdio" };
                            servers.push(McpServer {
                                name: name.clone(),
                                url_or_command: url_or_cmd,
                                transport: transport.to_string(),
                                scope: scope.to_string(),
                                status: "unknown".to_string(),
                            });
                        }
                    }
                }
            }
        }
    }

    Ok(servers)
}

/// Add an MCP server.
#[tauri::command]
pub async fn mcp_add_server(
    provider: String,
    name: String,
    url_or_command: String,
    transport: String,
    scope: String,
    env_vars: Vec<String>,
    extra_args: Vec<String>,
) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        _ => "claude",
    };

    let mut args = vec!["mcp".to_string(), "add".to_string()];

    if !transport.is_empty() {
        args.push("-t".to_string());
        args.push(transport);
    }

    if !scope.is_empty() {
        args.push("-s".to_string());
        args.push(scope);
    }

    for env in &env_vars {
        args.push("-e".to_string());
        args.push(env.clone());
    }

    args.push(name.clone());
    args.push(url_or_command);

    for arg in &extra_args {
        args.push(arg.clone());
    }

    let output = Command::new(binary)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} mcp add: {e}"))?;

    if output.status.success() {
        Ok(format!("Added MCP server '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to add MCP server: {stderr}"))
    }
}

/// Remove an MCP server.
#[tauri::command]
pub async fn mcp_remove_server(
    provider: String,
    name: String,
    scope: String,
) -> Result<String, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        _ => "claude",
    };

    let mut args = vec!["mcp", "remove"];
    let scope_owned;
    if !scope.is_empty() {
        args.push("-s");
        scope_owned = scope;
        args.push(&scope_owned);
    }
    args.push(&name);

    let output = Command::new(binary)
        .args(&args)
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} mcp remove: {e}"))?;

    if output.status.success() {
        Ok(format!("Removed MCP server '{name}'"))
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        Err(format!("Failed to remove: {stderr}"))
    }
}

/// List available slash commands / skills.
#[tauri::command]
pub async fn list_slash_commands(provider: String) -> Result<Vec<SlashCommand>, String> {
    let binary = match provider.as_str() {
        "codex" => "codex",
        _ => "claude",
    };

    // Try to get plugin list with skills
    let output = Command::new(binary)
        .args(["plugin", "list"])
        .output()
        .await
        .map_err(|e| format!("Failed to run {binary} plugin list: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let mut commands = parse_plugin_skills(&stdout);

    // Always include built-in slash commands for Claude Code
    if provider != "codex" {
        let builtins = vec![
            SlashCommand { name: "/commit".into(), description: "Create a git commit with a message".into(), source: "built-in".into() },
            SlashCommand { name: "/review-pr".into(), description: "Review a pull request".into(), source: "built-in".into() },
            SlashCommand { name: "/help".into(), description: "Show help and available commands".into(), source: "built-in".into() },
            SlashCommand { name: "/clear".into(), description: "Clear conversation history".into(), source: "built-in".into() },
            SlashCommand { name: "/compact".into(), description: "Compact conversation to save context".into(), source: "built-in".into() },
            SlashCommand { name: "/cost".into(), description: "Show token usage and cost".into(), source: "built-in".into() },
            SlashCommand { name: "/doctor".into(), description: "Check health of Claude Code".into(), source: "built-in".into() },
            SlashCommand { name: "/login".into(), description: "Switch authentication".into(), source: "built-in".into() },
            SlashCommand { name: "/logout".into(), description: "Log out".into(), source: "built-in".into() },
            SlashCommand { name: "/model".into(), description: "Switch model".into(), source: "built-in".into() },
            SlashCommand { name: "/permissions".into(), description: "View or update permissions".into(), source: "built-in".into() },
            SlashCommand { name: "/status".into(), description: "Show session status".into(), source: "built-in".into() },
            SlashCommand { name: "/terminal-setup".into(), description: "Install shell integration".into(), source: "built-in".into() },
        ];
        // Prepend builtins, then plugin-based
        let mut all = builtins;
        all.append(&mut commands);
        commands = all;
    }

    Ok(commands)
}

/// Parse `claude mcp list` output into structured servers.
fn parse_mcp_list(output: &str) -> Vec<McpServer> {
    let mut servers = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with("MCP") || line.starts_with("---") || line.starts_with("No ") {
            continue;
        }

        // Lines typically look like:
        // "name: url - status" or "name (scope): command args"
        // Try to parse flexible formats
        if let Some((name_part, rest)) = line.split_once(':') {
            let name = name_part.trim().to_string();
            let rest = rest.trim();

            let (url_or_cmd, status) = if let Some((u, s)) = rest.rsplit_once(" - ") {
                (u.trim().to_string(), s.trim().to_string())
            } else {
                (rest.to_string(), String::new())
            };

            let transport = if url_or_cmd.starts_with("http") {
                "http"
            } else if url_or_cmd.starts_with("sse:") || url_or_cmd.contains("sse") {
                "sse"
            } else {
                "stdio"
            };

            servers.push(McpServer {
                name,
                url_or_command: url_or_cmd,
                transport: transport.to_string(),
                scope: "user".to_string(),
                status: if status.contains("Connected") || status.contains("✓") {
                    "connected".to_string()
                } else if status.contains("auth") || status.contains("!") {
                    "needs_auth".to_string()
                } else if status.is_empty() {
                    "unknown".to_string()
                } else {
                    status
                },
            });
        }
    }

    servers
}

/// Parse `claude plugin list` output to extract skill-based slash commands.
fn parse_plugin_skills(output: &str) -> Vec<SlashCommand> {
    let mut commands = Vec::new();

    for line in output.lines() {
        let line = line.trim();
        // Look for lines like "- skill-name@marketplace (enabled)"
        if line.starts_with('-') || line.starts_with('•') {
            let clean = line.trim_start_matches(['-', '•', ' '].as_ref()).trim();
            if let Some((name_part, _)) = clean.split_once('@') {
                let name = name_part.trim();
                commands.push(SlashCommand {
                    name: format!("/{name}"),
                    description: format!("Plugin skill: {name}"),
                    source: "plugin".to_string(),
                });
            } else if let Some((name_part, _)) = clean.split_once(' ') {
                let name = name_part.trim();
                if !name.is_empty() {
                    commands.push(SlashCommand {
                        name: format!("/{name}"),
                        description: "Plugin skill".to_string(),
                        source: "plugin".to_string(),
                    });
                }
            }
        }
    }

    commands
}
