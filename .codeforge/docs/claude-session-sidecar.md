None of these changes touched the sidecar itself (`agent-sidecar/index.mjs`). The edits are in the Rust backend (Tauri command registration, headless indexer retry logic). The sidecar's protocol, streaming behavior, and invariants remain unchanged.

---
# Claude Session Sidecar

## Purpose

Wraps the `@anthropic-ai/claude-agent-sdk` `query()` function in a Node.js process that speaks NDJSON over stdin/stdout, allowing the Rust backend to drive Claude sessions without embedding a JS runtime.

## How it works

- Reads commands from stdin (`query`, `approval_response`, `set_mode`, `abort`) as line-delimited JSON
- Translates each `query` command into an SDK `query()` call with proper `resume` / `continue` / fresh mode based on Rust's authoritative session state
- Streams SDK events back to Rust as NDJSON (`text_delta`, `tool_use_start`, `tool_result`, `thinking_delta`, `usage`, etc.)
- Proxies tool approval requests to Rust via `canUseTool` callback, waits for `approval_response`, returns allow/deny decision to SDK
- Incremental streaming: diffs each assistant message snapshot by `(message.id, block index)` to emit only new text/thinking deltas, preventing duplication across tool calls
- Permission mode changes (`set_mode`) take effect on the *next* query turn (SDK's `setPermissionMode()` requires streaming input; sidecar uses non-streaming prompts)

## Key files

- **`index.mjs`** — stdin/stdout bridge, streaming diff logic, approval handling, mode switching
- **`package.json`** — declares dependency on `@anthropic-ai/claude-agent-sdk`

## Invariants & gotchas

- **Rust decides session mode**: the sidecar sets exactly one of `options.resume` / `options.continue` / neither from the `mode` field. Never infers resumability from prior state or error strings.
- **Resume failure is explicit**: if `mode === "resume"` but the SDK returns a different session ID (or throws), emit `session_resume_failed` — no silent downgrade to fresh.
- **Streaming state is per-message**: `streamMsgId` and `streamBlockLens` reset whenever a new assistant message (new `message.id`) arrives. Re-yielded snapshots of the same message emit only new chars.
- **Tool IDs are de-duped per turn**: `streamToolIdsSeen` guards duplicate `tool_use_start` events if the SDK yields the same message snapshot multiple times.
- **AskUserQuestion answers required**: approving an `AskUserQuestion` without `answers` makes the tool report "The user did not answer"; Rust always sends `{ "<question>": "<label>" }` in the approval response.
- **Mode switch is next-turn**: `set_mode` persists `sessionPermissionMode`; `handleQuery` applies it as `options.permissionMode` on the next query (including continue turns that don't re-send `permissionMode`).
- **Always emit `turn_completed`**: guaranteed by the `finally` block, even on abort or error, so Rust knows the turn ended.
---
