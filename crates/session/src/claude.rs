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
            // bypassPermissions since there's no TTY for Claude Code's
            // own approval TUI — commands would silently hang otherwise.
            "--permission-mode", "bypassPermissions",
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
            // Track active content blocks by their stream index for tool use / thinking.
            let mut active_blocks = std::collections::HashMap::<u64, BlockInfo>::new();
            // Track model name from message_start so we can use it in result.
            let mut current_model: Option<String> = None;

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
                        let evts = handle_stream_event(&obj, &mut turn_started, &mut active_blocks, &mut current_model);
                        // Mark that we got real streaming deltas so we skip
                        // the `assistant` snapshot which would duplicate them.
                        if evts.iter().any(|e| matches!(e, AgentEvent::ContentDelta { .. }
                            | AgentEvent::ToolUseStart { .. }
                            | AgentEvent::ThinkingDelta { .. })) {
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
                        active_blocks.clear();
                        let evts = handle_result(&obj, &current_model);
                        current_model = None;
                        evts
                    }
                    "user" => {
                        // Tool results come back as "user" type messages
                        // with content array containing tool_result blocks
                        let content = obj
                            .get("message")
                            .and_then(|m| m.get("content"))
                            .and_then(|c| c.as_array());

                        if let Some(blocks) = content {
                            let mut evts = Vec::new();
                            for block in blocks {
                                if block.get("type").and_then(|t| t.as_str()) == Some("tool_result") {
                                    let tool_use_id = block
                                        .get("tool_use_id")
                                        .and_then(|id| id.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let result_content = block
                                        .get("content")
                                        .map(|c| {
                                            if let Some(s) = c.as_str() {
                                                s.to_string()
                                            } else {
                                                serde_json::to_string_pretty(c).unwrap_or_default()
                                            }
                                        })
                                        .unwrap_or_default();
                                    let is_error = block
                                        .get("is_error")
                                        .and_then(|e| e.as_bool())
                                        .unwrap_or(false);

                                    // Try to get the tool name from tool_use_result
                                    let tool_name = obj
                                        .get("tool_use_result")
                                        .and_then(|r| r.get("stdout"))
                                        .and_then(|s| s.as_str())
                                        .map(|_| "Bash".to_string())
                                        .unwrap_or_default();

                                    evts.push(AgentEvent::ToolResult {
                                        tool_id: tool_use_id,
                                        tool_name,
                                        content: result_content,
                                        is_error,
                                    });
                                }
                            }
                            evts
                        } else {
                            vec![]
                        }
                    }
                    _ => {
                        debug!("Unhandled Claude event type: {event_type}");
                        vec![]
                    }
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

/// Tracks active content blocks by their stream index.
#[derive(Debug, Clone)]
struct BlockInfo {
    block_type: String,
    tool_id: Option<String>,
    tool_name: Option<String>,
}

fn handle_stream_event(
    obj: &Value,
    turn_started: &mut bool,
    active_blocks: &mut std::collections::HashMap<u64, BlockInfo>,
    current_model: &mut Option<String>,
) -> Vec<AgentEvent> {
    let event = match obj.get("event") {
        Some(e) => e,
        None => return vec![],
    };
    let inner = event.get("type").and_then(|t| t.as_str()).unwrap_or("");

    match inner {
        "message_start" => {
            // Capture model from message_start for later use in usage report
            if let Some(model) = event
                .get("message")
                .and_then(|m| m.get("model"))
                .and_then(|m| m.as_str())
            {
                *current_model = Some(model.to_string());
            }

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
            let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
            let block = event.get("content_block").unwrap_or(&Value::Null);
            let block_type = block.get("type").and_then(|t| t.as_str()).unwrap_or("text");

            match block_type {
                "tool_use" => {
                    let tool_id = block.get("id").and_then(|id| id.as_str()).unwrap_or("").to_string();
                    let tool_name = block.get("name").and_then(|n| n.as_str()).unwrap_or("tool").to_string();
                    active_blocks.insert(index, BlockInfo {
                        block_type: "tool_use".into(),
                        tool_id: Some(tool_id.clone()),
                        tool_name: Some(tool_name.clone()),
                    });
                    vec![AgentEvent::ToolUseStart { tool_id, tool_name }]
                }
                "thinking" => {
                    active_blocks.insert(index, BlockInfo {
                        block_type: "thinking".into(),
                        tool_id: None,
                        tool_name: None,
                    });
                    vec![]
                }
                _ => {
                    active_blocks.insert(index, BlockInfo {
                        block_type: block_type.into(),
                        tool_id: None,
                        tool_name: None,
                    });
                    vec![]
                }
            }
        }
        "content_block_delta" => {
            let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
            let delta = event.get("delta").unwrap_or(&Value::Null);
            let dtype = delta.get("type").and_then(|t| t.as_str()).unwrap_or("");

            match dtype {
                "text_delta" => {
                    let text = delta.get("text").and_then(|t| t.as_str()).unwrap_or("");
                    if text.is_empty() {
                        vec![]
                    } else {
                        vec![AgentEvent::ContentDelta { text: text.to_string() }]
                    }
                }
                "input_json_delta" => {
                    let json_str = delta.get("partial_json").and_then(|j| j.as_str()).unwrap_or("");
                    if let Some(block) = active_blocks.get(&index) {
                        if let Some(tool_id) = &block.tool_id {
                            return vec![AgentEvent::ToolInputDelta {
                                tool_id: tool_id.clone(),
                                input_json: json_str.to_string(),
                            }];
                        }
                    }
                    vec![]
                }
                "thinking_delta" => {
                    let text = delta.get("thinking").and_then(|t| t.as_str()).unwrap_or("");
                    if text.is_empty() {
                        vec![]
                    } else {
                        vec![AgentEvent::ThinkingDelta { text: text.to_string() }]
                    }
                }
                _ => vec![],
            }
        }
        "content_block_end" => {
            let index = event.get("index").and_then(|i| i.as_u64()).unwrap_or(0);
            if let Some(block) = active_blocks.remove(&index) {
                if block.block_type == "tool_use" {
                    if let Some(tool_id) = block.tool_id {
                        return vec![AgentEvent::ToolUseEnd { tool_id }];
                    }
                }
            }
            vec![]
        }
        _ => vec![],
    }
}

fn handle_result(obj: &Value, current_model: &Option<String>) -> Vec<AgentEvent> {
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
    // Use model from result, fall back to model captured from message_start
    let model = obj
        .get("model")
        .and_then(|m| m.as_str())
        .or_else(|| current_model.as_deref())
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
