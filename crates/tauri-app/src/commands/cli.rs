//! Claude Code CLI health check surfaced on the welcome screen.

use serde::Serialize;

/// Named CLI states — found-and-runs, found-but-broken, and not-found are
/// distinct verdicts, never conflated. `shell_env_resolved` says whether the
/// login-shell PATH probe worked, so a miss can explain its cause.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum ClaudeCliStatus {
    Ok { path: String, version: String, auth: AuthStatus, shell_env_resolved: bool },
    Broken { path: String, detail: String, shell_env_resolved: bool },
    NotFound { shell_env_resolved: bool },
}

/// Auth verdict from `claude auth status --json`. `Unknown` is its own state
/// (older CLI without the subcommand, unparseable output, timeout) — never
/// collapsed into a definitive logged-in/out.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "state", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum AuthStatus {
    LoggedIn { method: Option<String>, email: Option<String>, subscription: Option<String> },
    LoggedOut,
    Unknown { detail: String },
}

/// Locate `claude` (login-shell PATH, then native-installer / legacy-local /
/// Homebrew locations), prove the install executes via `claude --version`,
/// then read `claude auth status --json`.
#[tauri::command]
pub async fn claude_cli_status() -> Result<ClaudeCliStatus, String> {
    let shell_env_resolved = forge_session::shell_env::is_resolved();
    let Some(path) = forge_session::shell_env::locate_claude() else {
        return Ok(ClaudeCliStatus::NotFound { shell_env_resolved });
    };
    let display = path.to_string_lossy().into_owned();

    let version_out = run_claude(&path, &["--version"]).await;
    let status = match version_out {
        Err(detail) => ClaudeCliStatus::Broken { path: display, detail, shell_env_resolved },
        Ok(version) => {
            let auth = match run_claude(&path, &["auth", "status", "--json"]).await {
                Ok(stdout) => parse_auth_status(&stdout),
                Err(detail) => AuthStatus::Unknown { detail },
            };
            ClaudeCliStatus::Ok { path: display, version, auth, shell_env_resolved }
        }
    };
    Ok(status)
}

/// Run `claude <args>` with the login-shell env and a 10s timeout, returning
/// trimmed stdout on success and a one-line cause on any failure.
async fn run_claude(path: &std::path::Path, args: &[&str]) -> Result<String, String> {
    let mut cmd = tokio::process::Command::new(path);
    cmd.args(args).stdin(std::process::Stdio::null());
    forge_session::shell_env::apply(&mut cmd);

    match tokio::time::timeout(std::time::Duration::from_secs(10), cmd.output()).await {
        Err(_) => Err(format!("`claude {}` timed out after 10s", args.join(" "))),
        Ok(Err(e)) => Err(format!("failed to spawn: {e}")),
        Ok(Ok(out)) if !out.status.success() => Err(format!(
            "`claude {}` exited with {}: {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )),
        Ok(Ok(out)) => Ok(String::from_utf8_lossy(&out.stdout).trim().to_string()),
    }
}

/// Parse the `claude auth status --json` payload. The `loggedIn` boolean is
/// the verdict; the rest is display metadata.
fn parse_auth_status(stdout: &str) -> AuthStatus {
    let value: serde_json::Value = match serde_json::from_str(stdout) {
        Ok(v) => v,
        Err(e) => return AuthStatus::Unknown { detail: format!("auth status output not JSON: {e}") },
    };
    let str_field =
        |key: &str| value.get(key).and_then(|v| v.as_str()).map(str::to_owned);
    match value.get("loggedIn").and_then(|v| v.as_bool()) {
        Some(true) => AuthStatus::LoggedIn {
            method: str_field("authMethod"),
            email: str_field("email"),
            subscription: str_field("subscriptionType"),
        },
        Some(false) => AuthStatus::LoggedOut,
        None => AuthStatus::Unknown { detail: "auth status JSON has no loggedIn field".into() },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_serializes_to_the_tagged_camel_case_shape() {
        let ok = ClaudeCliStatus::Ok {
            path: "/opt/homebrew/bin/claude".into(),
            version: "2.1.0".into(),
            auth: AuthStatus::LoggedOut,
            shell_env_resolved: true,
        };
        let json = serde_json::to_string(&ok).unwrap();
        assert!(json.contains("\"state\":\"ok\""));
        assert!(json.contains("\"shellEnvResolved\":true"));
        assert!(json.contains("\"auth\":{\"state\":\"loggedOut\"}"));

        let missing = ClaudeCliStatus::NotFound { shell_env_resolved: false };
        let json = serde_json::to_string(&missing).unwrap();
        assert!(json.contains("\"state\":\"notFound\""));
    }

    #[test]
    fn parses_the_real_logged_in_payload() {
        // Verbatim shape from `claude auth status --json` (2.1.x).
        let payload = r#"{
            "loggedIn": true,
            "authMethod": "claude.ai",
            "apiProvider": "firstParty",
            "email": "user@example.com",
            "orgId": "0000",
            "orgName": "user@example.com's Organization",
            "subscriptionType": "max"
        }"#;
        match parse_auth_status(payload) {
            AuthStatus::LoggedIn { method, email, subscription } => {
                assert_eq!(method.as_deref(), Some("claude.ai"));
                assert_eq!(email.as_deref(), Some("user@example.com"));
                assert_eq!(subscription.as_deref(), Some("max"));
            }
            other => panic!("expected LoggedIn, got {other:?}"),
        }
    }

    #[test]
    fn logged_out_and_malformed_payloads_are_distinct_states() {
        assert!(matches!(parse_auth_status(r#"{"loggedIn": false}"#), AuthStatus::LoggedOut));
        assert!(matches!(parse_auth_status("not json"), AuthStatus::Unknown { .. }));
        assert!(matches!(parse_auth_status(r#"{"foo": 1}"#), AuthStatus::Unknown { .. }));
    }
}
