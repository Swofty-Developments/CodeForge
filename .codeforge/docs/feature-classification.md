# Feature Classification

**Purpose**: Maps changed file paths to feature slugs via exact file match or nearest-directory heuristic. Used to group diffs by feature and decide which features to notify on edits.

## How it works

- **Entry-point match** scores `4` per path, **exact file match** scores `3` per path — scores sum across multiple paths.
- When no exact match exists, computes the deepest shared directory prefix (ignoring file names) between the changed path and any of the feature's known paths.
- Features with any signal (`exact > 0` or `depth > 0`) are returned best-first: by exact score, then shared-dir depth, then slug lexicographically.
- Absolute paths outside `repo_root` are ignored (unclassifiable).
- Features with zero signal are excluded from the result.

## Key files

- `crates/forge-index/src/classify.rs` (core) — `classify()` scoring logic, `components()` path normalization, `shared_dir_depth()` fallback heuristic.

## Invariants & gotchas

- **Entry points rank above files** — an entry point is a stronger signal (score 4 vs 3) and must win when both match the same path.
- **Fallback uses directory prefix only** — file names are excluded from the shared-dir computation, so root-level files need an exact match.
- **Scores sum across paths** — a feature that matches multiple changed files accumulates higher exact score; this is intentional to surface multi-file refactors.
- **Paths are normalized** — `components()` strips leading `..`, `.`, and repo-root prefix; absolute paths outside the repo return empty (no classification).
