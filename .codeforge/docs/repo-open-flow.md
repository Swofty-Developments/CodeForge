# Repo Open Flow

## Purpose

Initializes a repository context (base or worktree) by installing the integration kit, loading the feature index and timeline, starting the daemon HTTP server, and triggering cold-start indexing if the repo has never been indexed. All open contexts share the same code path.

## How it works

- **Idempotent entry** — `open_context` canonicalizes the path and returns early if already open (re-reporting current state from the in-memory `RepoRuntime`).
- **Kit + daemon** — installs the MCP binary and hook kit into `.codeforge/`, opens the timeline store and feature index from disk, starts a daemon HTTP server on an ephemeral port.
- **Database upsert** — upserts the `repos` row and reads `indexed_at`; when `NULL` (CONTRACT-3: never indexed), spawns cold-start indexing in the background.
- **Live wiring** — subscribes to timeline events and spawns a forwarder task emitting `timeline:event` tagged with this context's canonical path (FZ-5); spawns a staleness poller emitting `index:status` on every verdict change.
- **Runtime registration** — inserts the `RepoRuntime` into `AppState.repos` keyed by canonical path, making the index/timeline/daemon accessible to commands.
- **Context teardown** — `close_context` stops all sessions rooted at the path (a session cannot outlive its context), aborts the forwarder and staleness tasks, shuts the daemon down, and removes the runtime from `AppState.repos`.

## Key files

- **repo_open.rs** — `open_context` (setup), `close_context` (teardown), timeline forwarder, `indexed_at` DB reads (CONTRACT-3).
- **repo_util.rs** — path canonicalization, git-repo detection, `repo_name` / `project_name` derivation, pure `RepoState` assembly (does not set `indexed_at`).
- **state.rs** — `RepoRuntime` definition (daemon handle, index, timeline, tasks, `reindexing` flag); `AppState` map keyed by canonical path.

## Invariants & gotchas

- **CONTRACT-3: `repos.indexed_at` (database) is the single source of truth** — `RepoState.indexed_at` is always loaded from the DB via `queries::get_repo_indexed_at`, never derived or cached elsewhere. `IS NULL` means never indexed.
- **W1: worktrees become independent contexts** — a worktree is keyed by its own canonical path in `AppState.repos`, not the base repo's; it gets its own daemon, index, timeline, and staleness poller.
- **FZ-5: live events tagged by context path** — the timeline forwarder wraps each event in `TimelineEventEnvelope { repo_path, event }` so the frontend appends events only to the active context (no cross-worktree leak).
- **Cold-start only when `indexed_at IS NULL`** — opening an already-indexed repo (even if stale) does not auto-reindex; the staleness poller emits the verdict and the UI prompts the user.
- **Canonical path as map key** — every command must canonicalize its `repo_path` argument so lookups into `AppState.repos` hit the same key; `open_context` canonicalizes again for safety.
- **Session teardown on context close** — `close_context` stops every session in `session_repos` mapped to the closing root before removing the runtime, preventing sidecars from outliving their index/timeline handles.
