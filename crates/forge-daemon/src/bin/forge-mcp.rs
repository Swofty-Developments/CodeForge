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
use std::path::PathBuf;

use serde_json::{json, Value};

/// Repo isn't tracked by FeatureForge (no manifest) — the user should open it.
const MSG_NO_MANIFEST: &str =
    "FeatureForge is not tracking this repo yet — open it in FeatureForge.";
/// A manifest exists but the daemon is corrupt/unreachable — reopen the repo.
const MSG_STALE: &str =
    "FeatureForge is not reachable — the app exited uncleanly; reopen the repo in FeatureForge.";

fn main() {
    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(req) = serde_json::from_str::<Value>(&line) else {
            continue;
        };
        // Requests without an id are notifications: never reply.
        let Some(id) = req.get("id").cloned() else {
            continue;
        };
        let method = req.get("method").and_then(Value::as_str).unwrap_or("");
        let params = req.get("params").cloned().unwrap_or(Value::Null);

        let response = match method {
            "initialize" => {
                // Echo the client's protocol version per the MCP handshake.
                let version = params
                    .get("protocolVersion")
                    .and_then(Value::as_str)
                    .unwrap_or("2025-06-18");
                json!({
                    "jsonrpc": "2.0", "id": id,
                    "result": {
                        "protocolVersion": version,
                        "capabilities": { "tools": {} },
                        "serverInfo": { "name": "featureforge", "version": env!("CARGO_PKG_VERSION") }
                    }
                })
            }
            "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
            "tools/list" => json!({
                "jsonrpc": "2.0", "id": id,
                "result": { "tools": tool_definitions() }
            }),
            "tools/call" => match handle_tool_call(&params) {
                Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
                Err(rpc) => json!({ "jsonrpc": "2.0", "id": id, "error": rpc }),
            },
            _ => json!({
                "jsonrpc": "2.0", "id": id,
                "error": { "code": -32601, "message": format!("method not found: {method}") }
            }),
        };

        if writeln!(stdout, "{response}").and_then(|_| stdout.flush()).is_err() {
            break;
        }
    }
}

fn invalid_params(message: impl Into<String>) -> Value {
    json!({ "code": -32602, "message": message.into() })
}

fn text_result(text: String, is_error: bool) -> Value {
    let mut result = json!({ "content": [{ "type": "text", "text": text }] });
    if is_error {
        result["isError"] = Value::Bool(true);
    }
    result
}

/// Dispatch one tools/call. `Ok` is an MCP result (possibly `isError`),
/// `Err` a JSON-RPC error object (bad params / internal).
fn handle_tool_call(params: &Value) -> Result<Value, Value> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or_else(|| invalid_params("missing tool name"))?;
    let args = params.get("arguments").cloned().unwrap_or(json!({}));
    let str_arg = |key: &str| -> Result<String, Value> {
        args.get(key)
            .and_then(Value::as_str)
            .map(str::to_owned)
            .ok_or_else(|| invalid_params(format!("{name}: missing required argument \"{key}\"")))
    };

    let request = match name {
        "list_features" => HttpCall::Get("/api/features".into()),
        "get_feature" => HttpCall::Get(format!("/api/features/{}", str_arg("slug")?)),
        "which_features" => HttpCall::GetWithQuery(
            "/api/classify".into(),
            vec![("path".into(), str_arg("path")?)],
        ),
        "feature_timeline" => {
            let mut query = vec![("feature".into(), str_arg("slug")?)];
            if let Some(limit) = args.get("limit").and_then(Value::as_u64) {
                query.push(("limit".into(), limit.to_string()));
            }
            HttpCall::GetWithQuery("/api/timeline".into(), query)
        }
        "record_note" => HttpCall::Post(
            "/api/notes".into(),
            json!({
                "text": str_arg("text")?,
                "featureSlugs": args.get("feature_slugs").cloned().unwrap_or(json!([])),
            }),
        ),
        other => return Err(invalid_params(format!("unknown tool: {other}"))),
    };

    let port = match discover_daemon() {
        DaemonLocation::Port(port) => port,
        DaemonLocation::NoManifest => return Ok(text_result(MSG_NO_MANIFEST.into(), true)),
        DaemonLocation::StaleManifest => return Ok(text_result(MSG_STALE.into(), true)),
    };
    match proxy_to_daemon(port, request) {
        Ok(ProxyOutcome::Ok(body)) => Ok(text_result(body, false)),
        Ok(ProxyOutcome::HttpError(status, body)) => {
            Ok(text_result(format!("daemon returned {status}: {body}"), true))
        }
        // The manifest advertised a port but nothing answered: stale port from an
        // uncleanly-exited app, not a "never opened" state.
        Ok(ProxyOutcome::Unreachable) => Ok(text_result(MSG_STALE.into(), true)),
        Err(message) => Err(json!({ "code": -32603, "message": message })),
    }
}

