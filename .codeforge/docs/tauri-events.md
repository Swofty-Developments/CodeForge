# Tauri Events

## Purpose

Event name constants for frontend-bound emissions so event names stay in sync across Rust and TypeScript. The constants are the frozen contract between the Tauri backend (`crates/tauri-app/src`) and the frontend (`crates/tauri-app/frontend/src`). Naming typos surface at compile time on the Rust side and are mirrored in `ipc.ts` for type-safe listeners on the frontend.

## How it works

`events.rs` exports six event channels:

- **`AGENT_EVENT`** (`"agent-event"`) — all session streaming (`AgentEventPayload`). The backend emits every turn's text deltas, tool uses, thinking, approval requests, questions, and turn completion. The frontend demuxes by `sessionId`. Includes subtypes:
  - `approval_request` — tool needs permission (regular tools)
  - `ask_user_question` — `AskUserQuestion` tool fired (questions rendered as cards, approval must carry `answers: {question_text → selected_label}` or the Agent SDK reports "The user did not answer the questions.")
  - `text_delta`, `thinking_delta`, `tool_use_start`, `tool_use_input`, `tool_result`, `turn_started`, `turn_completed`, `usage`
- **`TIMELINE_EVENT`** (`"timeline:event"`) — live timeline appends (`TimelineEvent`). Fired after each commit, note, or feature update. The frontend subscribes per-repo and updates the timeline view.
- **`INDEX_PROGRESS`** (`"index:progress"`) — indexing runtime progress (`IndexProgress`). A bounded stream of file-count / feature-count deltas while the index rebuilds.
- **`REPO_CHANGED`** (`"repo:changed"`) — repo opened/closed/re-indexed (`RepoState`). Triggers the frontend to refresh the sidebar feature tree.
- **`session_status_event(id)`** (dynamic) — per-session status updates (session info, model, phase). A `fn` returning `format!("session:{session_id}:status")`. Used for granular subscriptions when a component only cares about one session's lifecycle.

The frontend mirrors these constants in `ipc.ts` as `listen*` helpers (`listenAgentEvent`, `listenTimelineEvent`, etc.) so event subscription is typed.

## Key files

- **`events.rs`** (core) — event name constants and the `session_status_event(id)` formatter. The single source of truth. Imported by every backend module that emits events.
- **`frontend/src/ipc.ts`** — TypeScript mirror of the event names, plus typed `listen` wrappers. The frontend's single source of truth. Imported by every React/Solid component that subscribes to events.
- **`agent-sidecar/index.mjs`** — emits `approval_request` and `ask_user_question` events. The sidecar's `canUseTool` callback sends these to stdout; Rust parses them and re-emits as `AGENT_EVENT` payloads.

## Invariants & gotchas

- **Frozen contract.** Event names in `events.rs` are the definitive list. Changing a name breaks the frontend unless `ipc.ts` is updated in the same commit.
- **`AGENT_EVENT` is the session event bus.** Every session-scoped event — text, tools, approvals, questions — flows through this single channel. The frontend demuxes by `sessionId` in the payload.
- **`ask_user_question` approval requires `answers`.** When approving an `AskUserQuestion` request, the host (Tauri `approve_session` command) **must** send `answers: {question_text → selected_label}` alongside `decision: "allow"`. The sidecar merges `answers` into `updatedInput` per the Agent SDK contract (line 221–224, `index.mjs`). If `answers` is omitted, the SDK resolves the tool with "The user did not answer the questions." This was the root cause of the AskUserQuestion bug: the frontend sent only `{decision: "allow"}` with no answer map, so the sidecar resolved with `updatedInput: {questions, answers: undefined}` and Claude saw an empty response.
- **Session status is a dynamic channel.** Unlike the other five constants, `session_status_event(id)` generates a unique channel per session. Subscribing to one session's status does not leak events from another.
- **Event payloads are JSON-serialized at the Tauri boundary.** Rust emits `serde_json::Value`; the frontend receives typed objects. If a payload field changes shape, both sides must update.
