# App State

**Purpose**  
Global singleton holding the SQLite database, per-repo runtime resources (daemon, index, timeline), session registry, and embedded terminal manager. Constructed once at app launch and passed to all Tauri command handlers via `.manage()`.

**How it works**  
- `AppState` wraps a `std::sync::Mutex<Database>` for synchronous SQLite access; async event tasks use `spawn_blocking` to write without holding the lock across `.await`.
- Each open repository is registered in a `HashMap<PathBuf, RepoRuntime>` (keyed by canonical repo root) holding the daemon handle, feature index, timeline store, repo ID, and a forwarder task that streams timeline events to the frontend.
- `SessionManager` tracks all active Claude sessions; `session_repos` maps session IDs to repo roots so closing a repo tears down every session rooted at that repo.
- `TerminalManager` holds PTY instances; interior-mutable and killed on app drop.
- Helper methods (`index`, `timeline`, `index_and_timeline`) clone Arc handles from `repos` under a tokio lock for convenient single-repo lookups.

**Key files**  
- `crates/tauri-app/src/state.rs` — `AppState` and `RepoRuntime` definitions, index/timeline accessors.
- `crates/tauri-app/src/main.rs` — constructs `AppState` and wires it into the Tauri app via `.manage()`.

**Invariants & gotchas**  
- **Never hold `db` lock across `.await`** — it's a `std::sync::Mutex`; blocking inside async will deadlock. Use `spawn_blocking` for async writes.
- **Session must not outlive its repo** — `close_repo` must kill every session in `session_repos` for that repo root before removing the `RepoRuntime`.
- **One reindex per repo** — `RepoRuntime::reindexing` is an `AtomicBool` guard; a second reindex request must bail or queue.
- **Forwarder task must abort on close** — `RepoRuntime::forwarder_task` streams timeline events; abort it when removing the repo or the task leaks.
- **Canonical paths only** — `repos` keys must be canonicalized; symlink-mismatched paths will fail lookups.
