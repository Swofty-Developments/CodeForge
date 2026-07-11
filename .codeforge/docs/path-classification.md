# Path Classification

## Purpose

Maps changed file paths to feature slugs by scoring exact matches and directory proximity. Powers real-time hook ingestion (classify touched files → index the features) and diff-by-feature grouping (cluster uncommitted hunks by feature).

## How it works

`classify(repo_root, features, paths)` returns feature slugs ranked by relevance:

1. **Exact match scoring** (summed across all input paths):
   - Entry-point hit: +4
   - File hit: +3

2. **Directory fallback** (when no exact match):
   - Deepest shared directory prefix between the changed path and any path in the feature acts as a tiebreak score (depth in components)

3. **Sort & dedupe**:
   - Sort by (exact score DESC, dir depth DESC, slug ASC)
   - Filter out features with zero signal (no exact match, no shared dir)

**Scoring example**: editing `src/auth/mod.rs` when `entry` feature declares it as an entry point (score 4), `file` feature lists it as a file (score 3), and `sibling` feature owns `src/auth/session.rs` (dir depth 2) → rank: `["entry", "file", "sibling"]`.

Absolute paths are relativized against `repo_root` before component comparison; absolute paths outside the repo return empty (unclassifiable).

## Key files

- `crates/forge-index/src/classify.rs` — pure path-scoring logic
- Called by `FeatureIndex::classify_paths` (index.rs) during hook ingestion and diff grouping

## Invariants & gotchas

- **Root-level files require exact match**: files at repo root share no directory by definition (`shared_dir_depth` returns 0 for files with `len < 2` components), so `README.md` won't classify against `Cargo.toml` even though both are root files.

- **Scores sum across paths**: editing both `src/a.rs` and `src/b.rs` when a feature owns both scores 6 (3+3), outranking a feature that owns only one (3).

- **Absolute paths outside repo are silently dropped**: `components()` returns empty for out-of-repo absolute paths; they contribute no signal and won't classify.

- **Directory depth is a tiebreak, not a primary signal**: if two features have the same exact-match score, the one with deeper shared directory prefix wins. But a single exact match (score 3) beats any directory fallback.
