The agent's work this turn didn't touch `forge-core` domain types at all — it refactored the Tauri runtime layer's staleness polling. The domain model (the schema crate) is unchanged.

---
# Domain Model

## Purpose

Pure, IO-free schema crate defining every domain type that crosses the Tauri IPC boundary between Rust backend and SolidJS frontend — features, timeline events, diffs, sessions, worktrees, and errors.

## How it works

- All structs serialize as `camelCase` JSON via `#[serde(rename_all = "camelCase")]` for TypeScript interop.
- Enums (`FileRole`, `Actor`, `EventKind`, `SessionStatus`) serialize as `snake_case` strings to avoid magic numbers.
- Every type is mirrored 1:1 in `frontend/src/types.ts` and validated by roundtrip serde tests.
- Feature files live in a many-to-many relationship: one file can participate in N features.
- Timeline events are immutable, append-only records with SQLite rowids as stable ids; `TimelineFilter` implements backward-paging via `before_id`.
- Diff grouping is feature-centric: files appear under every feature they belong to (`shared: true`), with unmapped files isolated under a synthetic group (`unmapped: true`).

## Key files

- `lib.rs` — public re-exports and IPC serde contract documentation.
- `feature.rs` — `Feature`, `FeatureFile`, `FileRole`, `FeaturePatch`; the primary unit CodeForge organises code by.
- `timeline.rs` — `TimelineEvent`, `Actor`, `EventKind`, `TimelineFilter`; append-only event log schema.
- `diff.rs` — `DiffByFeature`, `FeatureDiffGroup`, `FileDiff`, `DiffHunk`, `DiffLine`; multi-level diff view data.
- `session.rs` — `RepoState`, `SessionInfo`, `SessionStatus`, `StartSessionOpts`, `IndexProgress`; session lifecycle types.
- `worktree.rs` — `Worktree`, `MergeResult`; git worktree tracking and merge outcomes.
- `error.rs` — `Error`, `Result`; shared error enum wrapping io/json/notfound/invalid/other.

## Invariants & gotchas

- **Never break serde contract**: all field renames must be coordinated with `frontend/src/types.ts` and the IPC call sites — a silent mismatch causes runtime deserialization panics.
- **Enums are stringly-typed**: `FileRole::Core` serializes to `"core"`, not `0`; the frontend expects exact string matches.
- **Unmapped files must set `unmapped: true`**: the frontend distinguishes the synthetic unmapped group by that boolean field, NOT by string-sniffing `slug == "unmapped"`.
- **Timeline ids are append-only rowids**: never mutate `TimelineEvent.id`; `before_id` paging depends on the rowid == insert-order invariant.
- **Optional fields serialize null or are omitted**: `FeaturePatch`, `StartSessionOpts`, and `TimelineFilter` use `#[serde(skip_serializing_if = "Option::is_none")]` — receiving `{}` must parse as `Default::default()`.
