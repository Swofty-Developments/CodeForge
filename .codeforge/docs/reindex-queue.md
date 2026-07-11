The edits don't touch the reindex queue's core implementation. They modify frontend UI (app-store, SessionPane) and headless indexer retries, which are outside the reindex-queue feature. The existing doc remains accurate.

---
# Reindex Queue

## Purpose

Durably tracks file edits that could not be classified to any feature (new files, renames, or stale index) so a subsequent reindex can retroactively resolve the file→feature link instead of losing the edit event forever.

## How it works

- Lives in `.codeforge/runtime/reindex.db` (SQLite with WAL mode), separate from the append-only timeline
- When `ingest::process_hook` receives a file edit that classifies to no features, it marks the timeline event `unclassified: true` and enqueues the path with `ReindexQueue::enqueue(path, event_id, session_id)`
- Repeated edits to the same path collapse to one row (UPSERT on `path`), keeping the most recent event ID and timestamp
- After a reindex completes, the indexer calls `pending()` to retrieve queued paths (oldest first), reclassifies them, updates the timeline events to link them to features, then calls `clear(paths)` to remove resolved entries
- The UI polls `len()` or `is_empty()` to show the "stale index — re-index pending" prompt

## Key files

- **crates/forge-daemon/src/reindex_queue.rs** — SQLite-backed queue; `enqueue`, `pending`, `clear`, `len` API
- **crates/forge-daemon/src/ingest.rs** — hooks `process_hook`; enqueues unclassified edits after appending to timeline
- **crates/forge-daemon/src/http.rs** — injects the shared `ReindexQueue` into the router state (used by hook endpoint)

## Invariants & gotchas

- **Durability first**: enqueue happens in `spawn_blocking` immediately after the timeline event appends (both hit SQLite). If enqueue fails, the unclassified edit is named in the timeline but won't auto-resolve on reindex — logged as a warning, not silent loss.
- **Collapse is semantic, not an optimization**: the `ON CONFLICT` UPSERT keeps the *latest* event ID per path. This is correct because the timeline event that survives is the one a human sees; intermediate edits to the same path before reindex are compressed into one "this path needs reclassification" signal.
- **The queue is a daemon runtime artifact, not user state**: it's `.gitignore`d. A reindex from scratch rebuilds correct timeline links; the queue only accelerates incremental resolution after edits land while the index is stale.
- **SQLite PRAGMA choices matter**: `journal_mode=WAL` allows readers during a write (enqueue + clear can happen concurrently), `busy_timeout=5000` prevents spurious lock failures under contention.
- **Clearing is manual**: the indexer must explicitly call `clear(resolved_paths)` after reclassifying. If it forgets, the queue grows unboundedly and the UI prompt never dismisses.
- **Ordering determinism**: `pending()` returns paths oldest-first by `enqueued_at`, with path as tiebreaker. This ensures a reindex processes the oldest unresolved edits first and produces stable results across runs.
