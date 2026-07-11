---
# Claude Session Sidecar

## Purpose

Wraps `@anthropic-ai/claude-agent-sdk`'s `query()` in a Node.js process that speaks NDJSON over stdin/stdout, letting the Rust backend (`forge-session`) drive embedded Claude sessions without shipping Node inside the Tauri app bundle.

## How it works

- Reads NDJSON commands on stdin: `query` (start a turn), `approval_response` (resolve a pending tool-permission request), `set_mode` (change permission mode for the next turn), `abort` (cancel current query).
- Calls `query({ prompt, options })` with the Agent SDK, diffing incremental snapshots to emit only new text/thinking deltas.
- Streams events to stdout: `text_delta`, `thinking_delta`, `tool_use_start`, `tool_use_input`, `tool_result`, `approval_request`, `ask_user_question`, `session_ready`, `turn_completed`, `usage`.
- Manages session state: captured `sessionId` from SDK init, `sessionPermissionMode` applied to every turn, pending approval callbacks keyed by `requestId`.
- Decides engagement mode from Rust's `mode` field: `"fresh"` (no resume/continue), `"resume"` (sets `options.resume`), `"continue"` (sets `options.continue`). Never infers resumability from prior state or error strings.
- Emits `session_resume_failed` when the SDK returns a different session ID than requested, surfacing resume failure as a named state rather than retrying silently.

## Key files

- **index.mjs** — stdin reader, query loop, per-(message-id, block-index) incremental diff logic, approval/abort handling, all stdout event emission.
- **package.json** — declares `@anthropic-ai/claude-agent-sdk` dependency and marks the package as ESM.

## Invariants & gotchas

- **Next-turn mode switching**: `set_mode` persists the mode in `sessionPermissionMode` and applies it to `options.permissionMode` on the *next* `query` call, because the SDK's `setPermissionMode()` is a streaming-only control request and this sidecar uses non-streaming `query(prompt)`.
- **Resume is DB-authoritative**: Rust decides `mode: "resume" | "continue" | "fresh"` from database state; the sidecar translates it to exactly one of `options.resume` / `options.continue` / neither. Never infer from process state or SDK error strings.
- **Always emit `turn_completed`**: guaranteed even on abort or error (via `finally` block), so Rust never hangs waiting.
- **Diff per (message id, block index)**: An agentic turn yields one assistant message per API round-trip between tool calls. `streamMsgId` / `streamBlockLens[]` reset when a new message id appears; repeated snapshots of the *same* message emit only new chars. This fixed the bug where mid-turn reasoning was sliced by the *previous* message's length and dropped as empty or corrupted.
- **Duplicate tool-card guard**: `streamToolIdsSeen` set tracks tool ids announced this turn; prevents duplicate `tool_use_start` events if the same message snapshot is yielded more than once.
- **Approval callbacks are promises**: `canUseTool` emits an `approval_request`, stores the resolve function in `pendingApprovals`, and blocks until `approval_response` arrives from Rust.
- **Resume failure is surfaced, not disguised**: when `options.resume` is set but the SDK returns a different session ID (or throws before init), emit `session_resume_failed` as a named event; never silently downgrade to a fresh session.
- **Load filesystem settings**: `options.settingSources = ["user", "project", "local"]` so embedded sessions read `.claude/settings` hooks (timeline feed), `CLAUDE.md` (feature index), and `.mcp.json` (forge MCP server) like terminal `claude` runs.
