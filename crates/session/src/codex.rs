use std::collections::HashMap;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, warn};

use crate::protocol::{self, JsonRpcMessage, JsonRpcResponse};
use crate::types::AgentEvent;

/// Message sent to the writer task.
enum WriterMsg {
    /// Write a raw NDJSON line to the child's stdin.
    Line(String),
}

/// A pending JSON-RPC request awaiting a response.
struct PendingRequest {
    tx: oneshot::Sender<Result<Value, String>>,
}

/// Adapter for the Codex `app-server` subprocess.
///
/// Communicates over NDJSON (newline-delimited JSON) using a JSON-RPC-like
/// protocol.
pub struct CodexSession {
    child: Child,
    /// Channel to send lines to the stdin writer task.
    writer_tx: mpsc::UnboundedSender<WriterMsg>,
    /// Counter for generating unique request IDs.
    next_id: Arc<AtomicU64>,
    /// Pending requests waiting for a response from the server.
    pending: Arc<tokio::sync::Mutex<HashMap<u64, PendingRequest>>>,
}

impl CodexSession {
    /// Spawn a new Codex `app-server` subprocess, run the initialize
    /// handshake, and start a thread.
    ///
    /// Returns the session handle and a receiver for agent events.
    pub async fn start(
        cwd: &Path,
        model: &str,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        let mut child = Command::new("codex")
            .arg("app-server")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn codex app-server")?;

        let stdout = child.stdout.take().context("Failed to capture codex stdout")?;
        let stderr = child.stderr.take().context("Failed to capture codex stderr")?;
        let stdin = child.stdin.take().context("Failed to capture codex stdin")?;

        let next_id = Arc::new(AtomicU64::new(1));
        let pending: Arc<tokio::sync::Mutex<HashMap<u64, PendingRequest>>> =
            Arc::new(tokio::sync::Mutex::new(HashMap::new()));

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (writer_tx, mut writer_rx) = mpsc::unbounded_channel::<WriterMsg>();

        // Stdin writer task.
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(WriterMsg::Line(line)) = writer_rx.recv().await {
                if let Err(e) = stdin.write_all(line.as_bytes()).await {
                    error!("Failed to write to codex stdin: {e}");
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush codex stdin: {e}");
                    break;
                }
            }
        });

        // Stderr reader task.
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    warn!("codex stderr: {line}");
                }
            }
        });

        // Stdout NDJSON reader task.
        let pending_clone = pending.clone();
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match protocol::parse_jsonrpc_line(&line) {
                    Ok(msg) => {
                        handle_jsonrpc_message(msg, &pending_clone, &tx).await;
                    }
                    Err(e) => {
                        debug!("Non-NDJSON line from codex: {e}: {line}");
                    }
                }
            }
            debug!("Codex stdout reader task finished");
        });

        let session = Self {
            child,
            writer_tx,
            next_id,
            pending,
        };

        // --- Initialize handshake ---
        let init_params = json!({
            "clientInfo": {
                "name": "codeforge",
                "title": "CodeForge",
                "version": "0.1.0"
            },
            "capabilities": {
                "experimentalApi": true
            }
        });
        session
            .send_request("initialize", init_params)
            .await
            .context("Codex initialize handshake failed")?;

        // Send `initialized` notification (no id).
        session.send_notification("initialized", json!({}))?;

        // Start a thread.
        let thread_start_params = json!({
            "model": model,
            "approvalPolicy": "on-request",
            "sandbox": "workspace-write"
        });
        session
            .send_request("thread/start", thread_start_params)
            .await
            .context("Codex thread/start failed")?;

        let _ = event_tx.send(AgentEvent::SessionReady);

        Ok((session, event_rx))
    }

    /// Send a JSON-RPC request and wait for the response.
    async fn send_request(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let request = json!({
            "method": method,
            "id": id,
            "params": params
        });
        let line = format!("{}\n", serde_json::to_string(&request)?);

        let (tx, rx) = oneshot::channel();
        {
            let mut map = self.pending.lock().await;
            map.insert(id, PendingRequest { tx });
        }

        self.writer_tx
            .send(WriterMsg::Line(line))
            .map_err(|_| anyhow::anyhow!("Codex stdin writer channel closed"))?;

        let result = rx
            .await
            .map_err(|_| anyhow::anyhow!("Codex response channel dropped for {method}"))?;

        result.map_err(|e| anyhow::anyhow!("Codex RPC error for {method}: {e}"))
    }

    /// Send a JSON-RPC notification (no response expected).
    fn send_notification(&self, method: &str, params: Value) -> Result<()> {
        let notif = json!({
            "method": method,
            "params": params
        });
        let line = format!("{}\n", serde_json::to_string(&notif)?);
        self.writer_tx
            .send(WriterMsg::Line(line))
            .map_err(|_| anyhow::anyhow!("Codex stdin writer channel closed"))?;
        Ok(())
    }

    /// Send a new turn with the given user message.
    pub async fn send_turn(&self, text: &str) -> Result<()> {
        let params = json!({
            "input": [{
                "type": "text",
                "text": text
            }]
        });
        self.send_request("turn/start", params).await?;
        Ok(())
    }

    /// Respond to an approval request from the Codex server.
    pub fn respond_to_approval(&self, request_id: &str, approve: bool) -> Result<()> {
        let id: u64 = request_id
            .parse()
            .context("Approval request_id must be a numeric JSON-RPC id")?;
        let decision = if approve { "approve" } else { "deny" };
        let response = JsonRpcResponse {
            id,
            result: Some(json!({ "decision": decision })),
            error: None,
        };
        let line = format!("{}\n", serde_json::to_string(&response)?);
        self.writer_tx
            .send(WriterMsg::Line(line))
            .map_err(|_| anyhow::anyhow!("Codex stdin writer channel closed"))?;
        Ok(())
    }

    /// Send SIGINT to interrupt the current operation.
    pub fn interrupt(&self) -> Result<()> {
        if let Some(pid) = self.child.id() {
            #[cfg(unix)]
            {
                unsafe {
                    libc::kill(pid as i32, libc::SIGINT);
                }
            }
            #[cfg(not(unix))]
            {
                let _ = pid;
                anyhow::bail!("Interrupt is only supported on Unix");
            }
        }
        Ok(())
    }

    /// Kill the child process.
    pub async fn stop(&mut self) -> Result<()> {
        self.child
            .kill()
            .await
            .context("Failed to kill codex process")?;
        Ok(())
    }

    /// Return the child PID if still running.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

