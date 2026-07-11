The edited files are in the session management feature, not the indexing runtime. The indexing runtime code itself (`indexing.rs`) was not modified this turn. The living doc is already accurate and complete — no update is needed.

---

# Indexing Runtime

Orchestrates cold-start / re-index tasks for a repository, merging new features with pinned ones, saving the index and meta (FZ-2 manifest), writing living-docs, and streaming IndexProgress events (`scan`, `decompose`, `validate`, `docs i-of-n`, `done`) to the frontend via `index:progress`.

## Purpose

A repository's feature index starts empty. When opened for the first time (`repos.indexed_at IS NULL`), `open_context` spawns a cold-start reindex automatically. Later, the UI can request a re-index (force vs. merge) from the command or when the staleness poller detects drift.

The indexing runtime:
- Prevents concurrent reindexes per repo (atomic `reindexing` flag, one task at a time)
- Merges the fresh index with the existing one (pinned features survive)
- Saves the index and rewrites the FZ-2 manifest hash
- Writes living-docs for features
- Records `indexed_at` in the DB (CONTRACT-3: the single source of truth)
- Bookends the timeline with `IndexStarted` / `IndexCompleted` events
- Emits a progress stream so the frontend can show an in-flight UI
- On failure, leaves the existing index untouched and surfaces an error stage

## How it works

**spawn_reindex(ctx)**
- Claims the per-repo `reindexing` flag (swap true; fails if already set)
- Spawns the async task; a `ReindexGuard` RAII releases the flag on drop (covers panic)

**run(ctx)**
- Appends `IndexStarted` to the timeline
- Spawns a forwarding task: `index:progress` events bridge the indexer's mpsc channel to Tauri
- Calls `run_indexed` (happy path): decompose → merge → save → write docs → persist `indexed_at`
- On success: appends `IndexCompleted { features, docsWritten, docsFailed }`, stamps `repo:changed` with the new count/timestamp
- On failure: appends `IndexCompleted { error }`, emits an explicit `error` stage to the UI, leaves the on-disk index + `indexed_at` unchanged

**run_indexed(ctx, progress_tx)**
- `Indexer::cold_start` runs the full decomposition pipeline, publishing progress stages
- Acquires `index.write()`, merges the fresh features (pins survive), saves, writes FZ-2 meta
- `Indexer::write_feature_docs` generates living-docs (warns on partial failures but does not abort)
- `persist_indexed_at` writes `repos.indexed_at = now` in a blocking task and returns the timestamp
- Returns `Completion { report, indexed_at }` so the event carries the real DB value

**Integration with staleness**
- The staleness poller (`staleness::spawn_poller`) runs every 10s in the background, re-hashing the manifest and emitting `index:status` only when the verdict changed
- While `reindexing` is true, the poller skips the check (mid-reindex the manifest is being rewritten) and resets its `last` memo so the first post-reindex emit always fires (typically back to `fresh`, clearing the status-bar dot)
- The frontend's `handleIndexStatus` pops the stale modal only on a state transition (prev.state !== status.state), preventing re-prompts on every poll when the verdict hasn't changed

## Key files

- `crates/tauri-app/src/runtime/indexing.rs` — spawn_reindex, run, run_indexed
- `crates/tauri-app/src/runtime/staleness.rs` — background poller, compute, emit logic
- `crates/tauri-app/src/runtime/repo_open.rs` — open_context spawns cold-start when `indexed_at IS NULL`, spawns the staleness poller
- `crates/tauri-app/src/state.rs` — `RepoRuntime.reindexing`, `RepoRuntime.staleness_task`

## Invariants & gotchas

- **One reindex at a time per repo**: the `ReindexGuard` releases the atomic flag on drop, covering panic and normal completion
- **Failure = no durable state change**: any error before `indexed_at` is written leaves the on-disk index untouched and surfaces an explicit `error` stage; a failed save or failed meta write is treated as a full failure
- **CONTRACT-3**: `repos.indexed_at` is the source of truth; `indexed_at IS NULL` ⇔ never indexed; reindex writes it on success and stamps `repo:changed` with the returned timestamp
- **The staleness poller skips polls while reindexing**: `reindexing.load()` guards the hash check so mid-reindex manifest churn doesn't spam `index:status`
