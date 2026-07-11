# Session Slice

## Purpose

Manages embedded Claude sessions in the Tauri frontend: starting, stopping, sending user input, approving/denying tool and question requests, renaming, and live permission-mode switching. Listens to `agent-event` emissions from the backend and updates the session UI state in real time.

## How it works

- **Session lifecycle**: `startSession` spawns a new sidecar-backed session and pushes a `SessionUi` record into `store.sessions`; `stopSession` kills the sidecar and filters it out. `resumeSession` spawns a fresh session that continues a past Claude session ID.
- **Live mode switching**: `setSessionMode` optimistically updates the permission mode in the UI and calls `ipc.setSessionMode` to push the change to the sidecar, rolling back on failure.
- **Message streaming**: The `handleAgentEvent` reducer (in `session-reducer.ts`) routes backend events by `payload.sessionId` (CONTRACT-1) and builds the transcript incrementally — `content_delta` appends text, `tool_use_start` pushes a tool block, `tool_result` marks the block complete, and `turn_completed` finalizes the live assistant message by prefixing its ID with `done-`.
- **Tool approval**: `approveRequest` answers both approval prompts and AskUserQuestion requests, clearing the pending state after the backend ACK. For questions, the `answers` map (question text → selected option) is required on approve.
- **Interrupt vs stop**: `interruptSession` aborts the in-flight turn but keeps the session alive; `stopSession` kills the sidecar and removes the entire session from the UI.
- **Composer prefill**: `prefillComposer` seeds the composer input and reveals the session pane — used for the "Ask Claude" flow from other panels.

## Key files

- **`session-slice.ts`** — exports `createSessionSlice`, which provides the actions: `startSession`, `sendMessage`, `approveRequest`, `setSessionMode`, `stopSession`, `renameSession`, `resumeSession`, `prefillComposer`, and the event handler.
- **`session-reducer.ts`** — the `handleAgentEvent` reducer that demuxes backend events and mutates `SessionUi.messages` plus run state; also exports message helpers (`findLiveAssistant`, `ensureLiveAssistant`, `finalizeLiveAssistant`, `pushSystemMessage`) used by both the reducer and the actions.

## Invariants & gotchas

- **Routing by sessionId is mandatory**: every backend event MUST carry `payload.sessionId` (CONTRACT-1); the reducer routes by exact match and silently drops events for unknown sessions.
- **Live assistant message is the streaming target**: the last assistant message without a `done-` ID prefix is the one receiving deltas. `finalizeLiveAssistant` locks it by prefixing `done-` and marking all in-flight tool blocks completed or errored.
- **`messages` is the single source of truth**: there is no parallel block mirror; all streaming state lives in `SessionUi.messages`.
- **Optimistic UI for mode + rename**: `setSessionMode` and `renameSession` update the UI first, then call IPC, rolling back on failure. This keeps the UI snappy but requires the `prev` value to be captured before mutation.
- **Empty toolId in `findToolBlock` matches the most recent tool block**: used when the backend event doesn't stamp a toolId (e.g., old Claude SDK versions). Tool blocks scan backwards from the last message.
- **Mid-turn steering closes the live message**: `sendMessage` calls `finalizeLiveAssistant` before pushing the new user message so the transcript stays coherent when the user interrupts a generating turn.
- **`claudeSessionId` is stamped once**: the first `turn_completed` or `session_ready` event that carries a turnId/message sets `s.claudeSessionId`; later events do not overwrite it. This ID is used for resuming past sessions.
- **Resume failure is surfaced explicitly**: `session_resume_failed` pushes an error-level system message and sets `runState: "error"` rather than silently falling back to a fresh session.