/// Handle a parsed JSON-RPC message from the Codex server.
async fn handle_jsonrpc_message(
    msg: JsonRpcMessage,
    pending: &tokio::sync::Mutex<HashMap<u64, PendingRequest>>,
    event_tx: &mpsc::UnboundedSender<AgentEvent>,
) {
    match msg {
        JsonRpcMessage::Response(resp) => {
            let mut map = pending.lock().await;
            if let Some(req) = map.remove(&resp.id) {
                if let Some(err) = resp.error {
                    let _ = req.tx.send(Err(err.message));
                } else {
                    let _ = req.tx.send(Ok(resp.result.unwrap_or(Value::Null)));
                }
            } else {
                debug!("Received response for unknown request id={}", resp.id);
            }
        }
        JsonRpcMessage::Request(req) => {
            // Server-initiated requests (e.g., approval requests).
            let method = &req.method;
            if method.contains("requestApproval") {
                if let Some(id) = req.id {
                    let description = req
                        .params
                        .get("command")
                        .and_then(|c| c.as_str())
                        .or_else(|| req.params.get("description").and_then(|d| d.as_str()))
                        .unwrap_or("Command execution")
                        .to_string();
                    let _ = event_tx.send(AgentEvent::ApprovalRequired {
                        request_id: id.to_string(),
                        description,
                    });
                }
            } else {
                debug!("Unhandled server request: {method}");
            }
        }
        JsonRpcMessage::Notification(notif) => {
            let method = notif.method.as_str();
            match method {
                "content.delta" => {
                    if let Some(text) = notif
                        .params
                        .get("delta")
                        .and_then(|d| d.as_str())
                        .or_else(|| notif.params.get("text").and_then(|t| t.as_str()))
                    {
                        let _ = event_tx.send(AgentEvent::ContentDelta {
                            text: text.to_string(),
                        });
                    }
                }
                "turn/started" => {
                    let turn_id = notif
                        .params
                        .get("turnId")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = event_tx.send(AgentEvent::TurnStarted { turn_id });
                }
                "turn/completed" => {
                    let turn_id = notif
                        .params
                        .get("turnId")
                        .and_then(|t| t.as_str())
                        .unwrap_or("unknown")
                        .to_string();
                    let _ = event_tx.send(AgentEvent::TurnCompleted { turn_id });
                }
                "turn/aborted" => {
                    let reason = notif
                        .params
                        .get("reason")
                        .and_then(|r| r.as_str())
                        .unwrap_or("Unknown reason")
                        .to_string();
                    let _ = event_tx.send(AgentEvent::TurnAborted { reason });
                }
                _ => {
                    debug!("Unhandled codex notification: {method}");
                }
            }
        }
    }
}
