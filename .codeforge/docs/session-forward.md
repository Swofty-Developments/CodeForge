# Session Event Forwarding

## Purpose

Bridges the session manager's internal event stream to the frontend by forwarding `AgentEvent` messages as Tauri events, persisting assistant content, usage metrics, and resume state to SQLite along the way.

## How it works

- **Spawned per session**: one forwarder task per `start_session` call, consuming the session's `mpsc::Receiver<AgentEvent>`.
- **Content buffering**: accumulates `ContentDelta` text until `TurnCompleted` fires, then persists the full assistant message to the DB.
- **Resume state tracking**: when `SessionReady` arrives with a Claude session UUID, updates the `sessions.claude_session_id` column for future resume.
- **Usage logging**: `UsageReport` events write token counts and cost to the `usage_log` table, tagged by thread and session.
- **Degraded-state signaling**: any DB write failure emits a `session_persistence_degraded` event to the frontend and logs an error — never silently swallowed.
- **Payload stamping**: every forwarded event is stamped with the forge `sessionId` (routing key) and `threadId` (SDK resume id) before emission on the single `agent-event` channel.

## Key files

- `crates/tauri-app/src/runtime/session_forward.rs` — core forwarder logic: event loop, persistence, degraded-state reporting.
- `crates/forge-session/src/payload.rs` — `AgentEventPayload` flat union structure, `from_event` mapper, camelCase serialization.
- `crates/forge-session/src/types.rs` — `AgentEvent` enum (sidecar out-event mirror).
- `crates/tauri-app/src/commands/sessions.rs` — `start_session` command that spawns the forwarder with thread row id.

## Invariants & gotchas

- **CONTRACT-1**: every payload MUST be stamped with the forge `sessionId` so the frontend can demux by one id space.
- **CONTRACT-2**: the thread row exists by construction before the forwarder is spawned; persistence is unconditional, not opt-in.
- **DB writes are async-wrapped**: all SQLite writes use `spawn_blocking` to avoid holding the `Arc<Mutex<Database>>` across an `.await`.
- **Persistence failure is a named state**: never return `Ok` after a write fails — emit `session_persistence_degraded` and log an error.
- **Content buffer is cleared on abort/error**: `TurnAborted` and `SessionError` drop the buffered text to prevent incomplete messages from landing in the DB.
- **SDK thread id is seeded from hint**: before `SessionReady` confirms the real SDK session id, the forwarder uses the `resume_session_id` hint (if any) so early events carry a plausible `threadId`.
