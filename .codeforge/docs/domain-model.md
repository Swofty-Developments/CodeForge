# Domain Model

## Purpose
Pure Rust data structures for every entity that crosses the Tauri IPC boundary (Rust backend ⇄ SolidJS frontend). Defines the serde contract for `Feature`, `TimelineEvent`, `DiffByFeature`, `SessionInfo`, and other core types. No IO or business logic—only shape and serialization.

## How it works
- Every struct is `#[serde(rename_all = "camelCase")]` so Rust `snake_case` fields become `camelCase` in JSON for TypeScript consumption.
- Enums (`FileRole`, `Actor`, `EventKind`, `SessionStatus`) serialize as `snake_case` strings (e.g. `"file_edited"`, not `"FileEdited"`).
- Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` to omit them from JSON when unset, enabling partial payloads like `FeaturePatch` and `TimelineFilter`.
- All shapes are mirrored 1:1 in `frontend/src/types.ts`; this crate is the single source of truth for the contract.
- Test coverage ensures serialized field names match expectations (`"entryPoints"`, `"featureSlugs"`, etc.) and roundtrip correctness.
- No domain logic lives here—validation, IO, and state management belong in consuming crates (`forge-index`, `forge-timeline`, `forge-git`).

## Key files
- `lib.rs` — re-exports all public types, documents the IPC serde contract
- `feature.rs` — `Feature`, `FeatureFile`, `FileRole`, `FeaturePatch`
- `timeline.rs` — `TimelineEvent`, `Actor`, `EventKind`, `TimelineFilter`
- `session.rs` — `SessionInfo`, `SessionStatus`, `StartSessionOpts`, `RepoState`, `IndexProgress`
- `diff.rs` — `DiffByFeature`, `FeatureDiffGroup`, `FileDiff`, `DiffHunk`, `DiffLine`
- `worktree.rs` — `Worktree`, `MergeResult`
- `error.rs` — `Error`, `Result` (thiserror-based, serialized as strings when crossing IPC)

## Invariants & gotchas
- **Never add IO or business logic** to this crate—it's pure data. If a type needs behavior, that lives in the crate that owns the domain (e.g. `forge-index` for feature derivation).
- **Never change field casing conventions** (`camelCase` structs, `snake_case` enums)—the frontend TypeScript types depend on this exact shape.
- **Optional fields must skip serialization when `None`**—commands like `update_feature` rely on absence meaning "don't change this field," not "set it to null."
- **Enums must remain string-serialized**—the frontend pattern-matches on `"file_edited"`, not `{ "FileEdited": null }`.
- **Frontend types are manually maintained**—changing a field name or adding a variant here requires a matching edit in `frontend/src/types.ts` or the UI breaks at runtime.
- **`TimelineEvent.id` is append-only SQLite rowid**—never mutated, used for pagination cursors (`before_id`). Ordering by `id` is equivalent to ordering by `ts`.
- **`Feature.files` is many-to-many**—a file can appear in multiple features; the `shared: true` flag in `FeatureDiffGroup` signals this.
- **`FeaturePatch` default is empty, not all-`None`**—`serde(default)` on the struct means `{}` deserializes to `FeaturePatch::default()`, enabling minimal JSON payloads.
- **`DiffLine.origin` is a single-char string** (`'+'`, `'-'`, `' '`)—serde serializes it as `"+"`, not as a character code.
- **`Worktree.is_base` distinguishes the main checkout**—the base is also a worktree; every repo open in the UI has at least one (the base).
