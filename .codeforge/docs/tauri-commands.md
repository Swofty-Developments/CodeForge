---
# Tauri Commands

## Purpose
IPC command layer exposing backend functionality to the Tauri frontend via `#[tauri::command]` handlers. Bridges Tauri events to `AppState` (repos, sessions, terminals) and delegates to domain modules: `forge-core`, `forge-session`, `forge-git`, `forge-timeline`. Every command returns `Result<T, String>` (errors formatted as `format!("{e:#}")` for anyhow chains); IDs and paths cross IPC as strings.

## How it works
Commands are grouped by domain in `crates/tauri-app/src/commands/{repo,features,timeline,sessions,worktrees,terminals,system}.rs`, registered in `main.rs` via `tauri::generate_handler![]`. Tauri's macro bridges them to frontend `ipc.ts` calls. State access is `tauri::State<'_, AppState>`, guarded by tokio Mutexes.

**Repo commands** (`open_repo`, `init_repo`, `close_repo`, `reindex_repo`, `daemon_status`, `check_claude_cli`) manage repository contexts: install integration kit, open timeline/index, start daemon, spawn cold-start indexing when first opened. A worktree opens through the SAME `repo_open::open_context` core (contract W1). Close tears down sessions rooted in the repo, shuts down daemon, drops runtime. `check_claude_cli` verifies Claude CLI installation and auth via `forge_session::shell_env::which("claude")`, used by the frontend to gate indexing features.

**Session commands** (`start_session`, `send_session_input`, `approve_session`, `interrupt_session`, `stop_session`, `list_sessions`, `list_past_sessions`, `set_session_mode`, `rename_session`) manage embedded Claude Code sessions via `SessionManager`. Precondition (CONTRACT-2): repo must be OPEN — a started session has a thread row by construction. Mode resolution is DB-authoritative (`resolve_session_mode`): no resume id → Fresh; an unrecorded id is a named error, never a silent downgrade. Events flow to the forwarder that re-emits `AgentEvent`s on the `agent-event` channel. Auto-renames thread from first message text.

**Approval flow** (`approve_session`): answers pending `approval_required` or `ask_user_question` events. For **AskUserQuestion**, the `answers: Option<serde_json::Value>` param must carry `{question_text: "selected_label", ...}` (", "-joined for multiSelect) on approve — omitting answers makes the Agent SDK tool return "The user did not answer the questions." Flow: QuestionCard (blocks.tsx) → `appStore.approveRequest(..., answers)` → `ipc.approveSession` → `approve_session` command → `manager.approve` → `claude.rs respond_to_approval` → sidecar `approval_response{answers}` → canUseTool resolve. QuestionCard builds `answers` only on approve; deny sends `None`.

**Worktree commands** (`list_worktrees`, `create_worktree`, `list_branches`, `fetch_remotes`, `add_worktree_for_branch`, `remove_worktree`, `merge_worktree`) wrap `forge_git` primitives (contract W3). Created worktrees inherit the base's feature model (`worktree_fs::inherit_features`), keep `.codeforge-worktrees/` ignored, then open as their own context (own daemon/index/timeline). Merge records a System note on the BASE timeline; the base must be open.

**Terminal commands** manage PTY lifecycle via `PtyManager` (spawn, resize, input, close). **Timeline commands** stream repo events. **Feature commands** expose the index.

## Key files
- `commands/mod.rs` — module registry (repo, features, timeline, sessions, worktrees, terminals, system) + doc header
- `commands/repo.rs` — open/close/init/reindex repo contexts; daemon status; CLI check (core)
- `commands/sessions.rs` — session lifecycle, approval flow, auto-rename, past sessions (core)
- `commands/worktrees.rs` — create/list/merge worktrees; branch listing; fetch
- `commands/terminals.rs`, `features.rs`, `timeline.rs`, `system.rs` — terminal PTYs, index reads, event streams, system utilities
- `main.rs` — Tauri builder + `invoke_handler` registration
- `frontend/src/ipc.ts`, `frontend/src/components/session/blocks.tsx` — IPC client; QuestionCard/ApprovalCard rendering
- `crates/forge-index/src/headless.rs` — `claude -p` invocation for indexing (6min timeout, read-only tools, sonnet-4-5)

## Invariants & gotchas
- **CONTRACT-2**: `start_session` requires an OPEN repo (no repo row → no thread anchor). Check `state.repos` first.
- **AskUserQuestion approval protocol**: sidecar's `approval_response` for AskUserQuestion MUST carry `answers` on allow, or Claude sees "The user did not answer the questions." `answers` maps question TEXT (not index!) → selected option LABEL (", "-joined for multiSelect).
- QuestionCard had two bugs: (1) used `<For>` without importing from solid-js → crashed, questions never rendered; (2) sent only `{decision:"allow"}` → sidecar resolved canUseTool with `updatedInput=input` (no answers). Fixed by importing `For`, adding `answers` param to the whole chain, and building answers from `selectedAnswers` state.
- Session mode is DB-decided (`resolve_session_mode`), not discovered sidecar-side. Resume id must name a recorded `sessions.claude_session_id`; unknown id → named error.
- Worktree paths are canonical (`repo_util::canonical`) to avoid aliasing bugs. Contract W4: `repo_path` in worktree commands may itself be a worktree; `forge_git::base_repo_root` derives the base first.
- Close repo tears down ALL its sessions before shutting daemon — a session can't outlive its repo.
- Errors are formatted `format!("{e:#}")` for full anyhow chain, then serialized as strings across IPC.
- **CLI availability**: `check_claude_cli` uses `forge_session::shell_env::which("claude")` to resolve the binary on the login-shell PATH. A missing CLI is NOT a hard error for opening a repo, but indexing features depend on it; `headless.rs` surfaces a named error ("Claude CLI not found on PATH...") when invoked without it.
