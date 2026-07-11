Confirmed: the changes are about session management (rename/resume/auto-title), not the color picker. The feature color picker code is unchanged.

---
# Feature Color Picker

## Purpose

Provides a user-settable graph node color for features. Users can override the default palette by assigning a semantic hex color (#RGB or #RRGGBB) to any feature, which persists across re-indexes and is reflected in the sidebar, graph, and timeline lanes.

## How it works

- Each `Feature` has an optional `color: Option<String>` field — `None` uses the default palette, `Some("#hex")` overrides it with a user-picked color.
- The `set_color` mutation validates the hex string (#RGB or #RRGGBB format), rejects invalid colors with a named error, and prevents phantom features (unknown slugs fail loudly; no silent upserts).
- Setting a color implies a pin (`pinned: true`) — user-edited colors survive re-indexing verbatim.
- The mutation persists the index to disk and appends a `FeatureEdited` timeline event with the color payload.
- The Tauri command `set_feature_color` wraps the mutation, acquires the index write lock, and ensures timeline durability (a failed append is a caller error, not a swallowed log line).
- `None` clears the color, reverting to the default palette; the pin remains.

## Key files

- **`crates/tauri-app/src/runtime/feature_color.rs`** — core mutation logic (`set_color`), validation, unit tests against a real `FeatureIndex`.
- **`crates/tauri-app/src/commands/features.rs`** — Tauri command wrapper (`set_feature_color`) that acquires locks, invokes the mutation, and appends the timeline event.
- **`crates/forge-core/src/feature.rs`** — defines `Feature.color` field (hex string or `None`).

## Invariants & gotchas

- **Validation is strict** — only `#RGB` or `#RRGGBB` hex strings (case-insensitive ASCII hexdigits) or `None` are allowed; bad colors fail the mutation, not silently stored.
- **No phantom features** — setting a color on an unknown slug is a named error; the mutation never creates a feature by side-effect.
- **Pin on edit** — setting or clearing a color always sets `pinned: true`; the user's color choice is treated as a human override that must survive re-index.
- **Timeline append is transactional** — a failed timeline append (after persisting the color) surfaces to the caller as an error, not a silent skip; the color is on disk but the audit log is incomplete.
- **Clearing a color** — `set_color(…, None)` removes the override but leaves the feature pinned; unpinning requires a separate `pin_feature(…, false)` call.
