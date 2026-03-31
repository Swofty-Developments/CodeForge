use serde::Serialize;
use tauri::State;
use tokio::process::Command;

use crate::state::TauriState;

#[derive(Debug, Clone, Serialize)]
pub struct BinaryStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SetupStatus {
    pub complete: bool,
    pub binaries: Vec<BinaryStatus>,
    pub has_any_binary: bool,
    pub gh_installed: bool,
    pub gh_authenticated: bool,
    pub gh_username: Option<String>,
}

#[tauri::command]
pub async fn check_setup_status(
    state: State<'_, TauriState>,
) -> Result<SetupStatus, String> {
    // Check if setup was already completed
    let complete = {
        let db = state.db.lock().map_err(|e| e.to_string())?;
        codeforge_persistence::queries::get_setting(db.conn(), "onboarding_complete")
            .unwrap_or(None)
            .map(|v| v == "true")
            .unwrap_or(false)
    };

    // Check binaries
    let claude = check_binary("claude").await;
    let codex = check_binary("codex").await;
    let has_any = claude.installed || codex.installed;

    // Check gh CLI
    let gh = check_binary("gh").await;
    let (gh_authenticated, gh_username) = if gh.installed {
        check_gh_auth().await
    } else {
        (false, None)
    };

    Ok(SetupStatus {
        complete,
        binaries: vec![claude, codex],
        has_any_binary: has_any,
        gh_installed: gh.installed,
        gh_authenticated,
        gh_username,
    })
}

#[tauri::command]
pub async fn complete_setup(
    state: State<'_, TauriState>,
) -> Result<(), String> {
    let db = state.db.lock().map_err(|e| e.to_string())?;
    codeforge_persistence::queries::set_setting(db.conn(), "onboarding_complete", "true")
        .map_err(|e| e.to_string())?;
    Ok(())
}

async fn check_binary(name: &str) -> BinaryStatus {
    let which = Command::new("which")
        .arg(name)
        .output()
        .await;

    match which {
        Ok(output) if output.status.success() => {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();

            // Try to get version
            let version = Command::new(name)
                .arg("--version")
                .output()
                .await
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
                .filter(|s| !s.is_empty());

            BinaryStatus {
                name: name.to_string(),
                installed: true,
                version,
                path: Some(path),
            }
        }
        _ => BinaryStatus {
            name: name.to_string(),
            installed: false,
            version: None,
            path: None,
        },
    }
}

async fn check_gh_auth() -> (bool, Option<String>) {
    let output = Command::new("gh")
        .args(["auth", "status", "--hostname", "github.com"])
        .output()
        .await;

    match output {
        Ok(o) => {
            let combined = format!(
                "{}\n{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );

            let logged_in = combined.contains("Logged in");
            let username = combined
                .lines()
                .find(|l| l.contains("account"))
                .and_then(|l| {
                    l.find("account ").map(|idx| {
                        l[idx + 8..]
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .trim_matches(|c: char| !c.is_alphanumeric() && c != '-' && c != '_')
                            .to_string()
                    })
                })
                .filter(|s| !s.is_empty());

            (logged_in, username)
        }
        Err(_) => (false, None),
    }
}
