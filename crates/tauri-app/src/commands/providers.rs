use serde::Serialize;
use std::process::Command;
use tauri::State;

use crate::state::TauriState;

#[derive(Debug, Serialize)]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub installed: bool,
    pub path: Option<String>,
    pub version: Option<String>,
    pub install_instructions: String,
    pub description: String,
    pub website: String,
}

#[tauri::command]
pub fn get_provider_info(state: State<'_, TauriState>) -> Vec<ProviderInfo> {
    let claude_path = state
        .db
        .lock()
        .ok()
        .and_then(|db| {
            codeforge_persistence::queries::get_setting(db.conn(), "claude_path")
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| "claude".to_string());

    let codex_path = state
        .db
        .lock()
        .ok()
        .and_then(|db| {
            codeforge_persistence::queries::get_setting(db.conn(), "codex_path")
                .ok()
                .flatten()
        })
        .unwrap_or_else(|| "codex".to_string());

    vec![
        probe_provider(
            "claude_code",
            "Claude Code",
            &claude_path,
            "AI coding agent by Anthropic. Reads your codebase, edits files, runs commands, and manages git.",
            "npm install -g @anthropic-ai/claude-code",
            "https://claude.ai/code",
        ),
        probe_provider(
            "codex",
            "Codex",
            &codex_path,
            "OpenAI's coding agent. Generates and edits code from natural language prompts.",
            "npm install -g @openai/codex",
            "https://github.com/openai/codex",
        ),
    ]
}

fn probe_provider(
    id: &str,
    name: &str,
    binary: &str,
    description: &str,
    install_instructions: &str,
    website: &str,
) -> ProviderInfo {
    let which_result = Command::new("which").arg(binary).output();

    let (installed, path) = match which_result {
        Ok(output) if output.status.success() => {
            let p = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, Some(p))
        }
        _ => (false, None),
    };

    let version = if installed {
        Command::new(binary)
            .arg("--version")
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .or_else(|| {
                Command::new(binary)
                    .arg("-v")
                    .output()
                    .ok()
                    .filter(|o| o.status.success())
                    .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            })
    } else {
        None
    };

    ProviderInfo {
        id: id.to_string(),
        name: name.to_string(),
        installed,
        path,
        version,
        install_instructions: install_instructions.to_string(),
        description: description.to_string(),
        website: website.to_string(),
    }
}
