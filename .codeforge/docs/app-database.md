The changes to `queries.rs` added new query functions (`list_past_sessions`, `update_thread_title`, `PastSession` struct) for session resumption. The notes mention a sidecar streaming fix, which doesn't touch the database schema or query layer — the doc is mostly accurate. I'll fold in the new query surface.

---
# App Database

## Purpose

Single SQLite file at `~/.codeforge/codeforge.db` persisting all app-level state: repos, conversation threads, messages, Claude SDK sessions, settings KV, and token usage logs. WAL mode with one connection per Tauri app instance.

## How it works

- `Database::open` connects to disk, sets WAL journal mode + foreign keys + performance PRAGMAs, then runs idempotent migrations on every open
- Migrations use `CREATE IF NOT EXISTS` for tables/indices; future column additions guard with `column_exists()` PRAGMA check before `ALTER TABLE ADD COLUMN`
- No version table or framework—migrations run unconditionally and must be re-runnable without error
- Tables enforce foreign-key cascades (`threads.repo_id → repos.id`, `messages.thread_id → threads.id`, `sessions.thread_id → threads.id`)
- Query layer (`queries.rs`) exposes plain functions taking `&Connection`:
  - Upsert repo on open
  - Insert thread/session/message rows
  - Update `sessions.claude_session_id` when SDK session is ready
  - `list_past_sessions(repo_id)` — all resumable sessions (those with a `claude_session_id`) for a repo, ordered by `threads.updated_at DESC`; live sessions must be filtered client-side
  - `update_thread_title(thread_id, title)` — for session rename, bumps `threads.updated_at`
- `repos.indexed_at` is the single source of truth for cold-start vs incremental index decision

## Key files

- `crates/tauri-app/src/db.rs` — `Database` struct, connection initialization, WAL/pragma setup
- `crates/tauri-app/src/migrations.rs` — idempotent schema definitions, `column_exists` guard for column additions
- `crates/tauri-app/src/queries.rs` — query functions (upsert_repo, insert_thread, list_past_sessions, update_thread_title, get_setting, etc.)

## Invariants & gotchas

- Migrations must be idempotent—no migration framework, so `CREATE IF NOT EXISTS` and `column_exists` guards are mandatory
- Single connection behind `Arc<Mutex<Database>>` in Tauri managed state; do not open multiple connections (WAL allows concurrent readers but this app uses one connection for simplicity)
- Foreign keys are `ON DELETE CASCADE`; deleting a repo row drops all threads/messages/sessions for that repo
- `repos.indexed_at` being `NULL` means "never indexed"; presence means indexed (RFC3339 timestamp)
- `sessions.claude_session_id` is filled after session start (via `session_ready` event), not at insert time
- `list_past_sessions` returns ALL sessions with a `claude_session_id` for a repo, including live ones—caller must filter out currently running sessions from the SessionManager registry
- All timestamps are RFC3339 strings (`chrono::Utc::now().to_rfc3339()`)
- PRAGMAs are tuned for single-connection desktop use—do not disable `foreign_keys` or switch off WAL mode
