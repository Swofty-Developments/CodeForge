# App Database

## Purpose

Global SQLite database at `~/.codeforge/codeforge.db` storing projects, sessions, conversations, and usage logs. Single source of truth for resume state, cold-start indexing decisions, and session history.

## How it works

- **WAL mode** with single connection behind `Arc<Mutex<Database>>` in Tauri managed state (pragmas: foreign keys on, synchronous normal, 5s busy timeout).
- **Idempotent migrations** via `CREATE ... IF NOT EXISTS` — no version table, runs on every open; future column adds guarded by `column_exists` PRAGMA.
- **Six tables** with cascading FK relationships: `repos` ← `threads` ← `messages`/`sessions`, plus standalone `settings` and `usage_logs`.
- **Resume support** via `sessions.claude_session_id` — populated on session_ready, queried by Rust to decide whether to resume or start fresh.
- **Upsert on open** — `upsert_repo` returns existing id or creates a new one; `last_opened_at` updated each time.
- **Indexing contract** — `repos.indexed_at` is `NULL` ⇔ never indexed; queried by cold-start flow to decide whether to run full index.

## Key files

- **crates/tauri-app/src/db.rs** — `Database` struct, opens connection, applies pragmas, wraps conn accessor.
- **crates/tauri-app/src/migrations.rs** — idempotent schema creation (`run_migrations`), `column_exists` guard for ALTER.
- **crates/tauri-app/src/queries.rs** — plain-function query layer (upsert, read, insert for repos/threads/sessions/messages/usage).

## Invariants & gotchas

- **Single connection only** — pragmas tuned for non-concurrent desktop use; multi-writer would need different synchronous/WAL config.
- **FK constraints on** — deleting a repo cascades to threads/sessions/messages; ensure Tauri command guards confirm before destructive ops.
- **`indexed_at` NULL semantics** — `None` means *never indexed*, not "stale" — cold-start flow relies on this distinction.
- **`claude_session_id` nullable** — only set after session_ready; `list_past_sessions` filters `WHERE claude_session_id IS NOT NULL` so unready rows don't surface as resumable.
- **RFC3339 timestamps** — all `*_at` columns stored as strings (`chrono::Utc::now().to_rfc3339()`); parsing required for date math.
- **No soft deletes** — cascade-delete is immediate; recovery would need WAL replay or pre-delete backup.
- **Migrations are append-only** — never alter existing `CREATE` blocks; add new columns via `column_exists` guard below the batch.
