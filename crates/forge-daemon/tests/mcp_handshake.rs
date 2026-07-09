//! forge-mcp stdio server: JSON-RPC handshake + tool surface, driven by
//! spawning the real binary and talking to it over stdin/stdout.

use std::io::{BufRead, BufReader, Write};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use serde_json::{json, Value};

/// A spawned forge-mcp process with line-framed stdin/stdout.
struct McpProc {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl McpProc {
    /// Spawn with `cwd` set to a daemon-less directory so daemon discovery finds
    /// nothing (tool calls then report the daemon-down error deterministically).
    fn spawn(cwd: &std::path::Path) -> Self {
        let mut child = Command::new(env!("CARGO_BIN_EXE_forge-mcp"))
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawn forge-mcp");
        let stdin = child.stdin.take().expect("child stdin");
        let stdout = BufReader::new(child.stdout.take().expect("child stdout"));
        Self { child, stdin, stdout }
    }

    fn send(&mut self, req: &Value) {
        let line = serde_json::to_string(req).unwrap();
        self.stdin.write_all(line.as_bytes()).unwrap();
        self.stdin.write_all(b"\n").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Read exactly one JSON-RPC response line.
    fn recv(&mut self) -> Value {
        let mut line = String::new();
        let n = self.stdout.read_line(&mut line).expect("read response");
        assert!(n > 0, "forge-mcp closed stdout before responding");
        serde_json::from_str(&line).expect("response is valid json")
    }

    fn request(&mut self, req: &Value) -> Value {
        self.send(req);
        self.recv()
    }
}

impl Drop for McpProc {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn handshake_and_tool_surface() {
    let tmp = tempfile::tempdir().unwrap();
    let mut mcp = McpProc::spawn(tmp.path());

    // initialize echoes the client protocol version and advertises the tools cap.
    let init = mcp.request(&json!({
        "jsonrpc": "2.0", "id": 1, "method": "initialize",
        "params": { "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test", "version": "0" } }
    }));
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], "2025-06-18");
    assert_eq!(init["result"]["serverInfo"]["name"], "featureforge");
    assert!(init["result"]["capabilities"]["tools"].is_object());

    // notifications/initialized carries no id → the server must not reply. We
    // verify that implicitly: the very next line we read is the tools/list result.
    mcp.send(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));

    let list = mcp.request(&json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" }));
    assert_eq!(list["id"], 2);
    let tools = list["result"]["tools"].as_array().expect("tools array");
    let names: Vec<&str> = tools.iter().filter_map(|t| t["name"].as_str()).collect();
    for expected in [
        "list_features",
        "get_feature",
        "which_features",
        "feature_timeline",
        "record_note",
    ] {
        assert!(names.contains(&expected), "missing tool {expected} in {names:?}");
    }

    // tools/call with no daemon reachable → a tool-level error (isError), not a
    // JSON-RPC error, so the model sees a readable message.
    let call = mcp.request(&json!({
        "jsonrpc": "2.0", "id": 3, "method": "tools/call",
        "params": { "name": "list_features", "arguments": {} }
    }));
    assert_eq!(call["id"], 3);
    assert_eq!(call["result"]["isError"], true);
    let text = call["result"]["content"][0]["text"].as_str().unwrap_or_default();
    assert!(text.contains("daemon is not running"), "unexpected error text: {text}");

    // Unknown method → JSON-RPC -32601.
    let unknown = mcp.request(&json!({ "jsonrpc": "2.0", "id": 4, "method": "does/not/exist" }));
    assert_eq!(unknown["error"]["code"], -32601);

    // Bad params (missing required arg) → tool-level error, not a crash.
    let bad = mcp.request(&json!({
        "jsonrpc": "2.0", "id": 5, "method": "tools/call",
        "params": { "name": "get_feature", "arguments": {} }
    }));
    assert_eq!(bad["id"], 5);
    assert_eq!(bad["error"]["code"], -32602);
}
