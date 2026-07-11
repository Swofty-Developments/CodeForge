---
# Tauri IPC Commands

**Purpose**

Tauri command handlers bridge the Svelte frontend to the Rust backend, exposing repository, feature, session, worktree, and terminal operations over the IPC boundary. Every command returns `Result<T, String>`, with errors formatted via `format!("{e:#}")` to preserve anyhow's cause chains.

**How it works**

- Commands are organized by domain: `repo`, `features`, `sessions`, `timeline`, `worktrees`, `terminals`.
- Each command takes `State<'_, AppState>` to access the global `repos` map (keyed by canonical path), `sessions`, `terminals`, and the app database.
- Repo commands (`open_repo`, `close_repo`, `reindex_repo`) manage `RepoRuntime` instances: daemon handles, feature indexes, timeline stores, two background tasks (timeline forwarder + staleness poller), and a `reindexing` flag.
- `RepoRuntime` owns the **staleness poller** (`spawn_poller`): first tick fires immediately on open, then every 10s. Emits `index:status` only when the verdict CHANGED (fresh ↔ stale/outdated), so the UI is re-prompted only on transitions. While `reindexing` is set the poll is skipped; `last` is reset so the first post-reindex verdict always emits (typically back to `fresh`). Aborted on context close.
- Feature/timeline commands require the target repo to be OPEN — they fail with "repo is not open" if the canonical path has no entry in `state.repos`.
- Session commands enforce CONTRACT-2: a session cannot start unless its repo is open (the repo row anchors persistence). Resume mode is DB-authoritative: a given `resume_session_id` must exist in `sessions.claude_session_id` or the command fails.
- **Auto-title on first message**: `send_session_input` detects sessions titled "Session N" and auto-generates a title from the input text (via `generate_title_from_text`: first sentence ≤60 chars, strips code fences). The rename happens synchronously after the message is sent, via the internal `do_rename_impl` (DB + in-memory update).
- **Session rename**: `rename_session` (Tauri command) and `do_rename_impl` (internal helper) find the session's `thread_id` from live sessions or DB, update `threads.title`, and sync the `SessionManager` if the session is running. The frontend supports double-click-to-rename (inline edit field).
- **Past session listing**: `list_past_sessions` returns resumable sessions with `claude_session_id` for a repo; the frontend filters out live sessions by id comparison before showing the "Resume a session" list.
- Worktree commands reuse the same `repo_open::open_context` core as `open_repo` (contract W1): each worktree becomes its own `RepoRuntime` with isolated daemon/index/timeline/staleness-poller.

**Key files**

- `main.rs` — Tauri app entry point, database setup, managed state, and command registration via `generate_handler![]`.
- `commands/mod.rs` — Domain module registry and shared `Result<T, String>` convention.
- `commands/repo.rs` — `open_repo`, `init_repo`, `close_repo`, `reindex_repo`, `daemon_status`.
- `commands/features.rs` — `get_features`, `get_feature`, `pin_feature`, `update_feature`, `set_feature_color`, `index_status`.
- `commands/sessions.rs` — `start_session`, `send_session_input`, `rename_session`, `list_past_sessions`, `approve_session`, `interrupt_session`, `stop_session`, `list_sessions`, `set_session_mode`. Internal helpers: `resolve_session_mode`, `create_thread_and_session`, `do_rename_impl`, `generate_title_from_text`.
- `commands/timeline.rs` — `get_timeline` (with `TimelineFilter`), `get_diff_by_feature`.
- `commands/worktrees.rs` — `list_worktrees`, `create_worktree`, `remove_worktree`, `merge_worktree`, `list_branches`, `add_worktree_for_branch`.
- `commands/terminals.rs` — `open_terminal`, `write_terminal`, `resize_terminal`, `close_terminal`, `list_terminals`.
- `state.rs` — `AppState` structure (repos map, session manager, terminals), `RepoRuntime` (added `staleness_task: JoinHandle`), helper methods (`index()`, `timeline()`, `index_and_timeline()`).
- `runtime/staleness.rs` — `spawn_poller(app, root, reindexing)` returns the `JoinHandle` owned by `RepoRuntime.staleness_task`; `compute(root)` is the on-disk verdict helper.

**Invariants & gotchas**

- **Repo open precondition**: Feature, timeline, and session commands assume the target repo is open. Calling them on a closed repo returns "repo is not open", not a panic.
- **Canonical paths only**: Every repo path crosses IPC as a string, is canonicalized via `repo_util::canonical()`, and becomes the HashMap key. Non-canonical paths will fail to match.
- **Locking discipline**: `db` is a `std::sync::Mutex` — take it in a tight scope, never held across `.await`. Tokio mutexes (`repos`, `sessions`, `session_repos`) are fine across awaits.
- **Session lifecycle**: A session cannot outlive its repo. `close_repo` tears down all sessions rooted at the closing repo; `start_session` fails if the repo is not open.
- **Resume must be recorded**: A `resume_session_id` must exist in `sessions.claude_session_id` or `start_session` fails with "cannot resume" — it is never silently replaced by a fresh start.
- **Auto-title fires once**: `send_session_input` only auto-renames on the first message (title starts with "Session "). Subsequent messages skip the rename path. A failed auto-rename is logged but not surfaced to the user.
- **Timeline append failures surface**: Pin/edit/merge operations persist their state FIRST, then append a timeline event. If the append fails, the error is surfaced (not swallowed) with context like "edit persisted but timeline append failed".
- **Staleness polling & reindex coordination**: The staleness poller resets its last-emitted state while `reindexing` is true, so the first post-reindex tick always re-emits (typically `fresh`, clearing the UI dot). The frontend pops the stale modal only on state TRANSITIONS (prev.state !== status.state), so a growing `changedFiles` list re-emitted by the 10s poller updates the status-bar dot without re-popping a dismissed modal.
- **Blocking IO in spawn_blocking**: `get_timeline` runs the rusqlite query in `tokio::task::spawn_blocking` to keep synchronous DB access off the async runtime thread. Same for `staleness::compute`, which hashes the manifest.
