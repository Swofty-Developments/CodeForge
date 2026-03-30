use std::path::Path;

use anyhow::{Context, Result};
use serde_json::Value;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;
use tracing::debug;

use crate::types::AgentEvent;

pub struct ClaudeSession {
    child: Child,
    stdin_tx: mpsc::UnboundedSender<String>,
}

impl ClaudeSession {
    pub async fn start(
        cwd: &Path,
        model: Option<&str>,
    ) -> Result<(Self, mpsc::UnboundedReceiver<AgentEvent>)> {
        let mut args = vec![
            "-p",
            "--output-format", "stream-json",
            "--verbose",
            "--input-format", "stream-json",
            "--include-partial-messages",
        ];
        if let Some(m) = model {
            args.push("--model");
            args.push(m);
        }

        let mut child = Command::new("claude")
            .args(&args)
            .current_dir(cwd)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("Failed to spawn claude CLI")?;

        let stdout = child.stdout.take().context("Failed to capture stdout")?;
        let stdin = child.stdin.take().context("Failed to capture stdin")?;

        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (stdin_tx, mut stdin_rx) = mpsc::unbounded_channel::<String>();

        // Stdin writer
        tokio::spawn(async move {
            let mut stdin = stdin;
            while let Some(msg) = stdin_rx.recv().await {
                let _ = stdin.write_all(msg.as_bytes()).await;
                let _ = stdin.write_all(b"\n").await;
                let _ = stdin.flush().await;
            }
        });

        // Stdout reader
        let tx = event_tx;
        tokio::spawn(async move {
            let reader = BufReader::new(stdout);
            let mut lines = reader.lines();
            let mut last_len: usize = 0;
            let mut turn_started = false;
            // Track whether we've received stream_event deltas for the current turn.
            // If so, skip `assistant` text processing to avoid duplicate content.
            let mut got_stream_deltas = false;

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
                    "system" => {
                        if obj.get("subtype").and_then(|s| s.as_str()) == Some("init") {
                            vec![AgentEvent::SessionReady]
                        } else {
                            vec![]
                        }
                    }
                    "stream_event" => {
                        let evts = handle_stream_event(&obj, &mut turn_started);
                        // Mark that we got real streaming deltas so we skip
                        // the `assistant` snapshot which would duplicate them.
                        if evts.iter().any(|e| matches!(e, AgentEvent::ContentDelta { .. })) {
                            got_stream_deltas = true;
                        }
                        evts
                    }
                    "assistant" => {
                        if got_stream_deltas {
                            // Already streaming via stream_event — skip the
                            // assistant snapshot to avoid dumping the full text
                            // again. Just update last_len so the counter stays
                            // in sync in case a future turn falls back to
                            // assistant-only events.
                            let full_text: String = obj
                                .get("message")
                                .and_then(|m| m.get("content"))
                                .and_then(|c| c.as_array())
                                .map(|blocks| {
                                    blocks
                                        .iter()
                                        .filter_map(|b| {
                                            if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                                                b.get("text").and_then(|t| t.as_str())
                                            } else {
                                                None
                                            }
                                        })
                                        .collect::<Vec<_>>()
                                        .join("")
                                })
                                .unwrap_or_default();
                            last_len = full_text.len();
                            vec![]
                        } else {
                            handle_assistant(&obj, &mut last_len, &mut turn_started)
                        }
                    }
                    "result" => {
                        last_len = 0;
                        turn_started = false;
                        got_stream_deltas = false;
                        handle_result(&obj)
                    }
                    _ => vec![],
                };

                for event in events {
                    if tx.send(event).is_err() {
                        return;
                    }
                }
            }
            debug!("Claude stdout reader finished");
        });

        Ok((Self { child, stdin_tx }, event_rx))
    }

    pub fn send_message(&self, text: &str) -> Result<()> {
        let msg = serde_json::json!({
            "type": "user",
            "session_id": "",
            "message": {"role": "user", "content": text},
            "parent_tool_use_id": null
        });
        self.stdin_tx
            .send(msg.to_string())
            .map_err(|_| anyhow::anyhow!("Claude stdin channel closed"))?;
        Ok(())
    }

    pub fn interrupt(&self) -> Result<()> {
        if let Some(pid) = self.child.id() {
            #[cfg(unix)]
            unsafe { libc::kill(pid as i32, libc::SIGINT); }
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

fn handle_assistant(obj: &Value, last_len: &mut usize, turn_started: &mut bool) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    // Extract full text from message.content array
    let full_text: String = obj
        .get("message")
        .and_then(|m| m.get("content"))
        .and_then(|c| c.as_array())
        .map(|blocks| {
            blocks
                .iter()
                .filter_map(|b| {
                    if b.get("type").and_then(|t| t.as_str()) == Some("text") {
                        b.get("text").and_then(|t| t.as_str())
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();

    if !full_text.is_empty() && !*turn_started {
        *turn_started = true;
        let id = obj
            .get("message")
            .and_then(|m| m.get("id"))
            .and_then(|id| id.as_str())
            .unwrap_or("unknown");
        events.push(AgentEvent::TurnStarted { turn_id: id.to_string() });
    }

    // Emit only the new delta (difference from last snapshot)
    if full_text.len() > *last_len {
        let delta = &full_text[*last_len..];
        if !delta.is_empty() {
            events.push(AgentEvent::ContentDelta { text: delta.to_string() });
        }
    }
    *last_len = full_text.len();

    events
}

fn handle_stream_event(obj: &Value, turn_started: &mut bool) -> Vec<AgentEvent> {
    let event = match obj.get("event") {
        Some(e) => e,
        None => return vec![],
    };
    let inner = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match inner {
        "message_start" => {
            if !*turn_started {
                *turn_started = true;
                let id = event
                    .get("message")
                    .and_then(|m| m.get("id"))
                    .and_then(|id| id.as_str())
                    .unwrap_or("unknown");
                vec![AgentEvent::TurnStarted { turn_id: id.to_string() }]
            } else {
                vec![]
            }
        }
        "content_block_start" => {
            let block = event.get("content_block").unwrap_or(&Value::Null);
            if block.get("type").and_then(|t| t.as_str()) == Some("tool_use") {
                let name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool");
                vec![AgentEvent::ContentDelta {
                    text: format!("\n> *Using {name}...*\n"),
                }]
            } else {
                vec![]
            }
        }
        "content_block_delta" => {
            let delta = event.get("delta").unwrap_or(&Value::Null);
            let dtype = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");
            if dtype == "text_delta" {
                let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("");
                if text.is_empty() { vec![] } else {
                    vec![AgentEvent::ContentDelta { text: text.to_string() }]
                }
            } else {
                vec![]
            }
        }
        _ => vec![],
    }
}

fn handle_result(obj: &Value) -> Vec<AgentEvent> {
    let mut events = Vec::new();

    // Extract usage data if present
    let cost_usd = obj
        .get("total_cost_usd")
        .or_else(|| obj.get("cost_usd"))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let usage = obj.get("usage");
    let input_tokens = usage
        .and_then(|u| u.get("input_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let output_tokens = usage
        .and_then(|u| u.get("output_tokens"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_read = usage
        .and_then(|u| u.get("cache_read_input_tokens"))
        .or_else(|| usage.and_then(|u| u.get("cache_read_tokens")))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let cache_write = usage
        .and_then(|u| u.get("cache_creation_input_tokens"))
        .or_else(|| usage.and_then(|u| u.get("cache_write_tokens")))
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let model = obj
        .get("model")
        .and_then(|m| m.as_str())
        .unwrap_or("unknown")
        .to_string();

    if cost_usd > 0.0 || input_tokens > 0 || output_tokens > 0 {
        events.push(AgentEvent::UsageReport {
            input_tokens,
            output_tokens,
            cache_read_tokens: cache_read,
            cache_write_tokens: cache_write,
            cost_usd,
            model,
        });
    }

    let is_error = obj.get("is_error").and_then(|e| e.as_bool()).unwrap_or(false);
    if is_error {
        events.push(AgentEvent::SessionError {
            message: obj.get("result").and_then(|r| r.as_str()).unwrap_or("Error").to_string(),
        });
    } else {
        events.push(AgentEvent::TurnCompleted {
            turn_id: obj.get("session_id").and_then(|s| s.as_str()).unwrap_or("").to_string(),
        });
    }

    events
}
