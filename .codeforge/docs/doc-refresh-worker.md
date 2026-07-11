The edited files this turn were all in `tauri-app` (staleness.rs, state.rs, repo_open.rs, frontend files), not in `forge-daemon/src/doc_refresh.rs`. The doc-refresh-worker feature itself was not touched. The existing living doc is already accurate to the current code.

Since no changes were made to this feature, the doc requires no updates.

---
# Doc Refresh Worker

## Purpose

Subscribes to timeline events and automatically regenerates living documentation for features when an agent session ends. Each `SessionEnded` event triggers parallel headless Claude passes to refresh docs for every feature touched during that turn.

## How it works

- Listens to the timeline broadcast channel for `SessionEnded` events
- On each session end, queries the most recent 1000 timeline events and slices out the ending session's turn (events between the previous session boundary and the terminating `SessionEnded`)
- Extracts all feature slugs from `FileEdited` events in the turn slice
- Queues each touched feature for refresh through a semaphore (limit 1 concurrent refresh) with deduplication (already-queued slugs are skipped)
- Spawns a headless `claude` process per feature to regenerate the living doc, passing the turn slice as context
- Records the refresh outcome (success or failure) as a `DocUpdated` timeline event, which broadcasts to the UI

## Key files

- **crates/forge-daemon/src/doc_refresh.rs** — core worker: `DocRefresher` (queue + semaphore), broadcast listener, turn-slicing logic
- **crates/forge-daemon/tests/doc_refresh.rs** — integration tests over real stores with injected refresh failures (nonexistent repo root)

## Invariants & gotchas

- **Pinned features are never auto-refreshed** — if `feature.pinned` is true, the doc is human-owned and the worker skips it
- **The semaphore limit is 1** — only one feature's doc can refresh at a time (each refresh may run for minutes)
- **Deduplication is by slug in the queue** — if a feature is already waiting, it won't be double-queued; but a turn ending *during* an active refresh will queue that slug again with the newer slice
- **Turn slicing stops at the session's previous boundary** — only events strictly between the prior `SessionEnded`/`SessionStarted` and the current `SessionEnded` are included, ensuring a second turn in the same session doesn't fold in the first turn's edits twice
- **Session-less `Note` events are included** — the `/api/notes` endpoint appends notes without a session, so they're kept in every overlapping turn's slice
- **Both success and failure are named states** — `DocUpdated` events land on the timeline with `outcome: "updated"` or `"failed"`; the UI renders both
- **Listener lag is logged but non-fatal** — if the broadcast buffer overflows, some `SessionEnded` events may be dropped (the worker logs missed count but cannot recover them)
- **Jobs are detached in a `JoinSet`** — each `SessionEnded` spawns an independent task so a long refresh never stalls the receiver into lagging; aborting the listener task aborts all in-flight jobs
