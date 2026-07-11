---
# IPC Layer (ipc-layer)

TypeScript wrappers around Tauri IPC invoke calls. Mirrors the Rust command signatures, handles JSON marshalling, and provides typed return values for the frontend.

## Purpose

Components never call `invoke()` directly. This layer provides:
- One typed function per Tauri command (the frozen contract in `main.rs`)
- Event listener helpers that unwrap `e.payload` and map Tauri's channel names to domain types
- Auto-conversion of camelCase JS args to snake_case Rust params (`repoPath` → `repo_path`)
- Centralized doc on event channels and their payloads

## How it works

Each `ipc.ts` function maps to a Tauri command registered in `main.rs`:
- `openRepo(path)` → `invoke("open_repo", {path})` → backend `open_repo` handler
- `checkClaudeCli()` → `invoke("check_claude_cli")` → backend `check_claude_cli` handler (new)
- Return types mirror `types.ts` (the frontend copy of Rust serde shapes)

Session approval flow (end-to-end, post-fix):
1. Sidecar emits `approval_request{canUseTool}` or `ask_user_question{questions}`
2. Session manager converts to `AgentEvent::ApprovalRequired{requestId, description}` or `AgentEvent::AskUserQuestion{requestId, questions}`
3. Frontend shows QuestionCard or approval pill
4. User clicks → `appStore.approveRequest(sessionId, requestId, true, answers?)` → `ipc.approveSession(id, requestId, approve, answers)`
5. Tauri `approve_session(id, requestId, approve, answers: Option<Value>)` → `manager.approve(requestId, approve, answers)` → `claude.rs respond_to_approval`
6. Sidecar resolves `canUseTool` with `updatedInput: {questions, answers}` where `answers` maps question text → selected option label(s)

Event channels (`listen<T>(channel, cb)`):
- `"agent-event"` — all session streaming, demuxed by `sessionId`
- `"timeline:event"` — `{repoPath, event}` (per-context)
- `"index:progress"` / `"index:status"` — indexing lifecycle
- `"terminal:data"` — BASE64-encoded PTY output (decode before `term.write`)
- `"terminal:exit"` — child process ended

## Key files

- `crates/tauri-app/frontend/src/ipc.ts` (core) — every invoke wrapper + listener
- `crates/tauri-app/frontend/src/types.ts` (support) — TypeScript mirrors of Rust serde shapes
- `crates/tauri-app/src/commands/*.rs` — Rust handlers these wrappers call (organized by domain: `repo`, `sessions`, `terminals`, `timeline`, `worktrees`, `features`)

## Invariants & gotchas

- Tauri auto-converts camelCase JS args to snake_case Rust params; keep names aligned
- Terminal data is BASE64-encoded on the wire (Tauri event payload is JSON); decode before writing to xterm
- `approveSession`'s `answers` param is required ONLY for `AskUserQuestion` approvals — it's a record of question text → selected option label(s). The sidecar's Agent SDK contract demands `updatedInput.answers` on allow; sending `null` causes Claude to see "The user did not answer the questions."
- QuestionCard must import `<For>` from solid-js; missing the import crashes rendering and questions never show
- `approval_response` for AskUserQuestion MUST carry `answers` on allow, or the Agent SDK tool returns an unanswered result
- `ClaudeCliStatus` is returned by `checkClaudeCli()` and includes `installed`, `version`, `authenticated` fields — use this to gate session creation from the UI
