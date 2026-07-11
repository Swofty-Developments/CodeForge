# Session Protocol

## Purpose

Defines the NDJSON event stream protocol between FeatureForge's Tauri backend and the Claude Agent SDK sidecar process. Each line of sidecar stdout is parsed into an `AgentEvent` and forwarded to the frontend as a flat `AgentEventPayload`.

## How it works

- **`AgentEvent` enum** (types.rs) — 14 tagged variants: `SessionReady`, `TurnStarted`, `ContentDelta`, `ThinkingDelta`, `ToolUseStart`, `ToolInputDelta`, `ToolUseEnd`, `ToolResult`, `ApprovalRequired`, `AskUserQuestion`, `TurnCompleted`, `TurnAborted`, `UsageReport`, `SessionError`, `SessionResumeFailed`, `SlashCommands`. Serde-tagged `snake_case` serialization.
- **`parse_sidecar_line`** (protocol.rs) — consumes one NDJSON line from sidecar stdout; unknown/malformed lines are silently skipped (the SDK/CLI may print non-JSON debug output). Empty deltas are dropped to avoid no-op renders. Maps `ready` (sidecar boot, no SDK session id yet) and `session_ready` (SDK init message with `sessionId` + `model`) to distinct `SessionReady` events with optional fields.
- **`augment_query_if_needed`** (protocol.rs) — intercepts outbound `{"type":"query"}` commands and stamps the engagement mode (`fresh` / `resume` / `continue`) + first-query-only `cwd`/`model`/`permissionMode` from `SidecarInitParams` (consumed exactly once). Follow-up queries in the same sidecar session get `mode: "continue"`. Explicit `model` / `permissionMode` on the command win over init defaults; `cwd` and `mode` are always stamped by the backend.
- **`AgentEventPayload::from_event`** (payload.rs) — flattens `AgentEvent` into a camelCase Tauri IPC payload with `eventType` discriminator, `sessionId`, `threadId`, and 15+ optional fields. The frontend demuxes by `sessionId` and switches on `eventType`.
- **`SessionMode`** (mode.rs) — 3-variant enum (`Fresh`, `Resume {claude_session_id}`, `ContinueInProcess`) decided once at query time from the DB-authoritative `sessions.claude_session_id`. Replaces old try-resume-catch-fresh cascades; the sidecar never infers resumability from error strings.
- **Per-message incremental diffing** (agent-sidecar/index.mjs) — the sidecar diffs assistant-message content against `streamBlockLens[i]` keyed by `(streamMsgId, block index)`. When a new assistant message id appears (each API round-trip between tool calls yields one), `streamMsgId` and `streamBlockLens` reset to zero; repeated snapshots of the same message emit only new chars. This prevents mid-turn reasoning/text from being sliced by the PREVIOUS message's cumulative length. Tool cards are deduped with a per-turn `streamToolIdsSeen` set.
- **`AskUserQuestion` event** (types.rs, protocol.rs, payload.rs) — native protocol support for the SDK's `ask_user_question` out-event. The raw `questions` array from the sidecar passes through as `serde_json::Value` (Rust) / `unknown` (TS), rendered by the frontend. `request_id` disambiguates concurrent questions.

## Key files

- **protocol.rs** — NDJSON parser (`parse_sidecar_line`), first-query augmentation (`augment_query_if_needed`), init-params injection.
- **types.rs** — `AgentEvent` enum (internal, serde-tagged `snake_case`).
- **payload.rs** — `AgentEventPayload` flat union (Tauri IPC, camelCase); `from_event` mapper; `persistence_degraded` Rust-originated event.
- **mode.rs** — `SessionMode` enum (Fresh/Resume/ContinueInProcess) and permission-mode validation (`PERMISSION_MODES`, `is_valid_permission_mode`).
- **agent-sidecar/index.mjs** — Node.js sidecar wrapping `@anthropic-ai/claude-agent-sdk`; per-message streaming-diff logic (lines 235–370).

## Invariants & gotchas

- **One-time init consumption**: `SidecarInitParams` in the `Mutex<Option<...>>` is `.take()`'d on the first `query`; re-running `augment_query_if_needed` on subsequent queries in the same sidecar yields `mode: "continue"` with no `cwd`/`model`/`resumeSessionId`.
- **Non-query commands pass through**: `augment_query_if_needed` only touches `{"type":"query"}`; `abort` and other control messages are returned unchanged without consuming the init slot.
- **Silent skip, never fatal**: `parse_sidecar_line` drops unparseable JSON, unknown event types, empty deltas, and non-object values (the SDK may print warnings/logs). Never propagate unknown events — the frontend switches on a closed set.
- **Mode is DB-authoritative**: `SessionMode` is decided once from `sessions.claude_session_id` at session start. The sidecar sets `options.resume` / `options.continue` / neither from the stamped `mode` field; it never infers resumability from prior state or error strings.
- **Resume failure is explicit**: `SessionResumeFailed` is a first-class event; never silently downgrade to a fresh session if `Resume` is requested.
- **`AgentEventPayload.message` is overloaded**: carries `claude_session_id` for `SessionReady` / `SessionResumeFailed`, error text for `SessionError`, generic message for `persistence_degraded`. The frontend disambiguates via `eventType`.
- **Streaming diff is per-message**: a single agentic turn yields MANY assistant messages (one per API round-trip). Before the per-message fix, global `lastTextLen`/`lastThinkingLen` counters truncated every post-tool-call message by the PREVIOUS message's length, emitting nothing (dropped as empty in protocol.rs) or corrupted tails. The UI showed only the shimmer typing indicator. Now `streamMsgId` / `streamBlockLens` reset when a new message id appears; duplicate tool cards are guarded by `streamToolIdsSeen`.
- **ThinkingBlock shows live tail**: while streaming, `ThinkingBlock` shows a 100-char preview of the latest reasoning instead of hiding content behind animated dots. When done, it collapses to an expandable disclosure showing the opening thought.
- **`questions` field is raw JSON**: the `AskUserQuestion` event's `questions` payload is an untyped `Value` / `unknown` — the SDK owns its shape. The frontend is responsible for rendering it (likely iterating over an array of `{question: string, ...}` objects).
