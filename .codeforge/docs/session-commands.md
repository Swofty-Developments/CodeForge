# Session IPC Commands

## Purpose

Tauri command layer that bridges the Electron-style frontend to the `SessionManager` lifecycle — start/stop sessions, send user prompts, approve permissions, switch modes, and list running sessions. Every session is anchored to an open repo and persisted as thread + session rows (CONTRACT-2).

## How it works

- **start_session**: Checks repo is open (CONTRACT-2 precondition), resolves `SessionMode::Fresh` or `Resume{claude_session_id}` from DB, starts the session via `SessionManager`, writes `thread` + `session` rows atomically (tears down the session if persistence fails), spawns the event forwarder that re-emits `AgentEvent`s on the Tauri `agent-event` channel, and tracks session → repo mapping.
- **send_session_input**: Queues a user prompt on a running session's sidecar stdin.
- **approve_session**: Answers a pending `approval_required` event (permission prompt) with approve/deny.
- **stop_session**: Kills the session's sidecar process and forgets its repo mapping (called on explicit user stop or when the repo closes).
- **list_sessions**: Returns a snapshot of all live sessions for the session pane.
- **set_session_mode**: Switches a running session's permission mode mid-session (`default|acceptEdits|plan|bypassPermissions`) — applied by the sidecar on its next turn.

## Key files

- **crates/tauri-app/src/commands/sessions.rs** — all six Tauri commands and the `resolve_session_mode` / `create_thread_and_session` helpers.
- **crates/forge-session/src/manager.rs** — the `SessionManager` that owns the sidecar processes and exposes `start_session`, `send`, `approve`, `stop`, `set_mode`, `list`.
- **crates/tauri-app/src/state.rs** — `AppState.sessions` (the manager) and `AppState.session_repos` (session → repo root mapping for teardown on close).

## Invariants & gotchas

- **CONTRACT-2 precondition**: `start_session` requires the repo to be open — otherwise there is no `repos.id` to anchor the thread row, so it errors with `"repo is not open"` before starting the session.
- **Atomic persistence or teardown**: if `create_thread_and_session` fails after the session has started, the session is torn down immediately rather than run headless (lines 51–56) — a session must have thread + session rows by construction.
- **Resume target must be recorded**: a resume `claude_session_id` must exist in the `sessions` table — an unrecorded id is a named error, never a silent fresh start (lines 83–87).
- **Session cleanup on repo close**: when a repo closes, all sessions rooted at that repo must be torn down (enforced by `close_repo` reading `session_repos`).
- **DB mutex locking**: `state.db` is a `std::sync::Mutex` — all DB writes from async commands go through `spawn_blocking` to avoid holding the mutex across `.await` (lines 93–99, 180–187).
- **Permission mode validation**: `set_session_mode` validates `mode` against the known set (`default|acceptEdits|plan|bypassPermissions`) in the manager — an unknown value is an error, not a no-op.
