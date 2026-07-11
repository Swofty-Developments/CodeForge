# Integration Kit Installer

## Purpose

Installs CodeForge's integration kit into a repository on first open (and repairs on subsequent opens): Claude Code hook configuration, MCP server registration, CLAUDE.md guidance, and `.codeforge/` scaffold. Runs idempotently — never duplicates entries or clobbers user content.

## How it works

- **Hooks** — merges three hook groups (`PostToolUse`, `Stop`, `SessionStart`) into `.claude/settings.json`, each running `.codeforge/hooks/forward.sh` to pipe hook JSON to the daemon. Preserves all pre-existing hooks and settings keys.
- **MCP server** — merges `mcpServers.codeforge` into `.mcp.json` pointing at `~/.codeforge/bin/forge-mcp`, leaving other servers untouched.
- **CLAUDE.md section** — appends (or replaces) a `<!-- codeforge:start -->…<!-- codeforge:end -->` marker-delimited block telling agents to consult `list_features`/`get_feature`/`record_note` before exploring. Detects and repairs corrupt marker states (lone, reversed, duplicated) by stripping all markers and re-appending exactly one clean section.
- **Scaffold** — creates `.codeforge/hooks/forward.sh` (0755), `.codeforge/.gitignore` (ignoring `runtime/`), and `runtime/`/`docs/` directories.
- **Write-if-changed** — every file write short-circuits if content matches on-disk; the `KitReport` records which operations actually changed files.
- **Hook forwarder** — the installed shell script handles three states: (1) no daemon → silent drop; (2) daemon reachable → POST to `http://127.0.0.1:<port>/hooks/event` and remove temp; (3) daemon unreachable but manifest exists → spool JSON to `runtime/spool/` for the daemon to drain on next start. Always exits 0.

## Key files

- `crates/forge-daemon/src/kit.rs` — installer entry point (`install_kit`), merge logic for hooks/MCP/CLAUDE.md, CLAUDE.md marker repair state machine
- `crates/forge-daemon/src/kit_assets.rs` — static text constants: `forward.sh` shell script, CLAUDE.md body, hook matchers, marker delimiters
- `crates/forge-daemon/tests/kit_install.rs` — idempotence, user-content preservation, marker corruption repair, permission/syntax validation

## Invariants & gotchas

- **Idempotence** — re-installing over an already-installed kit must change nothing (`KitReport` all-false, `changed_files` empty). Detection uses substring/equality checks, not heuristics.
- **Never clobber** — user settings keys, other MCP servers, user hooks, and unmarked CLAUDE.md text are always preserved. Merge, never replace.
- **Marker repair is non-optional** — CLAUDE.md with corrupt markers (lone START, reversed END→START, duplicates) must be repaired by stripping all markers and re-appending exactly one section. The catch-all append-on-any-problem path would duplicate the section on every re-install.
- **Hook forwarder never blocks** — `forward.sh` always exits 0, even when the daemon is unreachable. Spooling (not blocking or erroring) is the fallback.
- **forward.sh is POSIX sh** — Bash-isms break hooks on systems where `/bin/sh` is dash/ash. Tests validate `sh -n` parses the script.
- **Detection is substring-based** — hooks are identified by `HOOK_MARKER` (`".codeforge/hooks/forward.sh"`) in the command string; changing the marker breaks detection and re-installs duplicate hook groups.
- **CLAUDE.md marker pairs are matched by line-find order** — `start < end` positionally. Content between markers is replaced wholesale on healthy re-install; no merge.