/// Outcome of locating this repo's daemon manifest.
enum DaemonLocation {
    /// Repo root has a manifest advertising a usable `port`.
    Port(u16),
    /// A FeatureForge repo (`.featureforge/` present) with no daemon manifest —
    /// never opened, or not currently open.
    NoManifest,
    /// Manifest present but corrupt/portless — the app exited uncleanly.
    StaleManifest,
}

enum HttpCall {
    Get(String),
    GetWithQuery(String, Vec<(String, String)>),
    Post(String, Value),
}

enum ProxyOutcome {
    Ok(String),
    HttpError(u16, String),
    Unreachable,
}

/// Walk up from cwd, stopping at the FIRST ancestor that contains a
/// `.featureforge/` dir — that IS this repo's root. Its manifest alone decides
/// the state; we never walk past a corrupt/portless manifest into a parent
/// repo's daemon (which would silently answer for the wrong repo).
fn discover_daemon() -> DaemonLocation {
    let Ok(cwd) = std::env::current_dir() else {
        return DaemonLocation::NoManifest;
    };
    for dir in cwd.ancestors() {
        if !dir.join(".featureforge").is_dir() {
            continue;
        }
        let manifest: PathBuf = dir.join(".featureforge").join("runtime").join("daemon.json");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            return DaemonLocation::NoManifest;
        };
        return match parse_manifest_port(&text) {
            Some(port) => DaemonLocation::Port(port),
            None => DaemonLocation::StaleManifest,
        };
    }
    DaemonLocation::NoManifest
}

/// Extract the advertised `port` from a `daemon.json` body.
fn parse_manifest_port(text: &str) -> Option<u16> {
    serde_json::from_str::<Value>(text)
        .ok()
        .as_ref()
        .and_then(|v| v.get("port"))
        .and_then(Value::as_u64)
        .and_then(|p| u16::try_from(p).ok())
}

/// Proxy one call to the daemon on a throwaway current-thread runtime.
fn proxy_to_daemon(port: u16, call: HttpCall) -> Result<ProxyOutcome, String> {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| format!("failed to start runtime: {e}"))?;

    runtime.block_on(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("failed to build http client: {e}"))?;
        let base = format!("http://127.0.0.1:{port}");
        let request = match call {
            HttpCall::Get(path) => client.get(format!("{base}{path}")),
            HttpCall::GetWithQuery(path, query) => {
                client.get(format!("{base}{path}")).query(&query)
            }
            HttpCall::Post(path, body) => client.post(format!("{base}{path}")).json(&body),
        };

        let response = match request.send().await {
            Ok(r) => r,
            Err(e) if e.is_connect() || e.is_timeout() => return Ok(ProxyOutcome::Unreachable),
            Err(e) => return Err(format!("request failed: {e}")),
        };
        let status = response.status().as_u16();
        let body = response
            .text()
            .await
            .map_err(|e| format!("failed to read response: {e}"))?;
        if !(200..300).contains(&status) {
            return Ok(ProxyOutcome::HttpError(status, body));
        }
        // Pretty-print for the model; pass through verbatim if not JSON.
        let pretty = serde_json::from_str::<Value>(&body)
            .and_then(|v| serde_json::to_string_pretty(&v))
            .unwrap_or(body);
        Ok(ProxyOutcome::Ok(pretty))
    })
}

fn tool_definitions() -> Value {
    json!([
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
