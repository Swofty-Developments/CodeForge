The edit added `indexStatusByPath: Record<string, IndexStatus>` tracking at line 84, and the `handleIndexStatus` event consumer that updates this record (line 183). This is *not* part of the session-slice feature — it's part of the index-staleness detection feature. The session-slice entry points and key files remain unchanged; the only file touched was `app-store.ts`, which is the global store but not listed as a key file for session-slice.

The living doc is still accurate. No update needed.

---
# Session Slice

## Purpose

Manages the full lifecycle of Claude agent sessions: creation, message exchange, live permission-mode switching, abort vs. destructive stop, approval flow, and ingestion of streaming `agent-event` payloads into a normalized message/block structure.

## How it works

- `startSession` creates a new `SessionUi` record tagged with the active context path (worktree) and permission mode, then stores it in `AppStore.sessions`.
- `sendMessage` appends a user message, finalizes any live assistant message from a prior turn, sets `runState = "generating"`, and calls the backend IPC.
- `handleAgentEvent` demuxes incoming events by `sessionId`, mutates the matching session record via Solid's `produce`, and applies deltas to the live assistant message (the last assistant message without a `done-` prefix).
- Tool blocks are appended on `tool_use_start`, deltas accumulate in `toolInput`, and `tool_result` stamps the final output and status (`completed` | `error`).
- `turn_completed` finalizes the live assistant message (stamps it with a `done-` id, rolls up text blocks into `.content`, closes any running tool blocks), then flips `runState` to `"ready"`.
- `turn_aborted` finalizes the live assistant with `"error"` outcome, pushes a system warning, and resets runState — triggered by `interruptSession` (non-destructive stop).
- `setSessionMode` allows live permission-mode switching (W5) with optimistic update and rollback on IPC failure.

## Key files

- **session-slice.ts** — action creators (`startSession`, `sendMessage`, `approveRequest`, `interruptSession`, `stopSession`, `setSessionMode`) and the composer prefill flow.
- **session-reducer.ts** — the `handleAgentEvent` reducer that demuxes 16 event types into message/block mutations; shared helpers `ensureLiveAssistant`, `finalizeLiveAssistant`, `findToolBlock`.

## Invariants & gotchas

- **Live message identity**: a message is *live* iff `role === "assistant" && !id.startsWith("done-")`. Only one live assistant message exists at any moment. Finalization stamps a `done-` id to freeze it.
- **Routing contract**: every `AgentEventPayload` carries `sessionId`; the reducer routes strictly by this key (CONTRACT-1). Events for unknown sessions are silently dropped.
- **Tool block lookups**: `findToolBlock(s, toolId)` scans backwards across all messages. An empty `toolId` matches the most recent tool block regardless of id. Tool blocks live on the assistant message, never user messages.
- **Mid-turn steering**: sending a new message while `runState === "generating"` finalizes the live message with outcome `"completed"` before appending the new user turn. This handles prompt-chaining and `/stop` + immediate resend.
- **Message content hygiene**: `msg.content` is the rolled-up concatenation of text blocks, computed on finalization. Do not mutate `.content` directly during streaming — only append to blocks.
- **Permission mode is live**: `setSessionMode` mutates a running session's mode without restarting. The backend IPC applies it immediately. Optimistic update reverts on failure.
- **No block mirror**: `messages` is the single source of truth. Blocks are nested in `msg.blocks`, not mirrored in a parallel structure.
- **Two stop paths**: `interruptSession` (composer stop button) aborts the in-flight turn via a `{"type":"abort"}` message to the sidecar → `turn_aborted` event → the transcript stays, runState resets to ready. `stopSession` (tab close button ✕) kills the sidecar and filters the session from the store, deleting the whole transcript. The UI's stop button is non-destructive; only tab close is destructive.
