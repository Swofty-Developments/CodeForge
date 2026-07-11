# Shell Environment

**Purpose:** Ensures desktop-launched CodeForge finds the same `node`/`claude` binaries as the user's terminal by resolving the login-shell environment once per process and merging it over spawned commands.

**Why it exists:** macOS/Linux GUI apps (Finder/dock launches) receive a minimal environment without shell rc files; version managers (nvm, fnm, asdf) install shims only on the shell PATH, so a bare spawn of `node` would fail or pick a system binary with mismatched shared libraries.

**How it works:**

1. **One-shot resolution** (`shell_env::get()` via `OnceLock`):
   - Tries `$SHELL -l -i -c 'env -0'` (interactive login shell, NUL-separated output).
   - Falls back to `$SHELL -l -c 'env'` (non-interactive login, newline-separated).
   - If both fail, returns `ShellEnv::Unresolved` wrapping the minimal process env + a tracing warning.
   - Sets `TERM=dumb` to prevent rc files from prompting for input.

2. **Usage:**
   - `shell_env::which(cmd)` searches the resolved PATH for a binary (used for `node` in `claude.rs:182`, `claude` in `forge-index/src/headless.rs:26`).
   - `shell_env::apply(&mut cmd)` merges resolved vars **over** the existing command env (preserves Tauri-specific vars).
   - `shell_env::is_resolved()` checked in error messages to explain whether the login shell was reached.

3. **Callsites:**
   - `claude.rs`: spawns sidecar via `which("node")` + `apply(cmd)`.
   - `forge-index/headless.rs`: spawns `claude` CLI for indexing.
   - `tauri-app/terminal/pty.rs`: merges shell env into PTY spawn.
   - `tauri-app/terminal/manager.rs`: reads `SHELL` var for terminal shell choice.

**Key files:**
- `crates/forge-session/src/shell_env.rs` — resolution logic, cached singleton, env merge
- `crates/forge-session/src/locate.rs` — sidecar script path resolution (bundled + dev walk-up)
- `crates/forge-session/src/claude.rs` — sidecar spawn + `node_not_found_error` message branching on `is_resolved()`

**Invariants & gotchas:**
- The env is **merged over** command env, never replacing it wholesale (preserves `TAURI_*`, `RUST_LOG`, etc.).
- `Unresolved` is a NAMED state — callers can surface a meaningful error ("login-shell probes failed") rather than a generic "node not found."
- `which()` searches the resolved PATH even if `Unresolved`, but falls back to process `PATH` if the resolved map has none.
- NUL-separated parsing (`env -0`) handles values containing newlines; the fallback (`env` alone) splits on `\n` and tolerates multiline values poorly.
- Keys with whitespace or empty strings are skipped during parse (invalid env var names).
- If the parsed map is empty after filtering, the probe is considered failed (returns `None`).
