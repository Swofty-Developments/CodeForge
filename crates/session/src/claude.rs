use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::types::AgentEvent;

/// Adapter for the Claude Code CLI subprocess.
///
/// Spawns `claude` with `--output-format stream-json --verbose` and reads
/// streaming JSON events from stdout line by line.
pub struct ClaudeSession {
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
}

impl ClaudeSession {
    /// Spawn a new Claude Code subprocess.
    ///
    /// Returns the session handle and a receiver for agent events.
    pub async fn start(
        cwd: &Path,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        let mut child = Command::new("claude")
            .arg("--output-format")
            .arg("stream-json")
            .arg("--verbose")
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn claude CLI")?;

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture claude stdout")?;

        let stderr = child
            .stderr
            .take()
            .context("Failed to capture claude stderr")?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();

        // Stdout reader task — parses streaming JSON events.
        let tx = event_tx.clone();
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(obj) => {
                        if let Some(event) = parse_claude_event(&obj) {
                            if tx.send(event).is_err() {
                                break;
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Non-JSON line from claude stdout: {e}");
                    }
                }
            }
            debug!("Claude stdout reader task finished");
        });

        // Stderr reader task — log warnings.
        tokio::spawn(async move {
            let reader = BufReader::new(stderr);
            let mut lines = reader.lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if !line.trim().is_empty() {
                    warn!("claude stderr: {line}");
                }
            }
        });

        // Stdin writer task — receives strings and writes them to the child's
        // stdin so callers don't need to hold a mutable reference.
        let stdin = child
            .stdin
            .take()
            .context("Failed to capture claude stdin")?;
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                if let Err(e) = stdin.write_all(msg.as_bytes()).await {
                    error!("Failed to write to claude stdin: {e}");
                    break;
                }
                if let Err(e) = stdin.flush().await {
                    error!("Failed to flush claude stdin: {e}");
                    break;
                }
            }
        });

        // Emit a ready event.
        let _ = event_tx.send(AgentEvent::SessionReady);

        Ok((Self { child, stdin_tx }, event_rx))
    }

    /// Send a message (prompt) to the Claude subprocess via stdin.
    pub fn send_message(&self, text: &str) -> Result<()> {
        self.stdin_tx
            .send(format!("{text}\n"))
            .map_err(|_| anyhow::anyhow!("Claude stdin channel closed"))?;
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
        self.child.kill().await.context("Failed to kill claude process")?;
        Ok(())
    }

    /// Return the child PID if still running.
    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

/// Parse a Claude Code streaming JSON event into an [`AgentEvent`].
fn parse_claude_event(obj: &Value) -> Option<AgentEvent> {
    let event_type = obj.get("type")?.as_str()?;

    match event_type {
        "assistant" | "content_block_delta" => {
            // Try to extract text delta
            if let Some(delta) = obj.get("delta") {
                if let Some(text) = delta.get("text").and_then(|t| t.as_str()) {
                    return Some(AgentEvent::ContentDelta {
                        text: text.to_string(),
                    });
                }
            }
            // For assistant message blocks with direct content
            if let Some(content) = obj.get("content").and_then(|c| c.as_str()) {
                return Some(AgentEvent::ContentDelta {
                    text: content.to_string(),
                });
            }
            None
        }
        "message_start" => Some(AgentEvent::TurnStarted {
            turn_id: obj
                .get("message")
                .and_then(|m| m.get("id"))
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string(),
        }),
        "message_stop" => Some(AgentEvent::TurnCompleted {
            turn_id: obj
                .get("message")
                .and_then(|m| m.get("id"))
                .and_then(|id| id.as_str())
                .unwrap_or("unknown")
                .to_string(),
        }),
        "error" => Some(AgentEvent::SessionError {
            message: obj
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string(),
        }),
        _ => {
            debug!("Unhandled claude event type: {event_type}");
            None
        }
    }
}
