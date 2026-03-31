use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::debug;

use crate::shell_env;
use crate::types::AgentEvent;

/// Path to the agent sidecar script, relative to the workspace root.
const SIDECAR_SCRIPT: &str = "crates/tauri-app/agent-sidecar/index.mjs";

pub struct ClaudeSession {
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
    /// The Claude Agent SDK session ID captured from the `session_ready` event.
    claude_session_id: Arc<Mutex<Option<String>>>,
}

impl ClaudeSession {
    pub async fn start(
        cwd: &Path,
        model: Option<&str>,
        permission_mode: Option<&str>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        Self::spawn_sidecar(cwd, model, permission_mode, None).await
    }

    /// Resume a previous session using the Agent SDK's `resume` support.
    pub async fn resume(
        cwd: &Path,
        claude_session_id: &str,
        model: Option<&str>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        Self::spawn_sidecar(cwd, model, None, Some(claude_session_id)).await
    }

    /// Return the captured Claude session ID, if available.
    pub fn claude_session_id(&self) -> Option<String> {
        self.claude_session_id.lock().ok().and_then(|g| g.clone())
    }

    /// Send a user message by writing a `query` command to the sidecar's stdin.
    pub fn send_message(&self, text: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "query",
            "prompt": text,
        });
        self.stdin_tx
            .send(msg.to_string())
            .map_err(|_| anyhow::anyhow!("Sidecar stdin channel closed"))?;
        Ok(())
    }

    /// Send an abort command to cancel the current query.
    pub fn interrupt(&self) -> Result<()> {
        let msg = serde_json::json!({ "type": "abort" });
        self.stdin_tx
            .send(msg.to_string())
            .map_err(|_| anyhow::anyhow!("Sidecar stdin channel closed"))?;
        Ok(())
    }

    /// Respond to an approval request from the sidecar.
    pub fn respond_to_approval(
        &self,
        request_id: &str,
        approve: bool,
        message: Option<&str>,
    ) -> Result<()> {
        let decision = if approve { "allow" } else { "deny" };
        let mut msg = serde_json::json!({
            "type": "approval_response",
            "requestId": request_id,
            "decision": decision,
        });
        if let Some(m) = message {
            msg.as_object_mut().unwrap().insert("message".into(), Value::String(m.to_string()));
        }
        self.stdin_tx
            .send(msg.to_string())
            .map_err(|_| anyhow::anyhow!("Sidecar stdin channel closed"))?;
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    // ── Internal ─────────────────────────────────────────────────────────────

    async fn spawn_sidecar(
        cwd: &Path,
        model: Option<&str>,
        permission_mode: Option<&str>,
        session_id: Option<&str>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        // Resolve the sidecar script path.
        // Priority: 1) Bundled in app (production) 2) Dev workspace 3) CARGO_MANIFEST_DIR
        let sidecar_path = std::env::current_exe()
            .ok()
            .and_then(|exe| {
                // Production: bundled as a resource next to the binary
                // macOS: App.app/Contents/Resources/agent-sidecar/index.mjs
                // Linux/Windows: next to the binary in agent-sidecar/
                let exe_dir = exe.parent()?;

                // macOS .app bundle: binary is in Contents/MacOS/, resources in Contents/Resources/
                let macos_resource = exe_dir.parent()
                    .map(|contents| contents.join("Resources/agent-sidecar/index.mjs"));
                if let Some(ref p) = macos_resource {
                    if p.exists() { return macos_resource; }
                }

                // Linux/Windows: resources next to the binary
                let beside_exe = exe_dir.join("agent-sidecar/index.mjs");
                if beside_exe.exists() { return Some(beside_exe); }

                // Dev mode: walk up from target/debug/ to find workspace root
                let mut dir = exe_dir;
                for _ in 0..10 {
                    let candidate = dir.join(SIDECAR_SCRIPT);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                    dir = dir.parent()?;
                }
                None
            })
            .or_else(|| {
                // Fallback: CARGO_MANIFEST_DIR (compile time)
                let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
                let workspace_root = manifest_dir.parent()?.parent()?;
                let candidate = workspace_root.join(SIDECAR_SCRIPT);
                if candidate.exists() { return Some(candidate); }
                None
            })
            .unwrap_or_else(|| std::path::PathBuf::from(SIDECAR_SCRIPT));

        debug!("Spawning agent sidecar: node {}", sidecar_path.display());

        // Resolve `node` using the user's real shell PATH so that desktop-launched
        // apps (which may have a minimal environment) find the correct binary and
        // its matching shared libraries (e.g. libnghttp2).
        let node_bin = shell_env::which("node")
            .unwrap_or_else(|| std::path::PathBuf::from("node"));

        let mut cmd = Command::new(&node_bin);
        cmd.arg(&sidecar_path)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true);
        shell_env::apply(&mut cmd);

        let mut child = cmd.spawn().context(format!(
            "Failed to spawn node agent sidecar (node={})",
            node_bin.display()
        ))?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stdin = child.stdin.take().context("Failed to capture stdin")?;
        let stderr = child.stderr.take();

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Stderr reader — collect stderr and emit a SessionError if the sidecar
        // crashes before producing any stdout (e.g. shared-library mismatch).
        if let Some(stderr) = stderr {
            let err_tx = event_tx.clone();
            tokio::spawn(async move {
                let reader = BufReader::new(stderr);
                let mut lines = reader.lines();
                let mut stderr_lines = Vec::new();
                while let Ok(Some(line)) = lines.next_line().await {
                    if !line.trim().is_empty() {
                        tracing::warn!("sidecar stderr: {line}");
                        stderr_lines.push(line);
                    }
                }
                // If the sidecar wrote to stderr and then exited, surface it.
                if !stderr_lines.is_empty() {
                    let message = format!(
                        "Agent sidecar crashed: {}",
                        stderr_lines.join("; ")
                    );
                    let _ = err_tx.send(AgentEvent::SessionError { message });
                }
            });
        }
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();
        let claude_session_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        // Stdin writer task.
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                let _ = stdin.write_all(msg.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            }
        });

        // Store init params (cwd, model, permissionMode, sessionId) so they
        // can be injected into the first query command sent via send_message().
        let init_model = model.map(|s| s.to_string());
        let init_perm = permission_mode.map(|s| s.to_string());
        let init_sid = session_id.map(|s| s.to_string());
        let cwd_str = cwd.to_string_lossy().to_string();

        let init_params = Arc::new(Mutex::new(Some(SidecarInitParams {
            cwd: cwd_str,
            model: init_model,
            permission_mode: init_perm,
            session_id: init_sid,
        })));

        // Augmenting sender: injects init params into the first query command.
        let (user_tx, mut user_rx) = mpsc::unbounded_channel::<String>();
        let init_params_clone = init_params.clone();
        let raw_tx = stdin_tx.clone();
        tokio::spawn(async move {
            while let Some(msg) = user_rx.recv().await {
                let augmented = augment_query_if_needed(&msg, &init_params_clone);
                let _ = raw_tx.send(augmented);
            }
        });

        // Stdout reader task — parse sidecar NDJSON events.
        let tx = event_tx;
        let session_id_clone = claude_session_id.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();

            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                let obj = match serde_json::from_str::<Value>(&line) {
                    Ok(o) => o,
                    Err(_) => continue,
                };

                let event_type = obj.get("type").and_then(|t| t.as_str()).unwrap_or("");

                let events: Vec<AgentEvent> = match event_type {
                    "ready" => {
                        // Sidecar is initialised. Emit SessionReady with no session ID yet.
                        vec![AgentEvent::SessionReady { claude_session_id: None, model: None }]
                    }
                    "session_ready" => {
                        let sid = obj.get("sessionId").and_then(|s| s.as_str()).map(|s| s.to_string());
                        let confirmed_model = obj.get("model").and_then(|m| m.as_str()).map(|m| m.to_string());
                        if let Some(ref s) = sid {
                            if let Ok(mut guard) = session_id_clone.lock() {
                                *guard = Some(s.clone());
                            }
                        }
                        vec![AgentEvent::SessionReady { claude_session_id: sid, model: confirmed_model }]
                    }
                    "turn_started" => {
                        vec![AgentEvent::TurnStarted { turn_id: "sidecar".to_string() }]
                    }
                    "text_delta" => {
                        let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        if text.is_empty() {
                            vec![]
                        } else {
                            vec![AgentEvent::ContentDelta { text }]
                        }
                    }
                    "thinking_delta" => {
                        let text = obj.get("text").and_then(|t| t.as_str()).unwrap_or("").to_string();
                        if text.is_empty() {
                            vec![]
                        } else {
                            vec![AgentEvent::ThinkingDelta { text }]
                        }
                    }
                    "tool_use_start" => {
                        let tool_id = obj.get("toolId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let tool_name = obj.get("toolName").and_then(|s| s.as_str()).unwrap_or("tool").to_string();
                        vec![AgentEvent::ToolUseStart { tool_id, tool_name }]
                    }
                    "tool_use_input" => {
                        let tool_id = obj.get("toolId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let input_json = obj.get("inputJson").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        vec![AgentEvent::ToolInputDelta { tool_id, input_json }]
                    }
                    "tool_result" => {
                        let tool_id = obj.get("toolId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let tool_name = obj.get("toolName").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let content = obj.get("content").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let is_error = obj.get("isError").and_then(|b| b.as_bool()).unwrap_or(false);
                        vec![AgentEvent::ToolResult { tool_id, tool_name, content, is_error }]
                    }
                    "approval_request" => {
                        let request_id = obj.get("requestId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let tool_name = obj.get("toolName").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let input = obj.get("input")
                            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                            .unwrap_or_default();
                        let description = format!("{tool_name}: {input}");
                        vec![AgentEvent::ApprovalRequired { request_id, description }]
                    }
                    "ask_user_question" => {
                        let request_id = obj.get("requestId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        let questions = obj.get("questions")
                            .map(|v| serde_json::to_string_pretty(v).unwrap_or_default())
                            .unwrap_or_default();
                        let description = format!("Question: {questions}");
                        vec![AgentEvent::ApprovalRequired { request_id, description }]
                    }
                    "turn_completed" => {
                        let sid = obj.get("sessionId").and_then(|s| s.as_str()).unwrap_or("").to_string();
                        vec![AgentEvent::TurnCompleted { turn_id: sid }]
                    }
                    "usage" => {
                        let input_tokens = obj.get("inputTokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        let output_tokens = obj.get("outputTokens").and_then(|v| v.as_u64()).unwrap_or(0);
                        let cache_read = obj.get("cacheRead").and_then(|v| v.as_u64()).unwrap_or(0);
                        let cache_write = obj.get("cacheWrite").and_then(|v| v.as_u64()).unwrap_or(0);
                        let cost_usd = obj.get("costUsd").and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let model = obj.get("model").and_then(|m| m.as_str()).unwrap_or("unknown").to_string();
                        vec![AgentEvent::UsageReport {
                            input_tokens,
                            output_tokens,
                            cache_read_tokens: cache_read,
                            cache_write_tokens: cache_write,
                            cost_usd,
                            model,
                        }]
                    }
                    "error" => {
                        let message = obj.get("message").and_then(|m| m.as_str()).unwrap_or("Unknown error").to_string();
                        vec![AgentEvent::SessionError { message }]
                    }
                    _ => {
                        debug!("Unhandled sidecar event type: {event_type}");
                        vec![]
                    }
                };

                for event in events {
                    if tx.send(event).is_err() {
                        return;
                    }
                }
            }
            debug!("Sidecar stdout reader finished");
        });

        Ok((Self { child, stdin_tx: user_tx, claude_session_id }, event_rx))
    }
}

/// Parameters captured at session creation time, injected into the first
/// query command sent to the sidecar.
struct SidecarInitParams {
    cwd: String,
    model: Option<String>,
    permission_mode: Option<String>,
    session_id: Option<String>,
}

/// If `msg` is a JSON object with `type: "query"` and init params haven't
/// been consumed yet, inject the init params into the command.
fn augment_query_if_needed(msg: &str, init: &Arc<Mutex<Option<SidecarInitParams>>>) -> String {
    let mut parsed: Value = match serde_json::from_str(msg) {
        Ok(v) => v,
        Err(_) => return msg.to_string(),
    };

    if parsed.get("type").and_then(|t| t.as_str()) != Some("query") {
        return msg.to_string();
    }

    // Take the init params (only used once).
    let params = {
        let mut guard = init.lock().unwrap();
        guard.take()
    };

    if let Some(p) = params {
        let obj = parsed.as_object_mut().unwrap();
        obj.insert("cwd".into(), Value::String(p.cwd));
        if let Some(m) = p.model {
            obj.entry("model").or_insert(Value::String(m));
        }
        if let Some(pm) = p.permission_mode {
            obj.entry("permissionMode").or_insert(Value::String(pm));
        }
        if let Some(sid) = p.session_id {
            obj.entry("sessionId").or_insert(Value::String(sid));
        }
    }

    serde_json::to_string(&parsed).unwrap_or_else(|_| msg.to_string())
}

/// Walk up from `start` to find the workspace root (a directory containing
/// a Cargo.toml with `[workspace]`).
fn find_workspace_root(start: &Path) -> Option<std::path::PathBuf> {
    let mut dir = start.to_path_buf();
    loop {
        let cargo = dir.join("Cargo.toml");
        if cargo.exists() {
            if let Ok(contents) = std::fs::read_to_string(&cargo) {
                if contents.contains("[workspace]") {
                    return Some(dir);
                }
            }
        }
        if !dir.pop() {
            return None;
        }
    }
}
