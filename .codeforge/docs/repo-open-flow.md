# Repo Open Flow

## Purpose

Initializes a repository context: validates the path, installs the MCP integration kit, loads feature index and timeline, starts the HTTP daemon, registers the runtime, upserts the database entry, spawns a staleness poller, and kicks off cold-start indexing when `repos.indexed_at IS NULL`. Returns the `RepoState` to the frontend including index status and identity.

## How it works

- **Idempotent**: an already-open repo returns its current state without re-initialization (line 56–69).
- **Integration kit**: ensures `~/.codeforge/bin/forge-mcp` is current, then calls `forge_daemon::install_kit()` to register it in `.mcp.json` (lines 72–73).
- **Runtime assembly**: opens `TimelineStore` and `FeatureIndex` from disk, starts the HTTP daemon with clones of those, upserts the `repos` row, and reads back `indexed_at` from the database (lines 75–103). `indexed_at IS NULL` means never indexed (CONTRACT-3).
- **Live event forwarding**: subscribes to the timeline and spawns a task that emits `timeline:event { repoPath, event }` (FZ-5) so the frontend appends events only to the matching context, never leaking across worktrees (lines 106–124).
- **Staleness poller**: spawns a background task that re-checks index status every 10 seconds and emits `index:status` when the verdict changes (line 132). Skips checks while reindexing.
- **Cold-start indexing**: when `indexed_at IS NULL`, spawns a reindex task that runs the headless indexer, merges results (pinned features survive), writes docs, records `indexed_at = now()` to the database, and bookends the work with `IndexStarted`/`IndexCompleted` timeline events (lines 151–166).

## Key files

- **`crates/tauri-app/src/runtime/repo_open.rs`** (core) — `open_context()` orchestrates the entire flow; `close_context()` tears down runtime, stops sessions, aborts tasks, shuts daemon.
- **`crates/tauri-app/src/runtime/mcp.rs`** — locates the freshly built `forge-mcp` binary (sibling of exe or `target/{debug,release}`), copies to `~/.codeforge/bin/forge-mcp` when newer.
- **`crates/tauri-app/src/runtime/staleness.rs`** — spawns a poller that hashes the manifest every 10 seconds and emits `index:status` on change; skips while reindexing.
- **`crates/tauri-app/src/runtime/indexing.rs`** — runs the indexer, merges, saves, writes docs, records `indexed_at`, streams `index:progress`, emits `repo:changed`.
- **`crates/tauri-app/src/commands/repo.rs`** — `open_repo` command validates path and delegates to `open_context`; `init_repo` runs `git init -b main` then opens.

## Invariants & gotchas

- **CONTRACT-3**: `repos.indexed_at IS NULL` ⇔ never indexed. The database is the single source of truth; a null value triggers cold-start (line 103).
- **FZ-5**: timeline events must carry the emitting context's canonical path (`{ repoPath, event }`) so the frontend filters by path and never leaks events across worktrees (lines 107–118).
- **Idempotent open**: an already-open repo skips daemon/timeline setup and just reports current state; the caller must canonicalize paths so the `repos` map key matches across `open_repo`, `create_worktree`, and every command (lines 53–69).
- **One reindex at a time**: `reindexing` is an `AtomicBool` per repo; `spawn_reindex` returns `Err` when one is already running (enforced in `indexing.rs:42–45`).
- **Integration kit before daemon**: `install_kit()` must succeed before daemon start, or the MCP server won't be registered. If the binary can't be located and no existing copy exists, `ensure_mcp_binary()` returns `Err` and open fails (lines 72–73).
- **Sessions can't outlive their context**: `close_context()` stops every session mapped to the repo before dropping the runtime, so no sidecar keeps running against a closed context (lines 194–209).
