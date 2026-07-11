The existing doc is already accurate — the files edited this turn (`app-store.ts`, `headless.rs`, `SessionPane.tsx`, `styles-chrome.ts`) are unrelated to the timeline storage implementation. No changes needed.

---
# Timeline Storage

**Purpose**  
Append-only event log per repository, stored in SQLite at `.codeforge/runtime/timeline.db`, for recording agent/human/system activity and supporting live subscriptions via tokio broadcast channels.

**How it works**  
- Opens (or creates) `timeline.db` in WAL mode with `foreign_keys=ON`, `synchronous=NORMAL`, and 5s busy timeout; runs idempotent migrations to create the `events` table.
- `append(NewEvent)` assigns an `id` (SQLite rowid) and microsecond-truncated UTC timestamp, writes the event as a single row, and broadcasts it to all subscribers.
- `query(TimelineFilter)` filters by feature slug (JSON array membership), actor, event kinds, since timestamp, and `before_id` cursor; returns events newest-first with a default limit of 200.
- `subscribe()` returns a tokio `broadcast::Receiver` that receives every event appended after the subscription.
- Connection is behind a `std::Mutex` for in-process synchronization; WAL keeps external readers non-blocking.
- Enums (`Actor`, `EventKind`) are serialized as snake_case strings; timestamps as fixed-width RFC3339 micros for lexicographic order.

**Key files**  
- **`crates/forge-timeline/src/lib.rs`** — `TimelineStore` implementation: open, append, query, subscribe, migrations, enum serialization, row parsing.
- **`crates/forge-core/src/timeline.rs`** — domain types: `TimelineEvent`, `NewEvent`, `Actor`, `EventKind`, `TimelineFilter`.

**Invariants & gotchas**  
- Events are immutable once appended; `id` is append-only (rowid).
- Timestamps are truncated to microseconds at write time so round-trip equality holds; lexicographic string comparison on `ts` equals chronological order.
- `feature_slugs` is a JSON array stored as TEXT; filter queries use `json_each` for membership.
- The connection mutex serializes writes; lagged or slow subscribers may drop events (broadcast channel capacity 512).
- `before_id` must be strictly less-than for pagination (never `<=`) because `id` order equals time order.
- `enum_to_str` serializes enums via `serde_json::to_value` expecting a `String` variant; non-string serde representations break the contract.
