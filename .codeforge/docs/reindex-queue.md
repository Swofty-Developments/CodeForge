# Reindex Queue

## Purpose

Durably accumulates file paths that were edited but could not be classified to any feature (brand-new files, renames, or stale index). A subsequent reindex drains the queue and retroactively resolves the file→feature link instead of permanently losing the association.

## How it works

- When a `FileEdited` hook arrives and matches no feature, the timeline event is marked `unclassified: true` and the path is enqueued via `ReindexQueue::enqueue(path, event_id, session_id)`.
- Repeated edits to the same path collapse to a single row (UPSERT on `path` primary key), keeping only the most recent `event_id` and `enqueued_at` timestamp.
- On reindex (manual or automatic), the caller reads `pending()` to get all queued paths (oldest first), resolves them against the fresh index, then calls `clear(paths)` to remove successfully classified entries.
- Storage is a separate SQLite database (`<repo>/.codeforge/runtime/reindex.db`) so the append-only timeline DB remains clean; WAL mode and `busy_timeout=5000` allow concurrent access.
- Queue is opened once per daemon lifetime (daemon startup calls `ReindexQueue::open`) and shared across hook ingest and reindex operations.

## Key files

- **crates/forge-daemon/src/reindex_queue.rs** — core: `ReindexQueue` struct, `enqueue`/`pending`/`clear` methods, SQLite schema with single `pending_reindex` table.
- **crates/forge-daemon/src/ingest.rs** — calls `queue.enqueue` when `process_hook` discovers an unclassified file edit (lines 32–39 doc, actual invocation around file-edit handling).
- **crates/forge-daemon/src/lib.rs** — daemon startup: opens the queue (line 109), passes it to `build_router` and `drain_spool`.
- **crates/forge-daemon/src/http.rs** — wires `ReindexQueue` into axum state so hook endpoints can enqueue on ingest.

## Invariants & gotchas

- **Path is PRIMARY KEY**: repeated edits to the same unclassified file do NOT pile up; only the latest event survives. This means a stale index that never gets reindexed will show just one unclassified event per file no matter how many times the user edits it.
- **Separate database**: `reindex.db` is daemon-owned mutable state, distinct from the append-only `timeline.db`. Do not merge them; timeline events are permanent, queue entries are ephemeral.
- **Reindex is external**: the queue itself does NOT trigger reindexing. The caller (Tauri app or frontend) must call `pending()`, run a reindex, and `clear()` resolved paths. Orphaned queue entries (files deleted before reindex) are never auto-pruned — a future reindex that still finds no match will leave them queued.
- **Mutex poisoning halts the queue**: if the SQLite mutex panics while locked, all subsequent queue operations fail with "mutex poisoned". The daemon must be restarted to recover.
