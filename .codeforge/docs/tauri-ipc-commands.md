# Tauri IPC Commands

## Purpose
Rust command handlers that bridge the TypeScript frontend to Rust backend state (repos, features, sessions, worktrees, terminals). Every command returns `Result<T, String>` with anyhow errors formatted as `format!("{e:#}")` chains.

## How it works
- **Repo lifecycle** — `open_repo` validates a git directory, installs the integration kit, starts the daemon, and optionally triggers cold-start indexing; `init_repo` runs `git init` then opens through the same flow; `close_repo` tears down sessions, shuts down the daemon, and drops runtime state.
- **Feature CRUD** — `get_features` / `get_feature` read from the open index; `pin_feature`, `update_feature`, and `set_feature_color` mutate the index, persist it, and append a timeline event (a failed timeline append surfaces as an error, not a silent swallow).
- **Session management** — `start_session` resolves the `SessionMode` from the DB (fresh vs. resume), spawns the sidecar, persists a thread + session row (tearing down the session if persistence fails), and spawns an event forwarder; `send_session_input` queues a prompt and auto-generates a title on the first message; `stop_session` kills the sidecar and forgets the repo mapping.
- **Worktree ops** — `create_worktree` and `add_worktree_for_branch` delegate to `forge_git`, inherit the base's feature model, and open the worktree as its own context (own daemon/index/timeline); `remove_worktree` closes the context first so no daemon/session runs against a vanishing path; `merge_worktree` merges into the base and appends a System note to the base's timeline.
- **Terminals** — `open_terminal` spawns a PTY in `TerminalManager` and returns its id; `write_terminal` / `resize_terminal` / `close_terminal` forward to the manager; output streams on `terminal:data` (base64-encoded) and `terminal:exit` events.
- **Timeline** — `get_timeline` runs sync rusqlite queries in `spawn_blocking`; `get_diff_by_feature` reads the pending diff grouped by feature for the diff review view.

## Key files
- `crates/tauri-app/src/commands/mod.rs` — module root, documents the `Result<T, String>` convention
- `crates/tauri-app/src/commands/repo.rs` — repo lifecycle (open, init, close, reindex, daemon_status, check_claude_cli)
- `crates/tauri-app/src/commands/features.rs` — feature CRUD (get, pin, update, set_color) + index_status for staleness checks
- `crates/tauri-app/src/commands/timeline.rs` — timeline queries and diff-by-feature view
- `crates/tauri-app/src/commands/sessions.rs` — session lifecycle (start with resume-mode resolution, send with auto-title, approve, interrupt, stop, list, rename)
- `crates/tauri-app/src/commands/worktrees.rs` — worktree ops (list, create, add_for_branch, remove, merge), all reusing `open_context` / `close_context` from repo.rs
- `crates/tauri-app/src/commands/terminals.rs` — terminal PTY commands (open, write, resize, close, list)

## Invariants & gotchas
- **SESSION-REPO CONTRACT** — a session cannot start unless its repo is open; `start_session` checks this first and returns `"repo is not open"` if violated. A session whose thread/session rows can't be persisted is torn down immediately rather than running headless.
- **Timeline appends are surfaced** — a feature pin/edit/color whose index save succeeds but timeline append fails is a REAL error surfaced to the caller, not reduced to a log line. The timeline is the durable audit log.
- **Worktrees reuse repo open/close** — `create_worktree` and `add_worktree_for_branch` both call `repo_open::open_context` so the worktree gets its own daemon/index/timeline; `remove_worktree` calls `close_context` first so no state runs against a vanishing path. Contract W1.
- **Resume mode is DB-authoritative** — `start_session` resolves the `SessionMode` from the DB via `resolve_session_mode`; a given `resume_session_id` that isn't a recorded `claude_session_id` is a named error, never a silent fresh start.
- **IDs and paths cross IPC as strings** — UUIDs and PathBufs serialize to/from String; `repo_util::canonical` is called on every path parameter to resolve symlinks and normalize.
- **Auto-title on first message** — `send_session_input` checks if the title is still `"Session N"` and generates a readable title from the first ~60 chars or first sentence of the user's input, stripping code fences and whitespace.
