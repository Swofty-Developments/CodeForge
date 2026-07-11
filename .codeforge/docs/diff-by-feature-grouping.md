The core file hasn't changed. The edits this turn were to UI components (SessionPane, app-store) and indexing (headless.rs), none of which touch the grouping logic. The existing doc is still accurate.

---
# Diff-by-Feature Grouping

## Purpose

Organizes a flat list of `FileDiff`s into feature-keyed buckets for UI presentation. Files that belong to multiple features are cloned into each relevant group and marked `shared: true`; unclassified files land in a synthetic "unmapped" group.

## How it works

- For each file, call `classify(path)` to get zero or more feature slugs.
- If a file maps to multiple slugs, clone the `FileDiff` into each group and mark all those groups `shared: true`.
- Files with no slugs go into the synthetic `unmapped` bucket (`slug: "unmapped"`, `name: "Unmapped"`).
- Resolve each slug to a display name via `name_of(slug)`; fall back to the slug itself if no name exists.
- Sort groups by: unmapped flag (unmapped last), total changed lines descending, slug ascending.
- Return `DiffByFeature { groups }`.

## Key files

- `crates/forge-git/src/group.rs` — the `group_by_feature` entry point, bucketing logic, and sort order.

## Invariants & gotchas

- **Shared files are cloned**, not moved — the same `FileDiff` struct appears in multiple groups. Changes to one clone don't affect others unless you mutate a shared reference.
- **The unmapped bucket is always last**, even if it has more changed lines than every other group. The sort explicitly checks the `unmapped: bool` flag before comparing line counts.
- **`shared: true` means ANY file in the group belongs to multiple features**, not that every file does. A group with one single-feature file and one multi-feature file is marked shared.
- **Display name fallback**: if `name_of(slug)` returns `None`, the group's `name` field is the raw slug. The UI must handle kebab-case slugs gracefully.
- **Empty input → empty output**: `group_by_feature(vec![], …)` produces `DiffByFeature { groups: vec![] }`, not a lone unmapped group.
