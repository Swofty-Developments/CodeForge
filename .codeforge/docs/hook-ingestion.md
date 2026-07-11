# Hook Ingestion

**Purpose**

Receives Claude Code hook payloads (PostToolUse, Stop, SessionStart) forwarded from `.codeforge/hooks/forward.sh`, converts them to timeline events, classifies edited file paths into feature slugs, appends the event to the timeline (which broadcasts to the frontend), and replays spooled payloads on daemon startup.

**How it works**

- `forward.sh` sends hook JSON (session ID, event name, tool name/input) to the daemon HTTP server
- `parse_hook_event` maps recognized events to `EventKind` (FileEdited, CommandRun, SessionStarted, SessionEnded) and extracts edited paths from Edit/Write/MultiEdit/NotebookEdit tool calls
- Edited paths are relativized to the repo root and classified into feature slugs via the index
- File edits that classify to nothing (new/renamed files, stale index) are marked `unclassified: true` and their paths are queued for reindex
- On startup, `drain_spool` replays payloads from `.codeforge/runtime/spool/` in sorted order, deleting each after processing; invalid JSON is quarantined to `spool/rejected/`
- Non-file events (Bash, Stop, SessionStart) skip classification and always have empty feature slugs

**Key files**

- `crates/forge-daemon/src/hooks.rs` — parses raw hook JSON into `ParsedHook` (event kind + edited paths)
- `crates/forge-daemon/src/ingest.rs` — processes hooks (relativize, classify, append to timeline, enqueue for reindex), drains startup spool

**Invariants & gotchas**

- Hook processing is spawn_blocking because timeline append and reindex enqueue both hit SQLite synchronously
- Unclassified file edits MUST enqueue the path for reindex (otherwise new files never appear on the timeline under a feature)
- Paths in hook payloads are absolute; the index stores repo-relative paths — `relativize` before classify
- The three states (non-file, classified, unclassified) must remain distinct; collapsing "unclassified" into "empty slugs" breaks the reindex queue
- Spool files are processed in sorted order so event ordering is preserved across daemon restarts
- Unrecognized hooks (PreToolUse, Read tool) return `None` from `parse_hook_event` and are logged at debug but still ack'd with HTTP 200
