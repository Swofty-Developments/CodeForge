Perfect! I have all the information needed. Now I'll write the living doc for the Terminal PTY Manager feature.

# Purpose

Manages worktree-scoped PTY tabs (FZ-3). Spawns login shells (bash/zsh) in pseudo-terminals, relays stdin/stdout between the frontend's xterm instances and the PTY master, and emits output/exit events. Each terminal is a uuid-keyed `PtyProcess` owned by `TerminalManager`, which lives in `AppState`.

# How it works

**Spawning:** `open_terminal(cwd, shell)` resolves the shell (explicit arg → login-shell `$SHELL` → process `$SHELL` → OS default `/bin/zsh` on macOS), spawns a `portable_pty` pair in that cwd, enriches the env with `forge_session::shell_env` (so PATH/nvm shims resolve) + `TERM=xterm-256color`, and returns a uuid terminal id. The manager records a `Terminal{cwd, title, pty}` in its `Mutex<HashMap<id, Terminal>>`.

**I/O:** A blocking reader thread owns the `Child` and streams PTY output to the frontend on the `terminal:data` event — **chunks are base64-encoded** so raw bytes survive JSON. The frontend decodes and feeds them to xterm. Input keystrokes arrive as **raw UTF-8** via `write_terminal` and are written straight to the PTY master's writer.

**Exit:** When the child closes, the reader hits EOF, reaps the exit code, emits `terminal:exit{id, code}` once, and ends. `code` is `None` if the reap failed (a named "unknown", not a silent zero).

**Teardown:** `close_terminal` kills the child (its reader then hits EOF and ends) and removes it from the map. `TerminalManager::Drop` kills every remaining child — even through a poisoned lock — so closing the app leaves no orphan shells.

# Key files

- **crates/tauri-app/src/terminal/mod.rs** — public API: `TerminalManager`, event names (`terminal:data`, `terminal:exit`), and payload types.
- **crates/tauri-app/src/terminal/manager.rs** — owns the `Mutex<HashMap<id, Terminal>>`, resolves the shell, open/write/resize/close/list ops, and `AppSink` that bridges to Tauri events.
- **crates/tauri-app/src/terminal/pty.rs** — `PtyProcess::spawn`: opens the PTY pair, enriches the env, spawns the child, and wires the blocking reader thread. Sink-generic so tests use a channel sink instead of Tauri events.
- **crates/tauri-app/src/commands/terminals.rs** — Tauri command wrappers around `TerminalManager`.

# Invariants & gotchas

- **Base64 asymmetry:** PTY output → frontend is base64-encoded (raw bytes survive JSON); frontend → PTY is raw UTF-8 (xterm already has text).
- **Teardown is explicit, never best-effort:** `close_terminal` kills the child, `TerminalManager::Drop` kills every remaining child even through a poisoned lock. No orphan shells.
- **The slave must be dropped before reading:** the master reader never sees EOF if the slave is still open. `PtyProcess::spawn` drops `pair.slave` immediately after spawning the child.
- **Lock errors are named, not swallowed:** a poisoned `Mutex<HashMap>` surfaces as `TerminalError::LockPoisoned`, never an empty `list()` or silent write failure.
- **Reader thread owns the child:** it reaps the exit code after EOF so `terminal:exit` carries the real code, not `None` from a racy wait-before-close.
- **Initial geometry is placeholder (80×24):** the frontend issues `resize_terminal` as soon as xterm mounts with its measured dimensions.
