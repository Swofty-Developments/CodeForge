The edited files are about index staleness detection, not diff grouping. The feature "Diff by Feature Grouping" was not modified this turn — its living doc should remain unchanged.

---
# Diff by Feature Grouping

## Purpose

Groups file diffs by feature via index-based path classification. Files belonging to multiple features are duplicated across each group (marked `shared: true`); files that match no feature land in a synthetic "Unmapped" group at the end.

## How it works

- `group_by_feature` takes a list of `FileDiff`s and two callbacks: `classify` (path → feature slugs) and `name_of` (slug → display name).
- For each file, `classify` returns 0+ feature slugs. Empty = unmapped; 2+ = shared.
- Files in multiple features are cloned into each bucket; `shared: bool` is set to true for any bucket containing at least one multi-feature file.
- Groups are sorted by total changed lines (additions + deletions) descending, with the unmapped bucket always last (distinguished by the typed `unmapped: true` flag, not by string sniffing).
- The display name falls back to the slug if `name_of` returns `None`.
- `DiffByFeature { groups: Vec<FeatureDiffGroup> }` is the IPC contract returned to the frontend diff review view.

## Key files

- **crates/forge-git/src/group.rs** — `group_by_feature` pure function; receives classifier callbacks from caller.
- **crates/forge-core/src/diff.rs** — domain types (`DiffByFeature`, `FeatureDiffGroup`, `FileDiff`, `DiffHunk`, `DiffLine`); `#[serde(rename_all = "camelCase")]` for Tauri IPC.

## Invariants & gotchas

- **Unmapped is always last** — the sort explicitly checks `unmapped: bool` before changed-line count; don't sniff `slug == "unmapped"`.
- **Shared files are cloned, not moved** — a file in 3 features appears verbatim in 3 groups; the UI must handle duplicate rendering.
- **`shared: true` iff any file in the bucket belongs to 2+ features** — it's a bucket-wide flag, not per-file.
- **Display name fallback** — `name_of` returning `None` silently falls back to the slug; missing metadata won't crash grouping.
- **Classify is caller-supplied** — `group.rs` is pure; the caller wires up the index lookup (see entry point `crates/forge-git/src/group.rs:13`).
---
