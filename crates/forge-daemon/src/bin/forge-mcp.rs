//! forge-mcp — MCP stdio server proxying to the per-repo FeatureForge daemon.
//!
//! Hand-rolled JSON-RPC 2.0 over stdio (initialize, tools/list, tools/call).
//! Discovers the daemon by walking up from cwd looking for
//! `.featureforge/runtime/daemon.json`, then proxies tool calls to its HTTP API.
//!
//! Tool surface (frozen contract):
//! - `list_features` ()
//! - `get_feature` (slug: string)
//! - `which_features` (path: string)
//! - `feature_timeline` (slug: string, limit?: number)
//! - `record_note` (text: string, feature_slugs?: string[])

use std::io::{BufRead, Write};

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or("");

        let response = match method {
            "initialize" => Some(serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "result": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "featureforge", "version": env!("CARGO_PKG_VERSION") }
                }
            })),
            "notifications/initialized" => None,
            "tools/list" => Some(serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "result": { "tools": tool_definitions() }
            })),
            "tools/call" => {
                // IMPLEMENT(agent): read daemon.json (walk up from cwd), proxy the
                // tool call to http://127.0.0.1:{port}/api/... via reqwest::blocking
                // (or a small tokio runtime), return MCP content blocks.
                Some(serde_json::json!({
                    "jsonrpc": "2.0", "id": id,
                    "error": { "code": -32001, "message": "not implemented: tools/call proxy" }
                }))
            }
            _ => Some(serde_json::json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            })),
        };

        if let Some(resp) = response {
            let _ = writeln!(stdout, "{resp}");
            let _ = stdout.flush();
        }
    }
}

fn tool_definitions() -> serde_json::Value {
    serde_json::json!([
        {
            "name": "list_features",
            "description": "List all features of this repository from the FeatureForge index.",
            "inputSchema": { "type": "object", "properties": {}, "required": [] }
        },
        {
            "name": "get_feature",
            "description": "Get one feature (description, entry points, files, tags) by slug.",
            "inputSchema": { "type": "object", "properties": { "slug": { "type": "string" } }, "required": ["slug"] }
        },
        {
            "name": "which_features",
            "description": "Which features does a file path belong to?",
            "inputSchema": { "type": "object", "properties": { "path": { "type": "string" } }, "required": ["path"] }
        },
        {
            "name": "feature_timeline",
            "description": "Recent timeline events for a feature, newest first.",
            "inputSchema": {
                "type": "object",
                "properties": { "slug": { "type": "string" }, "limit": { "type": "number" } },
                "required": ["slug"]
            }
        },
        {
            "name": "record_note",
            "description": "Record a decision/note on the repo timeline, optionally tagged to features.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "text": { "type": "string" },
                    "feature_slugs": { "type": "array", "items": { "type": "string" } }
                },
                "required": ["text"]
            }
        }
    ])
}
