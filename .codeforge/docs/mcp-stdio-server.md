## MCP Stdio Server

**Purpose**

The `forge-mcp` binary is a hand-rolled JSON-RPC 2.0 server running over stdio that exposes CodeForge's feature index to MCP clients (e.g., Claude). It discovers the daemon by walking up from `cwd` to find `.codeforge/runtime/daemon.json`, then proxies all tool calls to the daemon's HTTP API.

**How it works**

- Reads line-delimited JSON-RPC 2.0 requests from stdin, writes responses to stdout.
- Implements three core methods: `initialize` (MCP handshake), `tools/list` (advertises five tools), and `tools/call` (proxies to daemon).
- Discovery: walks ancestors from `cwd` until the first `.codeforge/` is found, then reads `daemon.json` for the port.
- Proxies each tool call to the daemon's HTTP API on a throwaway current-thread tokio runtime with a 10s timeout.
- Returns tool-level errors (`isError: true`) for unreachable/stale daemons (readable by the model), JSON-RPC errors for bad params/unknown methods.

**Key files**

- **crates/forge-daemon/src/bin/forge-mcp.rs** — entire server implementation (289 lines): JSON-RPC dispatch, daemon discovery, HTTP proxy.
- **crates/forge-daemon/tests/mcp_handshake.rs** — integration test spawning the real binary, verifying handshake + tool surface + error states.

**Invariants & gotchas**

- **Frozen tool surface**: the five tools (`list_features`, `get_feature`, `which_features`, `feature_timeline`, `record_note`) and their argument names are a contract — renaming breaks deployed MCP clients.
- **Daemon discovery stops at the first `.codeforge/`**: never walks past a corrupt manifest into a parent repo's daemon (which would answer for the wrong repo).
- **Notifications (id-less requests) never get a reply**: the server silently skips them (e.g., `notifications/initialized`) per JSON-RPC spec.
- **Unreachable daemon is a tool-level error, not a crash**: returned as `isError: true` so the model sees the user-facing message ("open it in CodeForge" or "reopen the repo"), not an internal error.
- **HTTP responses are pretty-printed if JSON**: improves model readability; non-JSON bodies pass through verbatim.
- **Throwaway tokio runtime per call**: no persistent runtime — each `tools/call` spawns a fresh current-thread runtime to avoid static lifecycle issues.
