# Session Event Forwarding

## Purpose

Subscribes to a per-session `AgentEvent` stream from the session manager and forwards each event to the Tauri frontend via the `agent-event` IPC channel while durably persisting assistant messages, usage logs, and Claude session (resume) IDs to SQLite.

## How it works

- Every spawned session gets one `spawn_forwarder` task that drains a `tokio::mpsc::Receiver<AgentEvent>` in a loop until the session dies.
- Each event is stamped with the **session-manager ID** (`sessionId`) before emission so the frontend can demux by this single routing key; `threadId` carries the Claude SDK session uuid for display/resume only.
- Assistant text is buffered from `ContentDelta` events and flushed to the `messages` table on `TurnCompleted`; `TurnAborted` and `SessionError` clear the buffer without persisting.
- Usage logs (tokens, cost, model) are written to the `usage_logs` table on every `UsageReport` event; the Claude resume ID is written to `sessions.claude_session_id` when `SessionReady` arrives with a uuid.
- All DB writes run via `spawn_blocking` to avoid holding the `Arc<Mutex<Database>>` across an `.await`; a persistence failure emits a `session_persistence_degraded` event and logs an error instead of silently swallowing it.
- The forwarder ends when the session manager drops the sender (the `rx.recv()` loop exits on `None`).

## Key files

- `crates/tauri-app/src/runtime/session_forward.rs` — forwarder loop: event routing, buffering, persistence, and degraded-state reporting.

## Invariants & gotchas

- **CONTRACT-1**: every emitted payload carries the session-manager ID as `sessionId`; the frontend multiplexes by this ID space.
- **CONTRACT-2**: the `threads.id` row for `app_thread_id` MUST exist before `spawn_forwarder` is called — persistence writes are unconditional; a missing row is a fatal logic error upstream.
- **No silent failures**: if any DB write fails (messages, usage, resume id), the forwarder MUST emit a `session_persistence_degraded` event and log an error — the failure is surfaced state, not a warning.
- **DB mutex discipline**: never hold the `Arc<Mutex<Database>>` guard across an `.await` — all writes go through `spawn_blocking` so the mutex is always unlocked during async suspension.
- **Buffer flushing**: only `TurnCompleted` persists the accumulated assistant text; `TurnAborted` / `SessionError` must clear the buffer without writing (a failed turn is not durable).
- **Resume ID timing**: the `sdk_thread_hint` seeds the `threadId` field until `SessionReady` confirms the real Claude session uuid; once confirmed, the resume ID is persisted to `sessions.claude_session_id`.
