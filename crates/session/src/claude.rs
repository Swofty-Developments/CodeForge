use std::path::Path;
use std::sync::{Arc, Mutex};

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::debug;

use crate::types::AgentEvent;

pub struct ClaudeSession {
    child: Child,
    session_id: Arc<Mutex<Option<String>>>,
    cwd: std::path::PathBuf,
    event_tx: mpsc::UnboundedSender<AgentEvent>,
}

impl ClaudeSession {
    pub async fn start(
        cwd: &Path,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let _ = event_tx.send(AgentEvent::SessionReady);

        Ok((
            Self {
                child: Command::new("true").spawn()?,
                session_id: Arc::new(Mutex::new(None)),
                cwd: cwd.to_path_buf(),
                event_tx,
            },
            event_rx,
        ))
    }

    pub fn send_message(&mut self, text: &str) -> Result<()> {
        let mut args = vec![
            "-p".to_string(),
            "--output-format".to_string(),
            "stream-json".to_string(),
            "--verbose".to_string(),
        ];

        if let Some(ref sid) = *self.session_id.lock().unwrap() {
            args.push("--resume".to_string());
            args.push(sid.clone());
        }

        let mut child = std::process::Command::new("claude")
            .args(&args)
            .current_dir(&self.cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .context("Failed to spawn claude CLI")?;

        // Write prompt to stdin then close it
        if let Some(mut stdin) = child.stdin.take() {
            use std::io::Write;
            let _ = stdin.write_all(text.as_bytes());
            let _ = stdin.write_all(b"\n");
            // stdin drops here, closing the pipe
        }

        let stdout = child
            .stdout
            .take()
            .context("Failed to capture claude stdout")?;

        let tx = self.event_tx.clone();
        let sid_ref = self.session_id.clone();

        // Read stdout in a background thread (sync child process)
        std::thread::spawn(move || {
            use std::io::BufRead;
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                let line = match line {
                    Ok(l) => l,
                    Err(_) => break,
                };
                if line.trim().is_empty() {
                    continue;
                }
                match serde_json::from_str::<Value>(&line) {
                    Ok(obj) => {
                        // Capture session_id from system init or result
                        if let Some(sid) = obj.get("session_id").and_then(|s| s.as_str()) {
                            let mut lock = sid_ref.lock().unwrap();
                            if lock.is_none() {
                                *lock = Some(sid.to_string());
                            }
                        }
                        for event in parse_claude_event(&obj) {
                            if tx.send(event).is_err() {
                                return;
                            }
                        }
                    }
                    Err(e) => {
                        debug!("Non-JSON line from claude: {e}");
                    }
                }
            }
            let _ = child.wait();
        });

        Ok(())
    }

    pub fn interrupt(&self) -> Result<()> {
        if let Some(pid) = self.child.id() {
            #[cfg(unix)]
            unsafe {
                libc::kill(pid as i32, libc::SIGINT);
            }
        }
        Ok(())
    }

    pub async fn stop(&mut self) -> Result<()> {
        let _ = self.child.kill().await;
        Ok(())
    }

    pub fn pid(&self) -> Option<u32> {
        self.child.id()
    }
}

fn parse_claude_event(obj: &Value) -> Vec<AgentEvent> {
    let mut events = Vec::new();
    let Some(event_type) = obj.get("type").and_then(|t| t.as_str()) else {
        return events;
    };

    match event_type {
        "system" => {
            // Extract session_id for resume support
            // (handled by caller via the session_id field in result)
        }
        "assistant" => {
            // Extract text from message.content array
            if let Some(content) = obj
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            {
                events.push(AgentEvent::TurnStarted {
                    turn_id: obj
                        .get("message")
                        .and_then(|m| m.get("id"))
                        .and_then(|id| id.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                });
                for block in content {
                    if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                        events.push(AgentEvent::ContentDelta {
                            text: text.to_string(),
                        });
                    }
                }
            }
        }
        "content_block_delta" => {
            if let Some(text) = obj
                .get("delta")
                .and_then(|d| d.get("text"))
                .and_then(|t| t.as_str())
            {
                events.push(AgentEvent::ContentDelta {
                    text: text.to_string(),
                });
            }
        }
        "result" => {
            let is_error = obj
                .get("is_error")
                .and_then(|e| e.as_bool())
                .unwrap_or(false);
            if is_error {
                let msg = obj
                    .get("result")
                    .and_then(|r| r.as_str())
                    .unwrap_or("Unknown error")
                    .to_string();
                events.push(AgentEvent::SessionError { message: msg });
            } else {
                // Extract session_id from result for future --resume
                events.push(AgentEvent::TurnCompleted {
                    turn_id: obj
                        .get("session_id")
                        .and_then(|s| s.as_str())
                        .unwrap_or("unknown")
                        .to_string(),
                });
            }
        }
        "error" => {
            let msg = obj
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown error")
                .to_string();
            events.push(AgentEvent::SessionError { message: msg });
        }
        _ => {
            debug!("Unhandled claude event type: {event_type}");
        }
    }

    events
}
