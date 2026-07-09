//! Headless `claude -p` invocation + CLI JSON envelope handling.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::Duration;

use serde_json::Value;
use tokio::io::AsyncWriteExt;

use crate::{Error, Result};

/// One indexing call gets at most 6 minutes; the child is killed on timeout.
const HEADLESS_TIMEOUT: Duration = Duration::from_secs(360);
const MODEL: &str = "claude-sonnet-4-5";
/// Read-only exploration only — no Bash, no edits.
const READ_ONLY_TOOLS: &str = "Read,Glob,Grep";
const STDERR_SNIPPET_LEN: usize = 500;

/// Run `claude -p <prompt> --output-format json` in `repo_root` and return the
/// extracted result text. The binary resolves via the login-shell PATH so
/// desktop-launched apps find the right install.
pub(crate) async fn run_headless_claude(repo_root: &Path, prompt: &str) -> Result<String> {
    let claude = forge_session::shell_env::which("claude")
        .unwrap_or_else(|| PathBuf::from("claude"));

    let mut cmd = tokio::process::Command::new(&claude);
    // The prompt is piped on stdin, not passed as a positional arg: `--allowedTools`
    // is variadic (`<tools...>`) and would otherwise greedily swallow the prompt.
    cmd.arg("-p")
        .arg("--output-format")
        .arg("json")
        .arg("--model")
        .arg(MODEL)
        .arg("--allowedTools")
        .arg(READ_ONLY_TOOLS)
        .current_dir(repo_root)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    forge_session::shell_env::apply(&mut cmd);

    tracing::debug!(claude = %claude.display(), repo = %repo_root.display(), "spawning headless claude");
    let mut child = cmd
        .spawn()
        .map_err(|e| Error::Indexer(format!("failed to spawn {}: {e}", claude.display())))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(prompt.as_bytes()).await?;
        stdin.shutdown().await?; // EOF so `-p` starts generating
    }

    // On timeout the elapsed branch drops the child future; kill_on_drop reaps it.
    let output = tokio::time::timeout(HEADLESS_TIMEOUT, child.wait_with_output())
        .await
        .map_err(|_| {
            Error::Indexer(format!("claude timed out after {}s", HEADLESS_TIMEOUT.as_secs()))
        })?
        .map_err(Error::Io)?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(Error::Indexer(format!(
            "claude exited with {}: {}",
            output.status,
            snippet(stderr.trim())
        )));
    }

    extract_result_text(&String::from_utf8_lossy(&output.stdout))
}

/// Extract the result text from the `--output-format json` envelope
/// (`{"type":"result","subtype":"success","result":"...",...}`). A bare JSON
/// array is passed through for robustness; anything else is an error.
pub(crate) fn extract_result_text(stdout: &str) -> Result<String> {
    let trimmed = stdout.trim();
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|e| Error::Indexer(format!("claude output is not JSON: {e}")))?;

    match value {
        Value::Array(_) => Ok(trimmed.to_string()),
        Value::Object(obj) => {
            let is_error = obj.get("is_error").and_then(Value::as_bool).unwrap_or(false);
            let subtype = obj.get("subtype").and_then(Value::as_str).unwrap_or("");
            let result = obj.get("result").and_then(Value::as_str);
            if is_error || (!subtype.is_empty() && subtype != "success") {
                return Err(Error::Indexer(format!(
                    "claude reported an error ({subtype}): {}",
                    snippet(result.unwrap_or("no result text"))
                )));
            }
            result
                .map(str::to_string)
                .ok_or_else(|| Error::Indexer("claude envelope has no result field".into()))
        }
        _ => Err(Error::Indexer("unexpected claude output shape".into())),
    }
}

fn snippet(text: &str) -> &str {
    match text.char_indices().nth(STDERR_SNIPPET_LEN) {
        Some((idx, _)) => &text[..idx],
        None => text,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_result_from_success_envelope() {
        let stdout = r#"{"type":"result","subtype":"success","is_error":false,
            "result":"```json\n[{\"slug\":\"a\"}]\n```","session_id":"abc","total_cost_usd":0.01}"#;
        let text = extract_result_text(stdout).expect("extract");
        assert!(text.contains("```json"));
        assert_eq!(crate::parse::strip_fences(&text), r#"[{"slug":"a"}]"#);
    }

    #[test]
    fn error_envelope_is_an_error() {
        let stdout = r#"{"type":"result","subtype":"error_during_execution","is_error":true,"result":"boom"}"#;
        let err = extract_result_text(stdout).expect_err("should fail");
        assert!(matches!(err, Error::Indexer(_)));
        assert!(err.to_string().contains("boom"));
    }

    #[test]
    fn bare_array_passes_through() {
        let stdout = "\n[{\"slug\":\"a\",\"name\":\"A\"}]\n";
        assert_eq!(extract_result_text(stdout).expect("bare"), "[{\"slug\":\"a\",\"name\":\"A\"}]");
    }

    #[test]
    fn garbage_and_missing_result_are_errors() {
        assert!(extract_result_text("total garbage").is_err());
        assert!(extract_result_text("42").is_err());
        assert!(extract_result_text(r#"{"type":"result","subtype":"success"}"#).is_err());
    }
}
