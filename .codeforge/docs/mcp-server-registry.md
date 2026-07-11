# MCP Server Registry

## Purpose

Ensures the `forge-mcp` binary is installed at `~/.codeforge/bin/forge-mcp` and kept current with the built version. On every repo open, writes `.mcp.json` pointing Claude Code at this canonical copy so Claude can call CodeForge tools via stdio MCP.

## How it works

- **Auto-update on app start**: `ensure_mcp_binary()` locates the freshest `forge-mcp` build (sibling of the app exe, or `target/{debug,release}/forge-mcp` while walking up from the exe dir) and copies it to `~/.codeforge/bin` when newer by mtime.
- **Fallback to existing copy**: if no source build is found but `~/.codeforge/bin/forge-mcp` already exists, uses that copy (warns but proceeds).
- **Integration kit installer**: `forge_daemon::install_kit()` merges a `codeforge` stdio server entry into the repo's `.mcp.json`, pointing at the canonical `~/.codeforge/bin/forge-mcp` path.
- **Idempotent registration**: `.mcp.json` merge preserves all existing servers; the `codeforge` entry is updated only when command/args differ.
- **stdio proxy lifecycle**: the installed `forge-mcp` binary speaks JSON-RPC 2.0 over stdio, discovers the repo's daemon by walking up from cwd to find `.codeforge/runtime/daemon.json`, proxies tool calls (`list_features`, `get_feature`, `which_features`, `feature_timeline`, `record_note`) to the daemon's HTTP API.

## Key files

- `crates/tauri-app/src/runtime/mcp.rs` — binary sync: locate, copy, set exec perms
- `crates/forge-daemon/src/bin/forge-mcp.rs` — stdio MCP proxy: JSON-RPC parsing, daemon discovery, HTTP forwarding
- `crates/forge-daemon/src/kit.rs` — integration kit installer (`.mcp.json` merge is `install_mcp_json()`)
- `crates/tauri-app/src/runtime/repo_open.rs:72` — invokes `ensure_mcp_binary()` then `install_kit()` on every repo open

## Invariants & gotchas

- **Deterministic build selection**: `find_mcp_binary()` picks the newest by mtime across all candidates (sibling + all `target/{debug,release}` ancestors), never silently favours stale debug over fresh release or vice versa.
- **No silent failures**: when no build exists and no existing copy is present, `ensure_mcp_binary()` returns `Err` so repo-open surfaces "MCP integration unavailable" rather than registering a broken path.
- **Single-repo daemon scope**: `forge-mcp` stops at the FIRST ancestor with `.codeforge/` — a corrupt/missing `daemon.json` in a nested repo never falls through to a parent repo's daemon (which would answer for the wrong index).
- **Unix exec bits**: newly copied binaries get `chmod 0o755` on Unix; Windows no-op.
- **Idempotent `.mcp.json` writes**: the installer never clobbers user servers or rewrites the file when the `codeforge` entry already matches.
