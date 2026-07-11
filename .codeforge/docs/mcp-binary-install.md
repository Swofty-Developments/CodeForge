# MCP Binary Install

## Purpose

Ensures `~/.codeforge/bin/forge-mcp` always reflects the latest built MCP server binary. On app start, locates the freshest `forge-mcp` executable (sibling of the running app, or under `target/{debug,release}` while walking up the directory tree) and copies it to `~/.codeforge/bin/forge-mcp` when newer. This canonical path is registered in each repo's `.mcp.json` by `forge-daemon::install_kit`, so Claude Code sessions can invoke the MCP tools regardless of dev vs. release builds or Cargo workspace layout.

## How it works

Entry point is `ensure_mcp_binary()` in `runtime/mcp.rs`, called by `repo_open.rs:72` during every `open_context` flow (triggered by `open_repo` command and `create_worktree`).

**Source discovery** (`locate_source` → `find_mcp_binary`):  
1. Collect candidates: the sibling of the running exe (`current_exe().parent()/forge-mcp`), plus every `target/debug/forge-mcp` and `target/release/forge-mcp` found while walking up from the exe's directory.  
2. Pick the **newest by mtime** — never silently favours a stale debug build over a fresher release one (or vice versa).  
3. Return `None` if no candidates exist.

**Install logic** (`ensure_mcp_binary`):  
- Creates `~/.codeforge/bin/` if absent.  
- When a source is found and differs from the destination, `copy_if_newer` installs it (compares mtime, copies when source is newer or destination missing, sets executable bit on Unix via `chmod 0o755`).  
- When source resolves to the canonical copy itself (already in place), returns `Ok(dest)` immediately.  
- When no source is found but a previous copy exists, logs a warning and returns `Ok(dest)` — the old build remains usable.  
- When no source AND no existing copy, returns `Err` so `open_context` surfaces "MCP integration unavailable" instead of registering a path to nothing.

**Cross-platform notes**:  
- `home_dir` tries `$HOME` then `%USERPROFILE%` — fails when both absent.  
- `set_exec` is Unix-only; no-op on Windows (ACLs unnecessary for exe).

## Key files

- `crates/tauri-app/src/runtime/mcp.rs` — the install logic (153 lines, including tests).  
- `crates/tauri-app/src/runtime/repo_open.rs:72` — calls `ensure_mcp_binary()` during `open_context`.  
- `~/.codeforge/bin/forge-mcp` — the canonical destination, registered in `.mcp.json` by `forge-daemon::install_kit`.

## Invariants & gotchas

- **mtime-based selection**: `find_mcp_binary` always picks the newest candidate by modification time across debug + release profiles. If you rebuild in debug after a release build, the debug binary wins — deterministic, never stale.  
- **Graceful degradation**: when the source binary can't be found (e.g. app bundled without `forge-mcp` sibling), an existing `~/.codeforge/bin/forge-mcp` from a prior dev session remains usable. Only hard-fails when both source AND destination are absent.  
- **Idempotency**: `copy_if_newer` only overwrites when the source is strictly newer or the destination is missing — safe to call on every app start.  
- **Caller contract**: `repo_open.rs:open_context` maps `ensure_mcp_binary` errors to user-facing `Err(String)`, surfacing "MCP integration unavailable" in the frontend when the binary genuinely can't be provided.
