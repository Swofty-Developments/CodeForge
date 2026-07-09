//! One Claude Code session = one spawned `node agent-sidecar/index.mjs` process.
//!
//! Protocol (NDJSON over stdio):
//! - in: `{type:"query", prompt, cwd, model?, permissionMode?, sessionId?}` ·
//!   `{type:"approval_response", requestId, decision:"allow"|"deny", message?}` ·
//!   `{type:"abort"}`
//! - out: see [`crate::AgentEvent`].
//!
//! Mechanics (per docs/ARCHITECTURE.md §Sessions): spawn via
//! `shell_env::which("node")` + `shell_env::apply`, `kill_on_drop(true)`,
//! 4 tokio tasks (stdin writer / stdout NDJSON parser / stderr collector /
//! first-query augmenter injecting cwd/model/permissionMode/sessionId), bounded
//! channels (stdin 256, events 1024 caller-side), claude session id captured
//! from `session_ready` into a `OnceLock`.

use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::{Arc, Mutex, OnceLock};

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, Command};
use tokio::sync::mpsc;
use tracing::{debug, warn};

use crate::protocol::{self, SidecarInitParams};
use crate::{locate, shell_env, AgentEvent, Result};

/// Path to the agent sidecar script, relative to the workspace root (dev mode).
/// Bundled resolution order: macOS `exe_dir/../Resources/agent-sidecar/index.mjs`,
/// Linux/Windows `exe_dir/agent-sidecar/index.mjs`, then dev walk-up (≤10 parents),
/// then `CARGO_MANIFEST_DIR` fallback.
pub const SIDECAR_SCRIPT: &str = "crates/tauri-app/agent-sidecar/index.mjs";

const STDIN_CHANNEL_CAP: usize = 256;
/// Retained stderr lines for the crash message (oldest dropped past this).
const STDERR_BUFFER_LINES: usize = 50;

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
        let init = SidecarInitParams {
            cwd: cwd.to_string_lossy().into_owned(),
            model: model.map(str::to_string),
            permission_mode: permission_mode.map(str::to_string),
            session_id: None,
        };
        Self::spawn_sidecar(cwd, init, event_tx).await
    }

    /// Spawn a session resuming a previous Claude session id (SDK `resume`),
    /// with fresh/continue fallback handled sidecar-side.
    pub async fn resume(
        cwd: &Path,
        claude_session_id: &str,
        model: Option<&str>,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        // permission_mode intentionally not carried across resume (CodeForge parity).
        let init = SidecarInitParams {
            cwd: cwd.to_string_lossy().into_owned(),
            model: model.map(str::to_string),
            permission_mode: None,
            session_id: Some(claude_session_id.to_string()),
        };
        Self::spawn_sidecar(cwd, init, event_tx).await
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
        // Tasks first, so the stderr collector never reports a stop as a crash.
        for task in self.tasks.drain(..) {
            task.abort();
        }
        if let Err(e) = self.child.kill().await {
            debug!("sidecar kill (likely already exited): {e}");
        }
        Ok(())
    }

    /// OS pid of the sidecar process, if still running.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }

    async fn spawn_sidecar(
        cwd: &Path,
        init: SidecarInitParams,
        event_tx: mpsc::Sender<AgentEvent>,
    ) -> Result<Self> {
        let sidecar = locate::resolve_sidecar().ok_or_else(|| {
            crate::Error::Sidecar(
                "agent sidecar script not found (app resources, beside executable, or dev workspace)".into(),
            )
        })?;
        // Login-shell PATH: desktop-launched apps otherwise miss nvm/fnm node.
        let node = shell_env::which("node").unwrap_or_else(|| PathBuf::from("node"));
        debug!(node = %node.display(), sidecar = %sidecar.display(), cwd = %cwd.display(), "spawning agent sidecar");

        let mut cmd = Command::new(&node);
        cmd.arg(&sidecar)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true);
        shell_env::apply(&mut cmd);
        let mut child = cmd.spawn()?;

        let stdin = take_pipe(child.stdin.take(), "stdin")?;
        let stdout = take_pipe(child.stdout.take(), "stdout")?;
        let stderr = take_pipe(child.stderr.take(), "stderr")?;

        let claude_session_id: Arc<OnceLock<String>> = Arc::new(OnceLock::new());
        let (stdin_tx, mut raw_rx) = mpsc::channel::<String>(STDIN_CHANNEL_CAP);
        let (writer_tx, mut writer_rx) = mpsc::channel::<String>(STDIN_CHANNEL_CAP);

        // Task 1 — augmenter: splices init params into the first query only.
        let init_slot = Arc::new(Mutex::new(Some(init)));
        let augment_task = tokio::spawn(async move {
            while let Some(msg) = raw_rx.recv().await {
                let out = protocol::augment_query_if_needed(&msg, &init_slot);
                if writer_tx.send(out).await.is_err() {
                    return;
                }
            }
        });

        // Task 2 — stdin writer: one NDJSON line per command, flushed.
        let writer_task = tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(line) = writer_rx.recv().await {
                if let Err(e) = write_line(&mut stdin, &line).await {
                    // Dropping stdin lets the sidecar exit; stderr EOF surfaces it.
                    warn!("sidecar stdin write failed: {e}");
                    return;
                }
            }
        });

        // Task 3 — stdout NDJSON parser → AgentEvents.
        let sid_slot = claude_session_id.clone();
        let stdout_tx = event_tx.clone();
        let stdout_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stdout).lines();
            loop {
                let line = match lines.next_line().await {
                    Ok(Some(l)) => l,
                    Ok(None) => {
                        debug!("sidecar stdout closed");
                        return;
                    }
                    Err(e) => {
                        debug!("sidecar stdout read error: {e}");
                        return;
                    }
                };
                for event in protocol::parse_sidecar_line(&line) {
                    if let AgentEvent::SessionReady { claude_session_id: Some(sid), .. } = &event {
                        let _ = sid_slot.set(sid.clone());
                    }
                    if stdout_tx.send(event).await.is_err() {
                        return; // receiver dropped — stop reading
                    }
                }
            }
        });

        // Task 4 — stderr collector. Stderr EOF means the process exited, which
        // is always abnormal while the session is live (stop() aborts us first),
        // so a terminal SessionError is guaranteed on every sidecar exit path.
        let stderr_task = tokio::spawn(async move {
            let mut lines = BufReader::new(stderr).lines();
            let mut buffer: Vec<String> = Vec::new();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                warn!("sidecar stderr: {line}");
                if buffer.len() == STDERR_BUFFER_LINES {
                    buffer.remove(0);
                }
                buffer.push(line);
            }
            let message = if buffer.is_empty() {
                "agent sidecar exited unexpectedly".to_string()
            } else {
                format!("Agent sidecar crashed: {}", buffer.join("; "))
            };
            let _ = event_tx.send(AgentEvent::SessionError { message }).await;
        });

        Ok(Self {
            child,
            stdin_tx,
            claude_session_id,
            tasks: vec![augment_task, writer_task, stdout_task, stderr_task],
        })
    }
}

fn take_pipe<T>(pipe: Option<T>, name: &str) -> Result<T> {
    pipe.ok_or_else(|| crate::Error::Sidecar(format!("sidecar {name} not piped")))
}

async fn write_line(stdin: &mut ChildStdin, line: &str) -> std::io::Result<()> {
    stdin.write_all(line.as_bytes()).await?;
    stdin.write_all(b"\n").await?;
    stdin.flush().await
}
