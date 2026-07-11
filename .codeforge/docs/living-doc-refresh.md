# Living Doc Refresh

## Purpose

Auto-updates feature living docs when a Claude session ends. Spawns headless `claude` to fold edited files, commands, and recorded notes into the feature's `.codeforge/docs/<slug>.md` — no manual rewrites, no staleness drift.

## How it works

- `DocRefresher` listens to timeline `SessionEnded` events (Stop hooks firing).
- Slices the ending turn's `FileEdited`, `CommandRun`, and `Note` events from the timeline (newest-first query, stops at the prior session boundary).
- For each feature slug touched by file edits, queues a doc refresh (deduped, serialized via semaphore).
- Spawns headless `claude -p --output-format json --allowedTools Read,Glob,Grep` in the repo root with a prompt containing: feature metadata, the existing doc (or "write first version" if missing), and the compact turn event summary.
- Parses the JSON envelope, strips fences, clamps to 60 lines, writes atomically (tmp + rename).
- Records the outcome (`updated` or `failed`) as a `DocUpdated` timeline event.

## Key files

- `crates/forge-daemon/src/doc_refresh.rs` — orchestration: timeline listener, turn slicing, queue/semaphore serialization.
- `crates/forge-index/src/refresh.rs` — core refresh logic: prompt builder, headless claude invocation, doc write.
- `crates/forge-index/src/headless.rs` — `claude` CLI wrapper with retries, timeouts, and JSON envelope parsing.

## Invariants & gotchas

- Pinned features are skipped — `pinned: true` means the doc is human-owned, same rule as cold-start indexing.
- The feature is snapshotted before the minutes-long `claude` call, never held under lock across await.
- Turn slice is NEWEST-FIRST → filter → reverse (chronological). Misreading the order produces wrong-session slices.
- `Note` events are session-less (`/api/notes` endpoint) but folded into any turn whose window covers them — this is intentional (recorded notes surface even when attached off-band).
- Queue-dedupe prevents double-queuing a slug already waiting, but once refresh starts the slug is removed from the queue — a session ending DURING refresh re-queues it with newer events (flywheel semantics).
- Success and failure both land on the timeline as `DocUpdated` — the UI shows both; silent failure is never correct.
