# Session Event Forwarding

## Purpose

Bridges the `forge-session` sidecar's `AgentEvent` stream to the Tauri frontend via the single `agent-event` channel, stamping every payload with the session ID for frontend demux and persisting durable side-effects (messages, usage, resume ID) to the app SQLite database.

## How it works

- Spawns one Tokio task per session that consumes the `AgentEvent` rx channel and emits a flat `AgentEventPayload` to the `agent-event` Tauri channel for each event.
- Buffers `ContentDelta` fragments until `TurnCompleted`, then persists the full assistant message to the `messages` table.
- Writes `UsageReport` events to `usage_log` and `SessionReady` resume IDs to `sessions.claude_session_id` via `spawn_blocking` to avoid holding the `Arc<Mutex<Database>>` across async awaits.
- On any DB write failure, emits a `session_persistence_degraded` event (visible to the UI) and logs an error — persistence failures are never swallowed.
- Stamps every payload with the forge `sessionId` (the session-manager UUID / IPC routing key) plus the Claude SDK `threadId` (seeded from resume hint, then updated once `SessionReady` confirms the real one).
- Aborts (`TurnAborted` / `SessionError`) clear the assistant text buffer without persisting.

## Key files

- `crates/tauri-app/src/runtime/session_forward.rs` — core forwarder: event loop, payload emission, DB persistence calls, degraded-state reporting.
- `crates/forge-session/src/payload.rs` — flat `AgentEventPayload` struct (camelCase, optional fields) and `from_event` / `persistence_degraded` constructors.
- `crates/tauri-app/src/events.rs` — frozen Tauri event channel names (`AGENT_EVENT = "agent-event"`), mirrored in `frontend/src/ipc.ts`.

## Invariants & gotchas

- **CONTRACT-1**: every payload must carry the forge `sessionId` so the frontend can demux a shared channel; `threadId` is display-only / resume-only, not a routing key.
- **CONTRACT-2**: the thread row must exist before the forwarder spawns (caller's responsibility); all writes are unconditional — a missing row is a panic-worthy invariant violation, not a graceful skip.
- DB writes must go through `spawn_blocking` + `tokio::spawn_blocking` — never lock the `Arc<Mutex<Database>>` across an async `.await` or the runtime stalls.
- Persistence failures are a **named, surfaced state** (`session_persistence_degraded` event + error log), never a benign warning — the UI must know the session's stored history is incomplete.
- The assistant text buffer is cleared on abort/error — partial turns are never persisted, only complete `TurnCompleted` messages.
- `sessionId` stamping happens before emission (`AgentEventPayload::from_event` takes it as a parameter); a missing stamp would route events to the wrong session or drop them entirely.
