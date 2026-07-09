//! One Claude Code session = one spawned `node agent-sidecar/index.mjs` process.
//!
//! Protocol (NDJSON over stdio):
//! - in: `{type:"query", prompt, cwd, model?, permissionMode?, sessionId?}` ·
//!   `{type:"approval_response", requestId, decision:"allow"|"deny", message?}` ·
//!   `{type:"abort"}`
//! - out: see [`crate::AgentEvent`].
//!
//! Mechanics (per docs/ARCHITECTURE.md §Sessions — implement faithfully):
//! spawn via `shell_env::which("node")` + `shell_env::apply`, `kill_on_drop(true)`,
//! 4 tokio tasks (stdin writer / stdout NDJSON parser / stderr collector /
//! first-query augmenter injecting cwd/model/permissionMode/sessionId), bounded
//! channels (stdin 256, events 1024), claude session id captured from
//! `session_ready` into a `OnceLock`.

#![allow(dead_code)] // scaffold: fields are consumed once bodies are implemented

use std::path::Path;
use std::sync::{Arc, OnceLock};

use tokio::process::Child;
use tokio::sync::mpsc;

use crate::{AgentEvent, Result};

/// Path to the agent sidecar script, relative to the workspace root (dev mode).
/// Bundled resolution order: macOS `exe_dir/../Resources/agent-sidecar/index.mjs`,
/// Linux/Windows `exe_dir/agent-sidecar/index.mjs`, then dev walk-up (≤10 parents),
/// then `CARGO_MANIFEST_DIR` fallback.
pub const SIDECAR_SCRIPT: &str = "crates/tauri-app/agent-sidecar/index.mjs";

/// A live sidecar-backed Claude Code session.
pub struct ClaudeSession {
    child: Child,
    stdin_tx: mpsc::Sender<String>,
    /// The Claude Agent SDK session ID captured from the `session_ready` event.
    claude_session_id: Arc<OnceLock<String>>,
    /// Spawned reader/writer tasks; aborted on `stop()`.
    tasks: Vec<tokio::task::JoinHandle<()>>,
}

impl ClaudeSession {
    /// Spawn a fresh session (sidecar process + reader/writer tasks). Events are
    /// forwarded to `event_tx`.
    pub async fn start(
        cwd: &Path,
        model: Option<&str>,
        permission_mode: Option<&str>,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let _ = (cwd, model, permission_mode, event_tx);
        // IMPLEMENT(agent): resolve sidecar path + node, spawn, wire 4 tasks.
        todo!("ClaudeSession::start")
    }

    /// Spawn a session resuming a previous Claude session id (SDK `resume`),
    /// with fresh/continue fallback handled sidecar-side.
    pub async fn resume(
        cwd: &Path,
        claude_session_id: &str,
        model: Option<&str>,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let _ = (cwd, claude_session_id, model, event_tx);
        // IMPLEMENT(agent): as start(), passing sessionId init param.
        todo!("ClaudeSession::resume")
    }

    /// The SDK session id, once `session_ready` has been observed.
    pub fn claude_session_id(&self) -> Option<String> {
        self.claude_session_id.get().cloned()
    }

    /// Queue a user prompt: writes `{"type":"query","prompt":text}` (init params
    /// are spliced into the first query by the augmenter task).
    pub fn send_message(&self, text: &str) -> Result<()> {
        let msg = serde_json::json!({ "type": "query", "prompt": text });
        self.stdin_tx
            .try_send(msg.to_string())
            .map_err(|_| crate::Error::Sidecar("sidecar stdin channel closed".into()))
    }

    /// Abort the in-flight turn: writes `{"type":"abort"}`.
    pub fn interrupt(&self) -> Result<()> {
        self.stdin_tx
            .try_send(serde_json::json!({ "type": "abort" }).to_string())
            .map_err(|_| crate::Error::Sidecar("sidecar stdin channel closed".into()))
    }

    /// Answer a pending `approval_request`.
    pub fn respond_to_approval(&self, request_id: &str, approve: bool, message: Option<&str>) -> Result<()> {
        let mut msg = serde_json::json!({
            "type": "approval_response",
            "requestId": request_id,
            "decision": if approve { "allow" } else { "deny" },
        });
        if let Some(m) = message {
            msg["message"] = serde_json::Value::String(m.to_string());
        }
        self.stdin_tx
            .try_send(msg.to_string())
            .map_err(|_| crate::Error::Sidecar("sidecar stdin channel closed".into()))
    }

    /// Abort all tasks and kill the sidecar process.
    pub async fn stop(&mut self) -> Result<()> {
        // IMPLEMENT(agent): abort self.tasks, child.kill().await (stdin close is
        // the graceful second path — sidecar exits on rl close).
        todo!("ClaudeSession::stop")
    }

    /// OS pid of the sidecar process, if still running.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}
