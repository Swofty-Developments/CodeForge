# Hook Ingestion

## Purpose

Receives Claude Code hook payloads from `forward.sh` (either via live HTTP or replayed from disk spool), classifies edited files into features, appends timeline events, and queues unmatched paths for re-indexing.

## How it works

- `parse_hook_event` maps raw JSON to timeline event kinds: `PostToolUse` → `FileEdited` (Edit/Write/MultiEdit/NotebookEdit) or `CommandRun` (Bash); `Stop` → `SessionEnded`; `SessionStart` → `SessionStarted`.
- Edited file paths are relativized to the repo root and classified via the in-memory index; if classification returns empty slugs, the event is marked `unclassified: true` and each path is queued on `ReindexQueue` so a later reindex can resolve it.
- `process_hook` appends the event to the timeline (which broadcasts to live subscribers) and enqueues unmatched paths, both operations offloaded to `spawn_blocking` since they hit rusqlite.
- On daemon start, `drain_spool` replays payloads from `.codeforge/runtime/spool/` in sorted order, then deletes each; unparseable files are quarantined to `spool/rejected/` for inspection.
- Non-file-scoped events (Bash, Stop, SessionStart) have empty `edited_paths` and skip classification — `feature_slugs` is correctly empty, not unclassified.

## Key files

- **`crates/forge-daemon/src/hooks.rs`** — parses raw hook JSON into `ParsedHook` (event kind, payload, edited paths).
- **`crates/forge-daemon/src/ingest.rs`** — `process_hook` (classify → append → enqueue) and `drain_spool` (replay spooled payloads on startup).

## Invariants & gotchas

- **Three classification states must remain distinct**: no file edits (empty slugs is correct), classified to features (tagged normally), classified to nothing (marked `unclassified: true` and queued). Collapsing "classified to nothing" into "empty slugs" silently loses the reindex trigger.
- **Paths are relativized once** before classification and enqueue; the index and reindex queue expect repo-relative paths, but hooks send absolute ones.
- **Unparseable spooled payloads are quarantined, not deleted** — the rejected spool directory preserves lost data for inspection rather than reprocessing it forever or dropping it silently.
- **Blocking I/O is offloaded** to `spawn_blocking` since both timeline append and reindex enqueue hit rusqlite; running them on the async runtime would block the executor.
