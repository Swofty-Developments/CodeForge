//! Claude Code CLI health check surfaced on the welcome screen.

use serde::Serialize;

/// Named CLI states — found-and-runs, found-but-broken, and not-found are
/// distinct verdicts, never conflated. `shell_env_resolved` says whether the
/// login-shell PATH probe worked, so a miss can explain its cause.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ClaudeCliStatus {
    Ok { path: String, version: String, shell_env_resolved: bool },
    Broken { path: String, detail: String, shell_env_resolved: bool },
    NotFound { shell_env_resolved: bool },
}

/// Locate `claude` (login-shell PATH, then native-installer / legacy-local /
/// Homebrew locations) and prove the install actually executes by running
/// `claude --version`.
#[tauri::command]
pub async fn claude_cli_status() -> Result<ClaudeCliStatus, String> {
    let shell_env_resolved = forge_session::shell_env::is_resolved();
    let Some(path) = forge_session::shell_env::locate_claude() else {
        return Ok(ClaudeCliStatus::NotFound { shell_env_resolved });
    };
    let display = path.to_string_lossy().into_owned();

    let mut cmd = tokio::process::Command::new(&path);
    cmd.arg("--version").stdin(std::process::Stdio::null());
    forge_session::shell_env::apply(&mut cmd);

    let status = match tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await
    {
        Err(_) => ClaudeCliStatus::Broken {
            path: display,
            detail: "`claude --version` timed out after 10s".into(),
            shell_env_resolved,
        },
        Ok(Err(e)) => ClaudeCliStatus::Broken {
            path: display,
            detail: format!("failed to spawn: {e}"),
            shell_env_resolved,
        },
        Ok(Ok(out)) if !out.status.success() => ClaudeCliStatus::Broken {
            path: display,
            detail: format!(
                "`--version` exited with {}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            shell_env_resolved,
        },
        Ok(Ok(out)) => ClaudeCliStatus::Ok {
            path: display,
            version: String::from_utf8_lossy(&out.stdout).trim().to_string(),
            shell_env_resolved,
        },
    };
    Ok(status)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_to_the_tagged_camel_case_shape() {
        let ok = ClaudeCliStatus::Ok {
            path: "/opt/homebrew/bin/claude".into(),
            version: "2.1.0".into(),
            shell_env_resolved: true,
        };
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("\"state\":\"ok\""));
        assert!(json.contains("\"shellEnvResolved\":true"));

        let missing = ClaudeCliStatus::NotFound { shell_env_resolved: false };
        let json = serde_json::to_string(&missing).unwrap();
        assert!(json.contains("\"state\":\"notFound\""));
    }
}
