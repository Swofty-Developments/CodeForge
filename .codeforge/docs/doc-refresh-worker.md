The edited files (`headless.rs` and the frontend files) don't affect the doc-refresh-worker feature. The changes were to add retry logic in `headless.rs`, which `doc_refresh.rs` calls but doesn't need to document — that retry behavior is transparent to the caller. The frontend files are unrelated to this backend feature.

The existing doc is accurate and current.

---
# Doc Refresh Worker

## Purpose

Listens to the timeline broadcast for `SessionEnded` events, extracts the turn's file edits, spawns a living-doc refresh for each touched feature (via headless `claude`), and records success/failure as `DocUpdated` timeline events — the product flywheel that keeps docs current.

## How it works

- `DocRefresher::new` builds a semaphore-of-1 serializer and a dedupe queue keyed by slug.
- `spawn_listener` listens to the timeline broadcast; each `SessionEnded` spawns a detached job in a `JoinSet` so long refreshes never stall the receiver.
- `on_session_ended` queries the last 1000 timeline events, slices out one turn (events between this `SessionEnded` and the prior session boundary), extracts touched slugs from `FileEdited` events.
- `refresh_one` dedupes against `queued`, acquires the semaphore (serialize), skips pinned features (human-owned), shells out to `forge_index::refresh_feature_doc`, appends a `DocUpdated` event with `"updated"` or `"failed"` outcome.
- `turn_slice` reconstructs one session's turn from a newest-first event list: keeps `FileEdited`/`CommandRun` for the session plus any `Note` (session-less) in the window, stops at the nearest prior `SessionStarted`/`SessionEnded`.
- Aborting the listener drops the `JoinSet`, which aborts in-flight tasks; `kill_on_drop` in `forge-index` reaps running `claude` children.

## Key files

- **crates/forge-daemon/src/doc_refresh.rs** — refresher, listener, turn-slicing logic, `SessionEnded` reactor.

## Invariants & gotchas

- **Semaphore-of-1 serialize**: only one living-doc refresh runs at a time (each shells out for up to minutes) — never increase the permit count.
- **Dedupe window**: a slug already `queued` is not double-queued; once processing starts (`queued.remove`), newer turns CAN re-queue the slug before the first refresh finishes — exactly the flywheel semantics (most recent turn wins).
- **Pinned features skipped**: same rule as `Indexer::write_feature_docs` — pinned docs are human-owned, never machine-rewritten.
- **turn_slice NEWEST-FIRST input**: the store query returns newest-first; `turn_slice` reverses the slice before returning (chronological) — callers must not assume input order.
- **SESSION_SLICE_LIMIT of 1000**: a turn longer than this truncates old events (logged) but never bleeds into another session's events. The limit is per-query, not per-session.
- **JoinSet abort cascade**: aborting the listener task drops the `JoinSet` (aborts jobs), which aborts the `refresh_one` future, which cancels the `claude` child via `kill_on_drop` in `forge-index`. The kill cascade is Tokio-driven, not SIGTERM-trapped.
- **Lag tradeoff**: detached jobs (never awaited in `listen`) prevent backpressure lagging, but the receiver can still lag if `select!` processing itself is slow — very rare, but the lag arm logs it.
