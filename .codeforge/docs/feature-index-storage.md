The edited files are all in the frontend (app-store.ts, SessionPane.tsx, styles-chrome.ts) and forge-index/headless.rs. The headless.rs changes add retry logic for transient failures, but this doesn't affect the core feature-index-storage design — it's about headless claude invocation, not the storage model itself. No changes were made to the core index load/save/merge/pin/patch logic.

The existing doc is accurate and complete. No updates needed.

---
# Feature Index Storage

## Purpose

In-memory feature index backed by `.codeforge/features.json` that handles load/save/upsert operations and merge-reindex semantics (preserving human edits during automated re-indexing). JSON is always pretty-printed and slug-sorted for stable diffs and git-friendly commits.

## How it works

- **Load** reads `features.json` into a slug-sorted `Vec<Feature>`; missing file yields an empty index (fresh repo).
- **Save** writes pretty-printed JSON via atomic rename (`json.tmp` → `json`) so readers never observe half-written state; trailing newline included.
- **Upsert** replaces an existing feature by slug (or appends + re-sorts) and bumps `updated_at`.
- **Merge-reindex** preserves pinned features verbatim, carries pinned files forward into unpinned features, drops unpinned features absent from incoming, and appends new slugs — stable merge contract for human-vs-AI editing.
- **Pin/patch** operations always bump `updated_at`; patching a feature implies pinning it.
- **Meta-file** (`.codeforge/index-meta.json`) stores the content-hash manifest + `INDEX_VERSION` so the UI can detect stale indexes without re-running `claude`.

## Key files

- **`crates/forge-index/src/lib.rs`** — `FeatureIndex` struct: load/save/upsert/pin/patch/merge-reindex public API and storage I/O.
- **`crates/forge-index/src/merge.rs`** — `merge()` implements pinned-feature-survival + pinned-file-carry semantics for re-index operations.

## Invariants & gotchas

- **Atomic write** — save uses temp-file rename so concurrent readers never see partial JSON.
- **Slug stability** — features are always slug-sorted on save and after upsert; in-memory order matches on-disk order.
- **Pinned feature verbatim** — merge-reindex discards incoming version of a pinned feature entirely (name, files, group, everything).
- **Pinned file survival** — pinned files inside unpinned features overwrite same-path incoming entries or are re-appended if missing from incoming.
- **Editing implies pinning** — `apply_patch` always sets `feature.pinned = true`; UI must warn users that edits will survive re-index.
- **No doc deletion** — `merge_reindex` is purely in-memory; `.codeforge/docs/<slug>.md` files for dropped slugs are never deleted (orphan docs survive).
- **Meta-file sync** — after successful index/save, call `write_meta()` so `index_status()` returns `IndexStatus::Fresh`; skipping it leaves the index flagged as stale.
