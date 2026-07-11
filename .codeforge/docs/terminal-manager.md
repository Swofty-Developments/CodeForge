Now I have a complete picture of the terminal manager implementation. Let me write the first version of the living doc:

# Terminal Manager

**Purpose:** PTY session registry for embedded shell terminals. Spawns interactive shells per worktree, streams output to the frontend via Tauri events, and ensures clean teardown when a terminal or the app closes.

**How it works:**

One `TerminalManager` lives in Tauri `AppState`, holding a map of uuid → `Terminal` (shell PTY + metadata). Each terminal runs in a `portable_pty` pseudo-terminal with a dedicated blocking reader thread:

1. **Spawn:** `open(cwd, shell)` resolves the shell (explicit arg → login-shell `$SHELL` → process `$SHELL` → `/bin/zsh`), spawns a `PtyProcess` in that cwd with the login-shell environment + `TERM=xterm-256color`, and launches a reader thread that base64-encodes output chunks and emits them on `terminal:data`.
2. **I/O:** Frontend writes raw UTF-8 keystrokes via `write(id, bytes)`; the writer flushes immediately. Resize calls forward `SIGWINCH` to the child via `resize(id, cols, rows)`.
3. **Exit:** When the child dies, the reader hits EOF, reaps the exit code via `child.wait()`, and fires a one-shot `terminal:exit` event with the code (`None` if reaping failed).
4. **Cleanup:** `close(id)` kills the child and drops the terminal. `TerminalManager::drop` kills every remaining child (even through a poisoned lock) so closing the app leaves no orphan shells.

Frontend (`TerminalPanel.tsx`) listens to `terminal:data` + `terminal:exit`, routing output to the matching `xterm` instance by id. Output is base64-decoded via `atob` → `Uint8Array` before xterm writes it. The panel is a resizable dock with a tab strip (one tab per PTY + a "+" for new), styled to match the worktree tabs.

**Key files:**

- `terminal/pty.rs` — low-level `PtyProcess`: spawn shell, reader thread, write/resize/kill. Tauri-free; output goes to a `TerminalSink` trait (tests use a channel sink, prod uses `AppSink` to emit Tauri events).
- `terminal/manager.rs` — `TerminalManager`: uuid → `Terminal` map, open/write/resize/close/list methods, explicit cleanup on drop.
- `terminal/mod.rs` — public types (`TerminalInfo`, `TerminalData`, `TerminalExit`), event constants (`terminal:data`, `terminal:exit`), error enum.
- `frontend/components/terminal/TerminalPanel.tsx` — collapsible bottom dock with tabs, xterm instances, event listeners for `terminal:data`/`terminal:exit`.

**Invariants & gotchas:**

- **Output is base64, input is UTF-8.** PTY output arrives as base64-encoded bytes on `terminal:data`; the frontend decodes to `Uint8Array` before xterm writes. Input keystrokes are raw UTF-8, written straight to the PTY master.
- **Reader owns the child.** The reader thread holds the `Box<dyn Child>` so it can reap the exit code after EOF. The `PtyProcess` only keeps a `ChildKiller` for explicit `kill()` calls.
- **Drop the slave early.** `pair.slave` is dropped after spawning the child, so the master reader sees EOF when the child exits (otherwise the master would hold a slave ref and never get EOF).
- **Explicit teardown, not best-effort.** `close(id)` kills the child and removes the terminal. `TerminalManager::drop` kills every child, forcing cleanup through a poisoned lock if needed — no orphan shells allowed.
- **Lock errors are named, not swallowed.** A poisoned mutex returns `TerminalError::LockPoisoned`, never an empty result that silently hides corruption.
- **Initial size is a placeholder.** PTY spawns at 80×24; the frontend issues `resize_terminal` as soon as the xterm view mounts and measures its container.
- **Shell resolution is strict precedence.** Explicit `shell` arg → login-shell `$SHELL` (from `forge_session::shell_env`) → process `$SHELL` → OS default (`/bin/zsh` on macOS, `/bin/sh` elsewhere). No guessing; fallback is named.
- **Terminal title = cwd basename.** The tab label is the cwd's final path component (or the full path if it has no components, e.g. `/`).
