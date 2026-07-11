The changes are to the Tauri app's staleness detection system, not the daemon HTTP server. These files are part of a different feature. The daemon-http-server doc should remain unchanged since none of its key files or behavior was modified this turn.

---
# Daemon HTTP Server

## Purpose

Local Axum server bound to `127.0.0.1:0` serving a frozen route table for Claude Code hook ingestion (POST `/hooks/event`) and CodeForge query APIs (features, timeline, notes). Port written to `.codeforge/runtime/daemon.json` for discovery by MCP and hooks.

## How it works

- Binds to ephemeral port on daemon startup; writes `{port, pid, started_at}` manifest to `daemon.json`.
- Hook endpoint returns 200 immediately; ingestion happens async (classify paths → append timeline row → queue unclassified edits for reindex).
- Timeline/feature queries spawn blocking tasks (rusqlite behind a sync `Mutex`).
- Drains spooled hook payloads from `.codeforge/runtime/spool/` on startup (replayed forward in time); quarantines unparseable files to `rejected/`.
- Graceful shutdown removes `daemon.json` and aborts doc-refresh listener within 5s.
- Permissive CORS layer to allow web UIs and MCP tools to reach it.

## Key files

- `crates/forge-daemon/src/http.rs` — route table (`POST /hooks/event`, `GET /api/features`, `GET /api/timeline`, etc.) and handlers.
- `crates/forge-daemon/src/lib.rs` — `Daemon::start` entrypoint, bind/serve/shutdown lifecycle, `DaemonDeps` shared state.
- `crates/forge-daemon/src/ingest.rs` — hook payload processing, classification, spool draining, reindex queueing.
- `crates/forge-daemon/tests/daemon_http.rs` — end-to-end tests against real axum + timeline store.

## Invariants & gotchas

- Route table is a **frozen contract** for MCP tools and forward.sh hooks; adding/removing a route is a breaking change.
- Hook endpoint must NEVER block Claude (200 `{}` returned before ingestion completes).
- All timeline/index reads via `spawn_blocking` (no `.await` on the store's sync methods).
- Unrecognized hook events logged at `debug`, not error — silence is intentional for forward compatibility.
- Unclassified file edits get `feature_slugs: []` but `payload.unclassified: true` so the UI can distinguish "empty" from "stale index" and trigger reindex.
- Spool drain happens BEFORE the router serves live requests (prevents out-of-order timeline).
- Unparseable spool files quarantined, never silently dropped (data loss is worse than `.../rejected/` clutter).
- Graceful shutdown has a 5s timeout backstop; aborts if axum doesn't drain.
---
