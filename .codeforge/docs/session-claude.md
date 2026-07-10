# Claude Session Process (session-claude)

## Purpose

Spawns and manages a single Node.js sidecar process (`agent-sidecar/index.mjs`) that wraps the Claude Agent SDK, parsing its NDJSON output into typed `AgentEvent` enums and forwarding them to the Tauri UI via an mpsc channel.

## How it works

- **Spawn with login-shell PATH**: resolves `node` via `shell_env::which()` (probes the login shell to find nvm/fnm-installed Node), spawns the sidecar script located via bundled-app paths or dev-workspace walk-up, and pipes stdin/stdout/stderr with `kill_on_drop(true)`.
- **Four tokio tasks per session**: (1) augmenter — splices init params (`cwd`, `model`, `permission_mode`, `mode`) into the first query only; (2) stdin writer — flushes NDJSON commands line-by-line; (3) stdout NDJSON parser — deserializes each line into `AgentEvent` and forwards to the UI; (4) stderr collector — buffers the last 50 lines, emits `SessionError` on EOF (sidecar exit = crash).
- **Session modes as first-class protocol**: Rust decides `Fresh` / `Resume{id}` / `ContinueInProcess` from the DB and stamps it into every `query` message; the sidecar never infers resumability from SDK errors or prior state.
- **Claude session ID captured on first `session_ready`**: stored in a `OnceLock` and returned via `claude_session_id()` once observed (used to persist resumable sessions to the DB).
- **Bidirectional commands**: `send_message(prompt)` → `{"type":"query","prompt":text}`, `set_mode(mode)` → `{"type":"set_mode","mode":mode}`, `interrupt()` → `{"type":"abort"}`, `respond_to_approval(id, approve, msg)` → `{"type":"approval_response", ...}`.
- **Named `node` miss errors**: if `shell_env::which("node")` fails, the error explains whether login-shell resolution worked (install Node) or failed (fix shell startup files) — never a bare ENOENT from spawning an absent `node`.

## Key files

- `crates/forge-session/src/claude.rs` — core: spawns the sidecar, manages four tokio tasks, exposes `send_message` / `set_mode` / `interrupt` / `respond_to_approval` / `stop`.
- `crates/forge-session/src/mode.rs` — defines `SessionMode` (Fresh / Resume / ContinueInProcess) and the four SDK permission modes, replacing the old try-resume-catch-fresh cascade.
- `crates/forge-session/src/locate.rs` — resolves `agent-sidecar/index.mjs` from bundled app paths (macOS `Resources/`, Linux beside exe) or dev workspace walk-up (≤10 parents).
- `crates/forge-session/src/shell_env.rs` — probes the user's login shell (`$SHELL -l -i -c 'env -0'`) once per process to find nvm/fnm `node`; desktop apps otherwise get a minimal PATH.

## Invariants & gotchas

- **Init params spliced once**: the augmenter task injects `cwd`, `model`, `permission_mode`, and `mode` into the first `query` only (held in an `Arc<Mutex<Option<_>>>` that is `take()`n). Subsequent queries carry only `prompt` and `mode`.
- **Stderr EOF = crash**: the stderr task always emits `SessionError` on EOF because `stop()` aborts all tasks before killing the process; if stderr closes while tasks are live, the sidecar exited unexpectedly.
- **Never spawn node without a named miss**: `shell_env::which("node")` returns `None` → a `NodeNotFound` error explaining why (login-shell unresolved vs. node genuinely absent). Never fall back to bare `Command::new("node")`, which fails later with a confusing ENOENT.
- **Resume ID vs. permission mode**: `ClaudeSession::resume()` intentionally does NOT accept a `permission_mode` argument (CodeForge parity — permission modes are session-start only; resume inherits the resumed session's mode).
- **Channel capacities are fixed**: stdin 256, events 1024 caller-side. A full stdin channel → `try_send()` error = sidecar backpressure or hung; a full event channel → stdout task blocks until the UI drains it.
