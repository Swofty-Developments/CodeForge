# Indexing Runtime

## Purpose

Orchestrates cold-start feature decomposition by invoking the headless `claude -p` indexer in a background tokio task, streaming progress events to the frontend, merging results (preserving pinned features), persisting the index to disk, generating living docs for each feature, and bookending the operation with `IndexStarted`/`IndexCompleted` timeline events.

## How it works

- `spawn_reindex` claims an atomic per-repo "reindexing" flag (errors if one is already running) and spawns a tokio task; a `ReindexGuard` RAII type clears the flag on drop (covers task panics).
- `run` appends an `IndexStarted` timeline event, launches a mpsc-based progress forwarder that streams `IndexProgress` stages to the frontend via `index:progress` Tauri events, then delegates to `run_indexed`.
- `run_indexed` calls `Indexer::cold_start` (headless `claude -p` decomposition), merges the results into the in-memory index (pinned features survive), saves `features.json` and `index-meta.json` atomically, writes `.codeforge/docs/<slug>.md` for each feature (parallel, capped concurrency), and records `indexed_at` in the database.
- On success, emits `repo:changed` with updated state (feature count, `indexed_at`, branch, project name); on failure, appends an `IndexCompleted` event with an `error` payload and emits an explicit `error` stage on `index:progress` (never a clean completion).
- All failures short-circuit before `indexed_at` is written or a clean `IndexCompleted` is emitted — the existing on-disk index survives verbatim if the reindex fails at any stage (decomposition, save, doc generation, DB write).
- The reindex task owns a cloned `ReindexCtx` (app handle, DB, index, timeline, repo_root, daemon port) so no state lock is held during the long-running indexer run.

## Key files

- `crates/tauri-app/src/runtime/indexing.rs` — spawn entry point, progress forwarding, merge-and-save orchestration, timeline bookends, `indexed_at` persistence, failure handling.

## Invariants & gotchas

- **One reindex per repo at a time**: `spawn_reindex` errors if `reindexing` is already true; the `ReindexGuard` RAII type guarantees the flag is cleared even on panic.
- **Pinned features survive reindex**: `index.merge_reindex(features)` preserves any feature marked `pinned: true` from the pre-reindex state.
- **Failed reindex leaves the old index intact**: any `Err` short-circuits before `indexed_at` is written or a clean `IndexCompleted` is emitted, so the existing on-disk index (features.json, index-meta.json, docs) remains verbatim.
- **`indexed_at` write is mandatory for success** (CONTRACT-3): if `persist_indexed_at` fails the entire reindex is treated as a failure, preventing the UI from showing stale "never indexed" state right after a successful decomposition.
- **Empty feature set is a named failure**: if `cold_start` returns zero features (validation dropped all candidates or the model whiffed), the indexer errors rather than merging an empty set (which would drop the entire index).
- **Atomic JSON writes**: `save()` writes to a `.json.tmp` file then renames, so readers never observe a half-written index.
- **`index-meta.json` is rewritten over the merged set**: after a successful reindex, `write_meta()` rewrites the content-hash manifest from the new feature list so `index_status()` reports `fresh` (FZ-2 staleness detection).
