Now I have enough context. The doc's coverage is accurate but needs to reflect that the approval response flow now supports AskUserQuestion with answers that are plumbed through the entire chain and merged into updatedInput at the sidecar level. Let me write the updated doc:

---
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
- **Approval response flow**: `approve_session` command carries optional `answers` (for `AskUserQuestion`), plumbed through `manager.approve` → `claude.rs respond_to_approval` → sidecar `approval_response` → sidecar `canUseTool` resolve. The sidecar merges answers into the tool's `updatedInput` per the Agent SDK contract; approving a question without answers makes the tool report "The user did not answer the questions."

## Key files

- `crates/tauri-app/src/runtime/session_forward.rs` — core forwarder logic: event loop, persistence, degraded-state reporting.
- `crates/forge-session/src/payload.rs` — `AgentEventPayload` flat union structure, `from_event` mapper, camelCase serialization.
- `crates/forge-session/src/types.rs` — `AgentEvent` enum (sidecar out-event mirror).
- `crates/tauri-app/src/commands/sessions.rs` — `start_session` command that spawns the forwarder with thread row id; `approve_session` command carrying optional answers.
- `crates/forge-session/src/manager.rs` — `approve` method accepting `answers: Option<serde_json::Value>` for `AskUserQuestion` responses.
- `crates/forge-session/src/claude.rs` — `respond_to_approval` method writing `approval_response{requestId, decision, message?, answers?}` to sidecar stdin.
- `crates/tauri-app/agent-sidecar/index.mjs` — `canUseTool` callback: on allow, merges `resp.answers` into `updatedInput: {questions, answers}` for `AskUserQuestion`; non-question tools get `updatedInput: input` verbatim.
- `crates/tauri-app/frontend/src/components/session/blocks.tsx` — `QuestionCard` collects user selections, builds `answers` record (question text → label), calls `appStore.approveRequest(…, answers)`.

## Invariants & gotchas

- **CONTRACT-1**: every payload MUST be stamped with the forge `sessionId` so the frontend can demux by one id space.
- **CONTRACT-2**: the thread row exists by construction before the forwarder is spawned; persistence is unconditional, not opt-in.
- **APPROVAL-ANSWERS-CONTRACT**: when approving an `AskUserQuestion` request, the host MUST resolve with `updatedInput: {questions, answers}` where `answers` maps question TEXT (not index) → selected option LABEL (", "-joined for multiSelect). Resolving with no answers (or `updatedInput: input`) makes the Agent SDK tool return "The user did not answer the questions." The sidecar enforces this contract: it merges `resp.answers` into `updatedInput` before resolving the `canUseTool` promise, so the Rust side must send answers on allow for question requests.
- **DB writes are async-wrapped**: all SQLite writes use `spawn_blocking` to avoid holding the `Arc<Mutex<Database>>` across an `.await`.
- **Persistence failure is a named state**: never return `Ok` after a write fails — emit `session_persistence_degraded` and log an error.
- **Content buffer is cleared on abort/error**: `TurnAborted` and `SessionError` drop the buffered text to prevent incomplete messages from landing in the DB.
- **SDK thread id is seeded from hint**: before `SessionReady` confirms the real SDK session id, the forwarder uses the `resume_session_id` hint (if any) so early events carry a plausible `threadId`.
