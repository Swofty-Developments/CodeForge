# Terminal PTY Manager

## Purpose

Manages embedded pseudo-terminal instances via `portable_pty`. Spawns shell processes in isolated PTYs, streams their output to the Tauri frontend as base64-encoded chunks, and handles write, resize, kill, and exit lifecycle.

## How it works

- **Spawn**: `TerminalManager::open` allocates a PTY pair, spawns the requested shell (resolved from arg → login `$SHELL` → process `$SHELL` → OS default `/bin/zsh`), drops the slave handle so only the child owns it, then clones a reader and spins up a blocking reader thread.
- **Reader thread**: blocking loop over the PTY master reader; each chunk is base64-encoded and emitted on `terminal:data`, EOF triggers `child.wait()` to reap the exit code, then `terminal:exit` fires once and the thread ends.
- **Input**: frontend sends raw UTF-8 keystroke bytes to `write_terminal`; the manager writes them straight to the PTY master's writer handle.
- **Resize**: frontend calls `resize_terminal` when the xterm view changes geometry; PTY master issues SIGWINCH to the child shell.
- **Kill**: `close_terminal` removes the terminal from the map and calls `killer.kill()`; the reader thread hits EOF, reaps the exit code, emits `terminal:exit`, and ends.
- **Teardown**: `TerminalManager::Drop` kills every remaining child (even through a poisoned lock) so closing the app leaves no orphan shells.

## Key files

- **`crates/tauri-app/src/terminal/mod.rs`** — module root, event constants, payload types, error enum.
- **`crates/tauri-app/src/terminal/manager.rs`** — `TerminalManager` that owns the `HashMap<String, Terminal>` and bridges PTY output to Tauri events via `AppSink`. Explicit teardown on drop.
- **`crates/tauri-app/src/terminal/pty.rs`** — `PtyProcess` wrapper around `portable_pty`. Spawns the shell, clones reader/writer/killer handles, base64-encodes output chunks in a blocking reader thread, and reaps the child exit code.

## Invariants & gotchas

- **Output is base64-encoded** at the Rust layer; the frontend xterm decoder expects it. Input keystrokes are raw UTF-8 and written directly to the PTY master.
- **Slave handle must be dropped** after spawning the child — if the master side holds a copy, the reader never sees EOF when the child exits.
- **Reader thread owns the `Child`** so it can reap the exit code after EOF. The manager holds only the killer handle.
- **Exit code `None` is named "unknown"** (couldn't reap the child), NOT a silent zero.
- **Drop kills every child** even through a poisoned lock to prevent orphan shells when the app closes.
- **Initial geometry (80×24)** is a placeholder; the frontend issues a real resize once the xterm view mounts.
