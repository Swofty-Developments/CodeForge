The edited files are backend Tauri IPC command modules, headless indexing configuration, and main.rs bootstrap code. None of them touch the sidebar's tree rendering, activity badge logic, collapse state, or pin functionality. No changes needed to the living doc.

---
# Feature Tree Sidebar

## Purpose

Renders a nested, collapsible tree of features grouped by slash-delimited hierarchies (e.g., `backend/forge-index`), with activity badges showing unseen timeline events and persistent pin indicators. Drives feature navigation and provides the primary structural view of the codebase.

## How it works

- **Tree construction**: `buildFeatureTree` splits each feature's `group` field on `/` and nests them into a recursive `FeatureTreeNode` structure; features with no group are returned separately and rendered under an "Ungrouped" header only when real groups exist (otherwise the list stays flat).
- **Activity tracking**: On repo open, each feature's "seen" watermark is baselined to the current max timeline event id; selecting a feature advances its watermark, so the badge counts only events that land *after* the feature was last viewed.
- **Sorting**: Within each group or ungrouped bucket, features sort by most-recent timeline activity (falling back to `updatedAt`), then alphabetically by name.
- **Collapse state**: Group expand/collapse lives in a sidebar-local store (`collapse.ts`) keyed by the full slash path; defaults to expanded.
- **Entrance animation**: Each `FeatureRow` staggers its `fade-slide-up` animation via inline `animation-delay` derived from its index.
- **Pin indicators**: The amber pin icon is always visible when pinned, otherwise revealed on row hover; clicking it toggles the feature's pinned state via `appStore.pinFeature`.

## Key files

- **Sidebar.tsx** — root component, builds the tree, manages "seen" watermarks, renders group nodes or flat ungrouped list.
- **FeatureRow.tsx** — one feature row with name, activity badge (sky), and pin toggle (amber when pinned, hover-reveal otherwise).
- **GroupNode.tsx** — recursive group header with chevron, name, and subtree count; renders child groups and feature rows with left-border indent rail.
- **tree.ts** — `buildFeatureTree` logic: splits groups, nests into `FeatureTreeNode` structure, sorts by activity/name, separates ungrouped.
- **tree-styles.ts** — global CSS injection for group headers, chevron rotation, indent rail, collapse transition (grid-template-rows).
- **activity.ts** — derives `latestActivity` and `unseenCounts` from timeline events; stores per-feature "seen" watermarks; sorts features by activity.
- **collapse.ts** — sidebar-local store for group expand/collapse state, keyed by full slash path; defaults to expanded.
- **time.ts** — relative-time helpers (`formatAgo`, `createNow`), shared by sidebar and feature detail view.

## Invariants & gotchas

- **Watermark baseline timing**: The effect that baselines each feature's "seen" watermark to the current max event id must run *before* the user selects a feature, otherwise the badge will show stale counts from before the repo was opened.
- **Ungrouped render logic**: The "Ungrouped" header must appear only when real groups exist (`tree().roots.length > 0`), otherwise an all-ungrouped repo grows a spurious header. The fallback renders ungrouped features flat.
- **Collapse key uniqueness**: The synthetic "Ungrouped" bucket uses `"\0ungrouped"` (with a NUL prefix) as its collapse key to guarantee it never collides with a real group path.
- **Activity badge cap**: Badges cap at `99+` to avoid layout jank; the count is derived from `unseenCounts(store.timeline, seen)` and recomputed reactively as timeline events land.
- **Pin toggle event propagation**: `FeatureRow`'s pin button must `stopPropagation` to prevent the row's `onSelect` from firing when the user toggles the pin.
- **Entrance animation order**: `FeatureRow` indices must be stable within a render (derived from `<For>`'s index callback) so staggered entrance doesn't shuffle mid-animation.
- **Global style injection**: `injectTreeStyles` is guarded by `document.getElementById(STYLE_ID)` and called once from `Sidebar` to avoid duplicate `<style>` tags when the tree rerenders.
