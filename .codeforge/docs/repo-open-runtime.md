# Repo Open Runtime

## Purpose

Opens a path as a repo context: installs the kit, opens timeline + index, starts the daemon, upserts the repos table row, wires the timeline→`timeline:event` forwarder, registers `RepoRuntime`, and kicks off cold-start indexing when never indexed. Idempotent for an already-open repo. Also provides symmetric teardown (`close_context`) reused by `close_repo` and `remove_worktree`.

## How it works

**open_context**:
1. Canonicalizes `root` and checks if already open (returns current state if so).
2. Installs the MCP binary kit via `forge_daemon::install_kit`.
3. Opens `TimelineStore` and loads `FeatureIndex` from the repo root.
4. Starts the `Daemon` with `DaemonDeps` (index + timeline + root).
5. Upserts the `repos` row and reads `indexed_at` in one spawn_blocking round-trip (DB source of truth, CONTRACT-3).
6. Subscribes to timeline broadcast before spawning the forwarder task — events are wrapped in `TimelineEventEnvelope{repo_path, event}` (FZ-5) so the frontend appends them only to the active context (no cross-worktree leak).
7. Spawns the staleness poller (FZ-2): checks on-disk verdict immediately and every POLL_INTERVAL, emits `index:status` on change so the UI can prompt re-index (never auto-reindexes).
8. Registers `RepoRuntime` in `state.repos` with canonical path as key (contract W1).
9. If `indexed_at IS NULL` (never indexed), spawns cold-start indexing via `indexing::spawn_reindex`.
10. Assembles `RepoState`, sets `indexed_at` from DB (CONTRACT-3), fetches current branch + project name, emits `repo:changed`.

**close_context**:
1. Removes the `RepoRuntime` from `state.repos`.
2. Stops every session mapped to this root (sessions can't outlive their context).
3. Aborts forwarder + staleness tasks, shuts down the daemon.
4. Returns `true` if a context was open (now closed), `false` if nothing was open (legitimate no-op for `remove_worktree`, error for `close_repo`).

## Key files

- `crates/tauri-app/src/runtime/repo_open.rs`: core open/close logic
- `crates/tauri-app/src/runtime/repo_util.rs`: canonical path, git-repo detection, `RepoState` assembly, project naming
- `crates/tauri-app/src/runtime/indexing.rs`: cold-start reindex spawner
- `crates/tauri-app/src/runtime/staleness.rs`: staleness poller (FZ-2)

## Invariants & gotchas

- **CONTRACT-3**: `repos.indexed_at` (DB) is the single source of truth. `indexed_at IS NULL` ⇔ never indexed, triggering cold-start.
- **FZ-5**: Live events must be tagged with the emitting context's canonical path (`TimelineEventEnvelope`) so the frontend appends them only to the active context (no cross-worktree leak).
- **W1**: Worktrees become their own `RepoRuntime` keyed by their canonical path (each has its own daemon / index / timeline).
- Idempotency: an already-open repo just reports current state (lines 56–69).
- `canonical()` pins the `repos` map key so every command resolves the same key.
- Timeline subscription happens BEFORE spawning the forwarder to avoid losing events (line 106).
- Staleness poller emits `index:status` on change; never auto-reindexes (the UI decides).
- Sessions mapped to a closing context are stopped first (sessions can't outlive their context).
