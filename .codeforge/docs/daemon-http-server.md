None of the edited files relate to the daemon HTTP server feature. The changes are to frontend components (SessionPane UI, app store) and the headless indexing runtime. The daemon HTTP server itself (`crates/forge-daemon/src/http.rs`, `lib.rs`) was not touched. The existing doc is still accurate — no changes needed.

---
# Daemon HTTP Server

## Purpose

Local per-repo HTTP server bound to `127.0.0.1:0` that receives Claude Code hook payloads and exposes a REST API for feature queries, timeline events, and file classification. Writes `daemon.json` with `{port, pid, started_at}` so clients can discover the ephemeral port.

## How it works

- Axum router with CORS enabled: seven frozen routes (`POST /hooks/event`, `GET /api/features`, `GET /api/features/{slug}`, `GET /api/timeline`, `POST /api/notes`, `GET /api/classify`, `GET /api/health`).
- `POST /hooks/event` ingests raw Claude Code hook JSON, classifies edited paths against the feature index, appends a timeline event, and enqueues unclassified edits for re-indexing — always replies 200 `{}` fast to never block Claude.
- On startup drains spooled hook payloads from `.codeforge/runtime/spool/` (files written by `forward.sh` when daemon was down) and replays them in sorted order before serving live requests.
- Relativizes absolute paths from hooks to repo-root-relative before classification so they match feature storage format.
- Timeline mutations (`append`, `query`) run on `spawn_blocking` because `TimelineStore` wraps rusqlite in a sync `Mutex`.
- Graceful shutdown removes `daemon.json`; 5s timeout before abort backstop.

## Key files

- **crates/forge-daemon/src/http.rs** — route table, handlers, JSON schemas for timeline/classify params and note bodies.
- **crates/forge-daemon/src/lib.rs** — `Daemon::start` binds the listener, writes `daemon.json`, spawns the doc-refresh worker, drains spool, and returns `DaemonHandle` with shutdown channel.
- **crates/forge-daemon/src/ingest.rs** — shared hook→timeline logic for live and spooled payloads; marks unclassified edits and enqueues them for reindex.

## Invariants & gotchas

- Route table is the frozen contract: adding/removing/renaming routes breaks `forward.sh` and MCP clients. Treat them like a public API.
- Hook handler must reply fast (no blocking I/O) to never delay Claude's turn; `process_hook` spawns `spawn_blocking` for DB writes.
- Unclassified edits (file-scoped but `classify_paths` → empty) are distinct from non-file-scoped events (Stop/SessionStart) — the former are tagged `unclassified: true` in payload and paths enqueued; the latter naturally have empty slugs and are not requeued.
- Spool replay is sorted so events land in temporal order; invalid JSON is quarantined to `spool/rejected/` not deleted so it is preserved for inspection.
- `daemon.json` port is ephemeral (`127.0.0.1:0`) so clients must read the file; it is removed on graceful shutdown but may linger if killed hard.
- Timeline/index methods run on `spawn_blocking` because rusqlite is sync; never await them on the async pool or axum worker threads stall.
