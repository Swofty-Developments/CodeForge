---
# Session Manager

## Purpose

Owns all live Claude Code sessions for a Tauri app instance, spawning sidecar processes and routing IPC commands (send/approve/abort/stop/set-mode) to the correct session via UUIDs.

## How it works

- Sessions are keyed by UUID v4; the registry holds the `ClaudeSession`, display title, and model hint.
- `start_session` spawns a new session from an explicit mode (Fresh or Resume) — a failed resume surfaces as an event, never a silent fallback.
- Lifecycle commands (send/approve/abort/set-mode) validate input (e.g., mode names) at the manager layer before forwarding to the session.
- `list()` returns a snapshot of all sessions, distinguishing pre-handshake (`Starting`) from post-handshake (`Ready`) based on whether the Claude session ID has been captured from `session_ready`.
- `stop()` removes the entry, aborts all tokio tasks, and kills the sidecar; the stderr task is aborted first so a graceful stop never emits a `SessionError`.

## Key files

- **crates/forge-session/src/manager.rs** — registry of live sessions, routes IPC commands by UUID
- **crates/forge-session/src/claude.rs** — spawns sidecar + 4 tokio tasks (stdin writer, stdout parser, stderr collector, first-query augmenter)
- **crates/forge-session/src/shell_env.rs** — login-shell environment resolution (once per process, merged onto spawned commands)
- **crates/forge-session/src/locate.rs** — sidecar script resolution (macOS bundle, beside exe, dev walk-up, compile-time fallback)
- **crates/tauri-app/agent-sidecar/index.mjs** — Node.js sidecar wrapper around `@anthropic-ai/claude-agent-sdk`; emits NDJSON events over stdout

## Invariants & gotchas

- **Mode validation happens at the manager layer** — `set_mode` rejects invalid modes (`"yolo"`) as `Error::InvalidMode` before the session lookup; a known-valid mode then reaches the sidecar.
- **Permission mode intentionally not carried across resume** — CodeForge parity dictates that a resumed session starts in default permission mode regardless of the original.
- **Node must be found on the login-shell PATH, not process PATH** — desktop-launched apps get a minimal env; if `shell_env::which("node")` returns `None`, the error message explains whether the login shell was resolved (install Node) or unresolved (shell startup files broken).
- **The augmenter task splices init params (cwd/model/permissionMode/mode) into the first query only** — subsequent queries carry none of these fields; `set_mode` goes through a separate `{"type":"set_mode"}` message that bypasses the augmenter.
- **Stderr EOF always emits `SessionError`** — the sidecar exiting while tasks are live is abnormal; `stop()` aborts tasks first so the stderr collector is gone before kill, preventing spurious crash events.
- **The Claude session ID is captured from `session_ready` into a `OnceLock`** — `list()` uses this to distinguish `Starting` vs `Ready`; the hint passed at start (`thread_hint`) is only a fallback for display.
- **Streaming content is diffed per (message id, block index), not globally per turn** — an agentic turn yields one assistant message per API round-trip between tool calls; the sidecar's `streamMsgId`/`streamBlockLens` reset when a new message id appears, so mid-turn reasoning and text (post-tool-call assistant messages) are fully captured instead of dropped or corrupted by slicing against the previous message's length. Duplicate tool cards are guarded by `streamToolIdsSeen`, a per-turn set.
- **ThinkingBlock shows a live 100-char tail while streaming** — `liveTail()` slices the last 100 chars of flattened reasoning so the user sees what Claude is thinking *now*, not an ellipsis hiding content behind animated dots. When streaming completes, it collapses to a `preview()` of the opening 100 chars.
