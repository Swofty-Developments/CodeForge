Based on my investigation, this feature doesn't actually locate the MCP settings file itself — it delegates that to the Agent SDK by setting `settingSources: ["user", "project", "local"]`. The feature is about **configuring the sidecar to inherit user MCP servers** by enabling the SDK's filesystem settings loader.

# MCP Server Registry

## Purpose

Configures the sidecar to inherit the user's MCP server definitions from `~/.claude/mcp.json` (or `~/AppData/Roaming/Claude/mcp.json` on Windows) so embedded Claude sessions can access the same MCP tools as terminal sessions.

## How it works

- **Delegate to SDK**: sets `options.settingSources = ["user", "project", "local"]` when calling the Agent SDK's `query()` function.
- **User-level MCP config**: the `"user"` source instructs the SDK to load `~/.claude/mcp.json` (Mac/Linux) or `~/AppData/Roaming/Claude/mcp.json` (Windows) containing globally configured MCP servers.
- **Project-level MCP config**: the `"project"` source loads `.mcp.json` from the repo root (where the CodeForge integration kit installer registers `forge-mcp`).
- **Local overrides**: the `"local"` source loads `.claude/settings.local.json` for machine-specific settings.
- **SDK responsibility**: the Agent SDK resolves platform-specific paths and parses MCP configurations; the sidecar never touches MCP JSON directly.

## Key files

- `crates/tauri-app/agent-sidecar/index.mjs:158` — sets `settingSources` array before every `query()` call

## Invariants & gotchas

- **Always includes all three sources**: omitting `"user"` would disconnect embedded sessions from the user's personal MCP servers; omitting `"project"` would skip CLAUDE.md and `.mcp.json`; omitting `"local"` would drop machine-specific overrides.
- **SDK owns path resolution**: platform differences (Mac `~/.claude/` vs Windows `%APPDATA%\Claude\`) are handled by the Agent SDK, not the sidecar.
- **Static configuration**: `settingSources` is set once per query turn; changes to `~/.claude/mcp.json` mid-turn won't be picked up until the next query.
- **No MCP state in sidecar**: the sidecar process is stateless regarding MCP definitions — every query re-reads from disk via the SDK.
