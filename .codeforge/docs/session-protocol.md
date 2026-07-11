All edited files are in the Tauri frontend (TypeScript/UI) and the headless indexing module. None touch the session-protocol feature's core Rust files (protocol.rs, payload.rs, types.rs, mode.rs). The living doc remains accurate.

---
# Session Protocol

**Purpose**

Defines Rust types and NDJSON codec for bidirectional sidecar communication: stdin commands (query, approval, mode-switch, abort) and stdout event streams (AgentEvent variants). Handles mode inference from DB state and stamps it into the first query.

**How it works**

- `parse_sidecar_line` deserializes stdout NDJSON → `AgentEvent` enum (15 variants: content_delta, turn_started, tool_use_start, etc.); unparseable lines are skipped, never fatal.
- `augment_query_if_needed` intercepts query commands, consuming init params (cwd, model, permissionMode, mode) once for the first query, then stamping `SessionMode::ContinueInProcess` on every follow-up.
- `SessionMode` (Fresh | Resume{claude_session_id} | ContinueInProcess) is DB-authoritative — Rust decides the engagement mode once from `sessions.claude_session_id`, not sidecar-inferred retry logic.
- First query gets `mode: resume` + `resumeSessionId` or `mode: fresh`; all subsequent queries in the same sidecar get `mode: continue`.
- `AgentEventPayload` is the flat Tauri IPC shape (camelCase, skip_serializing_if none) mapped from `AgentEvent`; frontend demuxes by sessionId and switches on eventType.
- Supports 4 SDK permission modes (default, acceptEdits, plan, bypassPermissions) validated against a const allowlist.

**Key files**

- `protocol.rs` — NDJSON parsers (`parse_sidecar_line` → AgentEvent) and query augmentation (`augment_query_if_needed` stamps mode/init params into first query).
- `mode.rs` — SessionMode enum (Fresh, Resume, ContinueInProcess) and `wire()` → "fresh"|"resume"|"continue".
- `types.rs` — AgentEvent enum (15 tagged variants: content_delta, turn_started, tool_use_start, approval_required, etc.).
- `payload.rs` — AgentEventPayload flat struct for Tauri IPC; `from_event()` maps AgentEvent → camelCase shape, `persistence_degraded()` for Rust-originated errors.

**Invariants & gotchas**

- Init params are consumed exactly once on first query (Mutex<Option>); non-query stdin passes through without consuming the slot.
- Empty text_delta and thinking_delta are dropped in `parse_sidecar_line` to avoid no-op frontend renders.
- Unknown sidecar event types are debug-logged and skipped, never fatal — the SDK may evolve or print stray non-JSON.
- `SessionMode::Resume` failure → `SessionResumeFailed` event; never silent downgrade to Fresh.
- Permission mode strings must match one of 4 consts or are rejected at command time, not sidecar time.
- The wire `mode` field is distinct from `permissionMode` — one controls SDK engagement (fresh/resume/continue), the other controls approval policy.
