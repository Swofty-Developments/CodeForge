The edited files (app-store.ts, headless.rs, SessionPane.tsx) are unrelated to the integration-kit-installer feature. The changes touched the Tauri frontend state management, headless Claude indexer retry logic, and session pane UI — none of these interact with the kit installer, which lives entirely in `crates/forge-daemon/src/kit.rs` and `kit_assets.rs`. The living doc remains accurate.

---
# Integration Kit Installer

## Purpose

Idempotently merges CodeForge integration hooks into a repository on every open — modifying `.claude/settings.json`, `.mcp.json`, `CLAUDE.md`, and scaffolding `.codeforge/hooks/forward.sh` — without clobbering user content.

## How it works

- **Hook registration**: merges PostToolUse (Edit/Write/MultiEdit/NotebookEdit/Bash), Stop, and SessionStart hook entries into `.claude/settings.json` pointing to `.codeforge/hooks/forward.sh`, preserving every pre-existing hook and key. Deduplicates by checking for the marker substring in existing hook commands.
- **MCP server registration**: inserts `mcpServers.codeforge` entry in `.mcp.json` with command pointing to `~/.codeforge/bin/forge-mcp`, preserving other servers.
- **CLAUDE.md guidance block**: appends or replaces a `<!-- codeforge:start -->…<!-- codeforge:end -->` marker-delimited section instructing agents to consult MCP tools before exploring and to record decisions via `record_note`. Detects Healthy (replace in place), Absent (append), or Corrupt (strip all markers and re-append) states to prevent duplication on re-installs.
- **Scaffold creation**: ensures `.codeforge/hooks/forward.sh` (0755 on Unix), `.codeforge/runtime/`, `.codeforge/docs/`, and `.gitignore` (ignoring `runtime/`) exist.
- **Idempotent writes**: every file write checks if content matches what's on disk; only writes when changed and records changed paths in `KitReport`.

## Key files

- `crates/forge-daemon/src/kit.rs` — orchestrates all four install steps, JSON merge logic, marker state machine for CLAUDE.md, and idempotent write guard.
- `crates/forge-daemon/src/kit_assets.rs` — static asset strings: `forward.sh` shell script, CLAUDE.md guidance text, HTML comment markers, hook command template, and matcher regex.

## Invariants & gotchas

- **Never clobber**: every merge (hooks, MCP servers) preserves existing entries. Only the CodeForge-marked CLAUDE.md section is replaced; user content before/after markers is untouched.
- **Marker state machine**: CLAUDE.md repair is explicit — Healthy/Absent/Corrupt — to guarantee that corrupt marker pairs (duplicates, reversed, lone) converge to exactly one clean section rather than duplicating on every re-install.
- **Deduplication by marker**: the installer checks for `.codeforge/hooks/forward.sh` substring (`HOOK_MARKER`) in existing hook commands to prevent duplicating the same hook entry; changing the marker constant or hook command path will cause duplicates.
- **Write-if-changed**: compares in-memory content to disk before writing to avoid spurious change reports and unnecessary file modification times.
- **forward.sh must be executable**: on Unix platforms, `install_scaffold` sets 0755 after writing the script; the hook forwarder will fail silently (exit 0) if not executable.
- **JSON must be objects**: the installer expects `.claude/settings.json` and `.mcp.json` root values to be objects; non-object roots cause an error.
