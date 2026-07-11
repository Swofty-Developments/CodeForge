# Feature Validation

## Purpose

Transforms Claude's raw JSON feature output into validated `Feature` structs by sanitizing slugs, filtering out non-existent or out-of-repo files, clamping paths to repo-relative form, and deriving multi-level groups from shared directory prefixes when the model doesn't supply an explicit group.

## How it works

- **Parse with recovery**: `parse_features` strips markdown fences and parses JSON; if the model wraps the array in prose, it recovers the outermost `[...]` slice and logs a warning.
- **Path validation**: `clamp_path` canonicalizes each file path, rejects `../` escapes or paths outside the repo, verifies the file exists and is a regular file, then returns a repo-relative `PathBuf` or `None`.
- **Slug deduplication**: `sanitize_slug` lowercases, strips non-alphanumeric chars (collapsing runs to single `-`), and `validate_features` drops duplicate slugs (keeping the first).
- **Empty feature rejection**: Features left with no valid `entry_points` *and* no valid `files` after path filtering are dropped entirely.
- **Group assignment**: `resolve_group` uses the model's non-blank `group` field verbatim (whitespace/slash-trimmed), or falls through to `derive_group` which walks the shared directory prefix of all files, skips monorepo containers (`crates/`, `packages/`) in favor of their child (the crate/package name), drops structural noise (`src/`), and returns up to 3 meaningful segments (e.g., `crates/atomix-web/src/pages/markets/list.tsx` → `"atomix-web/pages/markets"`). Files with no shared directory derive `None` → "Ungrouped".
- **Role parsing**: `parse_role` maps the model's role string to `FileRole::{Core, Support, Test, Config, Unknown}`. Absent/blank → `Support`; unrecognized non-blank → `Unknown` (never conflated with `Support`).

## Key files

- **`crates/forge-index/src/parse.rs`** — parsing, validation, path clamping, multi-level group derivation, slug/role sanitization.

## Invariants & gotchas

- **Canonicalization is security-critical**: `clamp_path` must canonicalize before checking `starts_with(canonical_root)` to reject symlink escapes and `../` tricks; skipping it would allow indexing files outside the repo.
- **Group derivation is a NAMED rule, not a fallback**: when the model supplies a blank `group`, that triggers derivation; a present non-blank value is used as-is. The derivation walks the *shared* prefix of *all* files (entry_points ∪ files), so adding one file from a different tree can collapse the group to `None`.
- **Empty features are dropped silently**: a feature with valid JSON but no paths that resolve to existing files vanishes from the index (logged at `warn` level).
- **Duplicate slugs keep the first**: the second occurrence is dropped, not merged.
- **`Support` vs `Unknown` are distinct states**: a missing role is deliberate "unspecified" (`Support`); a non-empty unrecognized role is `Unknown` (logged). Do not conflate them.
- **Monorepo containers are skipped, not kept**: `crates/x/src/y.rs` derives to `"x"` (or `"x/..."` if deeper structure exists), never `"crates"` or `"crates/x"`. Adding a new container name to `CONTAINER_DIRS` changes derivation for all features in that subtree.
- **Depth cap at 3 meaningful segments**: deeper nesting is truncated (`a/b/c/d/e/f.rs` → `"a/b/c"`). Raising `MAX_GROUP_DEPTH` changes the sidebar's tree structure.
