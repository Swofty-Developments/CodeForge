# Session Lifecycle

## Purpose

Spawns and manages one Node.js sidecar process per Claude Code session, bridging the Rust backend with the Agent SDK via an NDJSON-over-stdio protocol. Handles process lifecycle (spawn, kill-on-drop), stdin command queueing, stdout event parsing, stderr crash detection, login-shell environment resolution, and graceful shutdown with abort.

## How it works

- **Process spawn**: Resolves `node` from the user's login-shell PATH (via `shell_env::which` and `shell_env::apply`), finds the sidecar script (`agent-sidecar/index.mjs`) by walking up from the executable or in bundled app resource directories, spawns with `kill_on_drop(true)` and piped stdio.
- **Four concurrent tasks**: An augmenter task splices init params (`cwd`, `model`, `permissionMode`, `mode`) into the first query only; a writer task drains the stdin channel and writes NDJSON commands (`query`, `abort`, `approval_response`, `set_mode`); a stdout reader parses NDJSON events and forwards them to the caller; a stderr collector buffers the last 50 lines and emits a terminal `SessionError` on EOF (sidecar crash or unexpected exit).
- **Channels**: Stdin commands queue in a 256-slot bounded channel; events flow to the caller via a 1024-slot channel. Backpressure drops stdin writes if the sidecar lags; callers must size their event channel to never block readers (blocking a reader blocks all other tasks).
- **Session ID capture**: The stdout reader watches for `session_ready` and stores the Claude Agent SDK session ID in a `OnceLock`, exposed via `claude_session_id()` for resume flows.
- **Graceful shutdown**: `stop()` aborts all tasks first (so the stderr collector knows the exit is intentional and doesn't report a crash), then kills the child process; task abort order guarantees stderr EOF never races with a stop.

## Key files

- **crates/forge-session/src/claude.rs** — `ClaudeSession` struct, spawn/stop/send/interrupt/respond methods, the four tokio tasks (augmenter, stdin writer, stdout reader, stderr collector), and the NDJSON protocol plumbing.
- **crates/forge-session/src/shell_env.rs** — Resolves the user's login-shell environment once per process (`$SHELL -l -i -c 'env -0'`) to surface nvm/fnm `node` in desktop-launched apps; a fallback probe (`$SHELL -l -c 'env'`) runs if the interactive probe fails; both failing yields a flagged `Unresolved` state (not silently passed as resolved).
- **crates/forge-session/src/locate.rs** — Resolves the sidecar script in priority order: macOS `Contents/Resources/agent-sidecar/`, beside the exe (Linux/Windows), walking up from the exe/cwd (dev builds, ≤10 parents), compile-time fallback.

## Invariants & gotchas

- **Stderr EOF is always abnormal**: The stderr collector assumes any sidecar exit is a crash (emits `SessionError`) unless `stop()` aborted the tasks first. Never kill the child before aborting tasks or a spurious crash event fires.
- **Init params are spliced once**: The augmenter intercepts only the first `query` message and merges `cwd`/`model`/`permissionMode`/`mode` into it; all subsequent queries pass through unmodified. Sending a raw first query without queueing it through `send_message` breaks the protocol.
- **Unresolved shell-env is a named state**: If both shell probes fail, `ShellEnv::Unresolved` is returned (not an error), but `node_not_found_error` explains the PATH search was limited; callers must check `is_resolved()` to diagnose `node` misses.
- **Resume mode omits permission_mode**: `ClaudeSession::resume` intentionally passes `None` for `permission_mode` (CodeForge parity); permission state is session-specific and not carried across resume.
