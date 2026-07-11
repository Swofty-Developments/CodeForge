---
# Feature Index Storage

## Purpose

In-memory feature index backed by `.codeforge/features.json` that handles load/save, pinning, path→feature classification, and re-index merges that preserve user edits. Supports staleness detection via a content-hash manifest (`.codeforge/index-meta.json`) plus a background poller so the UI can prompt re-indexing only when needed.

## How it works

- **Load/save**: `FeatureIndex::load` reads `.codeforge/features.json` (missing file → empty index); `save` writes atomically (tmp + rename) with features sorted by slug for stable diffs.
- **Classification**: `classify_paths` maps changed file paths to feature slugs by scoring exact file matches (3 points), entry-point matches (4 points), or deepest shared directory prefix as a fallback. Features with no signal are excluded.
- **Pinning**: User-pinned features survive re-index verbatim; pinned files inside unpinned features survive the file-list replacement (same-path incoming entries are overwritten, missing ones re-appended).
- **Re-index merge**: `merge_reindex` replaces unpinned features by slug, drops unpinned features absent from the incoming index, and preserves all pinned state (both features and individual files).
- **Staleness**: `index-meta.json` stores the sha256 of every referenced file at index time; `meta::status` recomputes hashes to return `never | fresh | stale | outdated` without side effects. A 10s background poller per open repo (spawned on open, aborted on close) re-checks the verdict and emits `index:status` only when the state changes, so external edits (git pull, other editor) surface without reopening the repo. Polls are skipped while `reindexing` is true; the first post-reindex poll always emits (typically back to `fresh`).
- **Atomic writes**: Both `features.json` and `index-meta.json` use tmp + rename so readers never observe partial state.

## Key files

- **`lib.rs`** — `FeatureIndex` API: load, save, upsert, pin, `merge_reindex`, `classify_paths`, and `write_meta` for the staleness manifest.
- **`merge.rs`** — Re-index merge semantics: pinned features survive verbatim; pinned files survive inside unpinned features; unpinned features are replaced or dropped.
- **`classify.rs`** — Path→feature scoring: entry-point match (4), exact file (3), or deepest shared directory prefix as tiebreak.
- **`meta.rs`** — `IndexMeta` (sha256 manifest + version), `status` (pure staleness verdict), and atomic `write`.

## Invariants & gotchas

- **Pinned features are immutable during re-index**: `merge_reindex` discards incoming data for any pinned slug. Pinned `group` fields must survive, so the merge never overwrites them.
- **Pinned files inside unpinned features also survive**: same-path incoming entries are replaced with the pinned version; pinned files absent from the incoming list are re-appended.
- **Atomic writes are mandatory**: both `save` and `write_meta` use tmp + rename. Direct writes would expose half-written state to readers.
- **Staleness is pure**: `meta::status` reads existing files and re-hashes; it never runs `claude` or triggers a re-index. This lets the UI prompt the user without side effects.
- **Staleness poller memo**: the per-repo poller only emits when the `IndexStatus` verdict changed since the last emit (compared by `PartialEq`). A growing `changedFiles` list in a still-`Stale` state is considered different and is emitted (updates the status-bar count); the frontend only re-pops the `StaleModal` when `prev.state !== status.state`, so a dismissed modal isn't re-shown every tick.
- **Poller lifecycle**: spawned by `staleness::spawn_poller` on repo open, first tick immediate, aborted on close. The `JoinHandle` lives in `RepoRuntime.staleness_task`.
- **Missing files at `write_meta` time are omitted from the manifest** (logged, not tracked for staleness) — a deliberate rule, not a silent guess.
- **Features and file lists are always slug-sorted**: load, save, and merge all enforce `sort_by(|a, b| a.slug.cmp(&b.slug))` for stable JSON diffs.
- **`INDEX_VERSION` bump forces `outdated` state**: when the indexing technique changes (prompt, derivation, meta shape), increment `INDEX_VERSION` in `lib.rs` so existing indexes report `outdated` and the UI can prompt a re-index.
