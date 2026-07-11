# Timeline Storage

## Purpose

Append-only per-repo event log backed by SQLite (WAL mode). Events are immutable once appended; supports filtering, live subscriptions via broadcast channel, and stores event payloads as JSON.

## How it works

- Database lives at `<repo_root>/.codeforge/runtime/timeline.db`, with WAL journaling and `PRAGMA synchronous=NORMAL` for performance.
- `NewEvent` (without `id`/`ts`) is appended via `TimelineStore::append()`, which assigns a `rowid` and UTC timestamp (truncated to microsecond precision), writes to the `events` table, and broadcasts the stored event to all subscribers.
- Single SQLite connection behind a `std::Mutex` serialises this-process writes; WAL mode keeps external readers non-blocking.
- `query()` builds a SQL `WHERE` clause from a `TimelineFilter` (feature slug via `json_each`, actor, kinds, since, before_id pagination), always returning newest-first (`ORDER BY id DESC`), default limit 200.
- `subscribe()` returns a `tokio::sync::broadcast::Receiver` that receives every event appended after the subscription.
- Enums (`Actor`, `EventKind`) serialise to snake_case strings for stable column storage; payloads and `feature_slugs` are stored as JSON text.

## Key files

- **`crates/forge-timeline/src/lib.rs`** — single-file crate: `TimelineStore`, migrations, `append()`, `query()`, broadcast channel, row/column conversions.
- **`crates/forge-timeline/tests/store.rs`** — integration tests covering filters, pagination (`before_id`), subscriptions, persistence across reopens, concurrent appends.

## Invariants & gotchas

- **Events are immutable.** Once appended, `id` and `ts` never change; no UPDATE/DELETE operations exist.
- **Timestamps are RFC3339 microsecond strings** stored as TEXT (`YYYY-MM-DDTHH:MM:SS.ffffffZ`) so lexicographic string comparison (`ts >= ?`) is chronologically correct.
- **`feature_slugs` is exact-match only.** Filtering by `"auth"` does NOT match an event tagged `["auth-flow"]`—the filter uses `json_each(feature_slugs).value = ?`.
- **Broadcast channel capacity is 512.** Slow subscribers lag and miss events (acceptable by contract); high-rate appenders may overrun the channel.
- **`before_id` paginates strictly older events.** `before_id: Some(42)` returns only `id < 42`, so when the result is empty, no older events exist.
- **Single connection is sync-locked.** Long queries or high concurrent append rates can block; external readers must tolerate `busy_timeout=5000` (5s).
