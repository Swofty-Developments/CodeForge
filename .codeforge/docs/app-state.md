# App State

## Purpose

Tauri-managed singleton holding the app SQLite database, per-repo runtimes (index, timeline, daemon), session manager, session-to-repo mappings, and terminal manager. All Tauri commands receive this state as a shared dependency.

## How it works

- **`AppState` is Tauri-managed state** — constructed once in `main.rs`, stored via `.manage(app_state)`, injected into every command via `State<'_, AppState>`.
- **`RepoRuntime` wraps one open repository's services** — daemon handle, feature index, timeline store, background tasks (event forwarder, staleness poller), and a reindexing flag to serialize index rebuilds.
- **`repos` maps canonical repo root → `RepoRuntime`** — created by `open_repo`, torn down by `close_repo`. A worktree opens the same core root (not its worktree path).
- **`session_repos` maps session id → repo root** — tracked so `close_repo` can tear down every session rooted at the closing repo (a session must not outlive its runtime).
- **`db` is a `std::sync::Mutex`** — taken in tight scopes, never across an `.await`; async tasks write via `spawn_blocking`.
- **App DB lives at `~/.codeforge/codeforge.db`** — missing `HOME`/`USERPROFILE` is a fatal startup error, not a silent `.` fallback.

## Key files

- `crates/tauri-app/src/state.rs` — `AppState` and `RepoRuntime` structs, index/timeline accessor helpers.
- `crates/tauri-app/src/main.rs` — DB path resolution, state construction, Tauri builder registration.
- `crates/tauri-app/src/db.rs` — SQLite wrapper with WAL mode and migrations; single connection behind `Arc<Mutex>`.
- `crates/tauri-app/src/commands/repo.rs` — `open_repo`, `close_repo`, runtime lifecycle commands.

## Invariants & gotchas

- **Locking order: never hold `db` across an `.await`** — `std::sync::Mutex` will panic if a thread blocks while holding it. Use `spawn_blocking` for async DB writes.
- **`session_repos` is authoritative for teardown, not routing** — event routing uses `sessionId` in payloads (CONTRACT-1), but close logic iterates this map to kill sessions.
- **One reindex per repo** — `RepoRuntime.reindexing` is an `AtomicBool` slot; `spawn_reindex` fails fast if a reindex is already running.
- **Canonical repo root as key** — both `repos` and `session_repos` use the canonical path (symlinks resolved) so worktrees and their core share the same runtime.
- **`repos` cleanup aborts background tasks** — dropping a `RepoRuntime` must abort `forwarder_task` and `staleness_task` to prevent orphaned emitters.
