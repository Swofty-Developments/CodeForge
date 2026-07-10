# App Database (app-db)

## Purpose

Single SQLite database at `~/.codeforge/codeforge.db` tracking repos, threads, messages, sessions, settings, and usage logs. Serves as the desktop app's persistence layer for session history, resume capability, and cross-repo state.

## How it works

- **WAL journal mode + foreign keys ON** — configured via pragmas on every open; safe for concurrent reads, single-writer desktop use.
- **Idempotent migrations** — `CREATE ... IF NOT EXISTS` for all schema objects; runs on every open without version tracking (see `column_exists` guard pattern for future column additions).
- **Repo upsert** — when a repo is opened, `upsert_repo` creates or updates its row and returns the id; `indexed_at` column (nullable) signals whether cold-start indexing ran.
- **Session resume** — `sessions.claude_session_id` stores the SDK session identifier; `claude_session_exists` checks whether a prior session is resumable.
- **Query layer** — plain functions in `queries.rs` taking `&Connection`; no ORM, minimal row mappers.
- **Managed state** — single `Database` instance wrapped in `Arc<Mutex<Database>>` in Tauri managed state; commands lock, run query, unlock.

## Key files

- **`crates/tauri-app/src/db.rs`** — `Database` struct, pragmas, single-connection open, in-memory test constructor.
- **`crates/tauri-app/src/migrations.rs`** — idempotent schema creation (`CREATE ... IF NOT EXISTS`), index creation, `column_exists` guard helper.
- **`crates/tauri-app/src/queries.rs`** — plain-function query layer (settings, repos, threads, sessions, messages, usage_logs).

## Invariants & gotchas

- **Migrations MUST be idempotent** — they run unconditionally on every open; `CREATE ... IF NOT EXISTS` for new tables/indexes, `column_exists` guard for column additions.
- **Foreign keys are ON** — deleting a repo cascades to threads → messages/sessions → usage_logs; test this when altering schema.
- **Single connection, single writer** — the `Arc<Mutex<>>` wrapper enforces one query at a time; long-running queries block Tauri commands.
- **`repos.indexed_at` is the cold-start oracle** — `None` means never indexed; don't write this column outside the indexing flow.
- **RFC3339 timestamp strings** — all `*_at` columns store `chrono::Utc::now().to_rfc3339()`; parsing is caller's responsibility.
- **Test with in-memory DB** — `Database::open_in_memory()` skips WAL pragmas (not supported for `:memory:`), retains foreign keys ON.
