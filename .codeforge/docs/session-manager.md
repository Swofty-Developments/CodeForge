# Session Manager

## Purpose

Multi-session registry that spawns and manages independent ClaudeSession processes (Node agent-sidecar wrapping the Claude Agent SDK). Decides engagement mode (Fresh/Resume/Continue) from the app database and forwards events to the frontend without hidden fallback behavior.

## How it works

- Each `start_session` spawns a fresh sidecar process with an explicit `SessionMode` from the caller — no hidden resume→fresh downgrades; failures surface as `session_resume_failed` events.
- Generates sequential v4 UUIDs for session ids (distinct from the SDK's claude_session_id); frontend tracks turn state via events, manager only distinguishes Starting/Ready.
- Permission mode is intentionally *not* carried across resume (CodeForge parity); mid-session escalation via `set_mode` validates the mode string before lookup (named error, not silent no-op).
- Commands (`send`, `approve`, `abort`, `set_mode`) forward to the sidecar NDJSON protocol; `stop` removes the session and kills the process.
- Each session gets a `SessionEntry` with the `ClaudeSession`, display metadata (title, model), and a thread_hint superseded by the sidecar-confirmed id once `session_ready` arrives.

## Key files

- `crates/forge-session/src/manager.rs` — registry HashMap and all public manager commands (start/send/approve/set_mode/abort/stop/list)
- `crates/forge-session/src/claude.rs` — ClaudeSession: sidecar spawn, 4 tokio tasks (stdin writer / augmenter / stdout NDJSON parser / stderr collector), protocol mechanics
- `crates/forge-session/src/mode.rs` — SessionMode (Fresh/Resume/Continue) + permission mode validation (default/acceptEdits/plan/bypassPermissions)
- `crates/forge-session/src/lib.rs` — crate root, Error types, exports

## Invariants & gotchas

- **IDs are distinct:** FeatureForge session id (uuid v4, manager registry key) ≠ claude_session_id (SDK session id, captured from `session_ready`). Frontend must distinguish them.
- **No silent mode fallback:** if resume fails, the sidecar emits `session_resume_failed` to the event stream; the manager never silently downgrades Resume→Fresh. Caller interprets failure events.
- **Permission mode doesn't resume:** `StartSessionOpts.permission_mode` is dropped on Resume (CodeForge parity). Use `set_mode` post-handshake if needed.
- **set_mode validates first:** unknown modes are rejected with `InvalidMode` *before* the session lookup, not a silent no-op or generic NotFound.
- **Session status is two-state:** Starting (pre-handshake) or Ready (post-`session_ready`). Turn-level state (generating/error) lives frontend-side via event stream; `list()` snapshots never expose it.
- **Stderr EOF = crash:** the stderr collector always sends `SessionError` on EOF; `stop()` aborts tasks first to prevent a normal shutdown from masquerading as a crash.
