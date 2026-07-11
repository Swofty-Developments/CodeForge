The session management changes (interrupt vs. stop, abort pipeline) are entirely scoped to SessionPane/Composer and don't touch the sidebar at all. The sidebar remains unaffected — no changes needed to the doc.

---
# Feature Tree Sidebar

## Purpose

Hierarchical sidebar that organizes features into a collapsible tree grouped by slash-delimited `group` metadata. Displays activity badges, pin icons, and live indexing progress, providing the primary navigation surface for the feature catalog.

## How it works

- `tree.ts` parses each feature's `group` field (e.g. `backend/forge-index`) into a nested `FeatureTreeNode` tree, sorting groups alphabetically and features by recent activity.
- `Sidebar.tsx` orchestrates the view: builds the tree via `buildFeatureTree`, tracks unseen activity per feature (watermark-based), and renders either a flat list (all ungrouped) or a tree with a synthetic "Ungrouped" bucket (when real groups exist).
- `GroupNode.tsx` recursively renders collapsible group headers (chevron + segment name + descendant count) with collapse state in a sidebar-local store; children nest under a left-border indent rail.
- `FeatureRow.tsx` renders each feature with a sky badge (unseen event count), hover-reveal pin toggle (amber when pinned), and staggered entrance animation (20ms per index).
- Activity badges baseline to the current timeline head on repo open, then clear when a feature is selected; the footer shows ONLY live indexing progress (`store.indexProgress` — stage/count/shimmer bar), while resting status (daemon health, index freshness, reindex prompt) lives in the bottom status bar.

## Key files

- **Sidebar.tsx** — root component, builds tree, tracks activity watermarks, handles selection/pin toggling
- **tree.ts** — `buildFeatureTree` algorithm: nests features by group segments, sorts alphabetically and by activity
- **GroupNode.tsx** — recursive collapsible group header with chevron, segment name, count, and nested children
- **FeatureRow.tsx** — feature row with name, activity badge, and pin icon
- **tree-styles.ts** — one-time style injection for group headers and the indent rail (id-guarded to prevent duplication)

## Invariants & gotchas

- Ungrouped features ONLY get a header when real groups exist — an all-ungrouped repo stays flat, otherwise the synthetic "Ungrouped" node appears via `ungroupedNode()`.
- Activity watermarks baseline to `maxEventId()` when the repo opens; `seen[slug]` must remain undefined until initialization completes or badges will over-count historical events.
- The synthetic "Ungrouped" node uses `UNGROUPED_KEY = " ungrouped"` (leading NUL char) as its path to guarantee no collision with real group paths.
- Collapse state lives in a sidebar-local `collapsedGroups` store (not the main app store), so it persists across rerenders but resets on repo change.
- Sidebar width is driven by `store.sidebarWidth` from the shell's resize handle; do not hard-code width in CSS or it will conflict with the drag-resize behavior.
- `tree-styles.ts` injects global styles once via `id=sidebar-tree-styles`; never inline those styles into `GroupNode` or recursion will duplicate them hundreds of times.
- The footer shimmer bar respects `prefers-reduced-motion` by falling back to a static gradient.
- The sidebar footer shows ONLY live indexing progress — staleness status (the `index:status` event from the background poller) drives the status bar dot and stale-index modal, not the sidebar footer.
