The edited files (sessions.rs and frontend components) are unrelated to index staleness detection. The staleness.rs file itself was not changed this turn. The doc remains accurate.

---
# Index Staleness Detection

## Purpose

Polls every open repository's index status every 10 seconds and emits `index:status` events when the verdict changes, alerting the UI when the on-disk index becomes stale or outdated. The detection is pure (never re-indexes) so the UI can prompt a re-index without side effects.

## How it works

- Each open repo spawns a background poller that computes index status off the async runtime via `tokio::task::spawn_blocking` (sha256 hashing is blocking IO).
- The staleness verdict has four states: `never` (no index yet), `fresh` (hashes + version match), `stale` (manifest files changed or vanished since indexing), or `outdated` (stored version < current `INDEX_VERSION`).
- A poller ticks every 10 seconds, skipping entirely while `reindexing` is set (mid-reindex the manifest is being rewritten) and resetting `last` so the first post-reindex verdict is always re-emitted.
- Emits `index:status` `{ repoPath, status }` only when the verdict **changed** since the last emit—a steady state stays silent so the UI isn't re-prompted every tick.
- The staleness check re-hashes every file in `.codeforge/index-meta.json` (a content-hash manifest + version written at index time) and compares against the stored sha256; mismatches or missing files → `stale`, version mismatch → `outdated`.
- Probe failures are logged but do not fabricate a fallback verdict—`IndexStatus` has no error state—and the poller keeps running (the repo IS open).

## Key files

- **crates/tauri-app/src/runtime/staleness.rs** — spawns the per-repo poller, emits `index:status` events when the verdict changes.
- **crates/forge-index/src/meta.rs** — pure staleness/version computation (`status()`), reads `features.json` + `index-meta.json`, re-hashes manifest files, and writes the meta manifest at index time (`write()`).
- **crates/forge-index/src/lib.rs** — defines `INDEX_VERSION` (bumped when indexing technique changes) and the `index_status` public API.

## Invariants & gotchas

- **Never fabricate a verdict** — probe failures are logged, not mapped to "fresh"/"never" (no error state exists).
- **Poller must not outlive its repo** — the returned `JoinHandle` is owned by `RepoRuntime` and aborted on context close.
- **Emit-on-change only** — comparing `last` against the new verdict prevents flooding the UI with duplicate status events.
- **Skip polling mid-reindex** — while `reindexing` is set, the manifest is being rewritten so a verdict against old meta is noise; `last` is reset so the post-reindex verdict is always re-emitted (typically back to `fresh`).
- **Version mismatch takes precedence** — `outdated` is returned before checking hashes; bump `INDEX_VERSION` when the indexing technique changes.
- **Manifest omits missing files** — a file referenced by a feature but absent at `write_meta()` time is omitted from the manifest (logged) and not tracked for staleness (documented rule, not a guess).
- **Frontend filters by `repoPath`** — the `index:status` event includes `repoPath` so multi-repo UIs can route the prompt to the right worktree.
