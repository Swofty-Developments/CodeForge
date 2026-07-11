None of the changed files relate to the MCP stdio server (`forge-mcp.rs`). The edits are:
- `app-store.ts`: Tauri frontend store (session pane state)
- `headless.rs`: Claude CLI indexing invocation (unrelated to the MCP server)
- `SessionPane.tsx`: Frontend session UI component
- `styles-chrome.ts`: New frontend styles file

The living doc remains accurate and requires no updates.

---
# MCP Stdio Server

## Purpose

Standalone stdio MCP server binary (`forge-mcp`) that exposes CodeForge's five tools (`list_features`, `get_feature`, `which_features`, `feature_timeline`, `record_note`) to any MCP client. Discovers the per-repo daemon by walking up from cwd and proxies all tool calls to the daemon's HTTP API.

## How it works

- Hand-rolled JSON-RPC 2.0 server over stdin/stdout (no dependencies on the MCP SDK).
- On each `tools/call`, walks up from `cwd` looking for the **first** `.codeforge/` ancestor, then reads `.codeforge/runtime/daemon.json` to extract the daemon's advertised HTTP `port`.
- Spawns a throwaway current-thread tokio runtime to proxy the tool call to `http://127.0.0.1:{port}/api/{endpoint}` (10s timeout), then formats the JSON response and returns it as MCP text content.
- Returns a user-facing error when the repo isn't tracked (no manifest) or the manifest is corrupt/portless (stale from an unclean exit) — never silently answers for a parent repo.
- Tool schema frozen as a stable contract: changing signatures breaks MCP clients that hard-code these tools.

## Key files

- **`crates/forge-daemon/src/bin/forge-mcp.rs`** — complete stdio server (parsing, dispatch, discovery, proxy).

## Invariants & gotchas

- **Discovery stops at the FIRST `.codeforge/` ancestor** — never walks past a corrupt manifest into a parent repo's daemon, which would silently answer for the wrong repo.
- **Tool signatures (name, argument names, types) are a frozen contract** — changing them breaks MCP clients that hard-code these tools; only add new tools or optional arguments.
- **Requests without an `id` are notifications** — never reply, per JSON-RPC spec.
- **Throwaway runtime per call** — no persistent HTTP pool; simple but not optimized for high-frequency calls.
- **Pretty-prints JSON responses** for model readability; passes through non-JSON verbatim.
