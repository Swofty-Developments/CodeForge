# Feature Sidebar

## Purpose

Renders the left sidebar with a hierarchical feature tree, pinning, unseen activity badges, collapse state per group, and sort-by-recent-activity. Establishes each feature's watermark at the current timeline head so badges count only activity that lands after the repo is opened, then clears the watermark on selection.

## How it works

- **Tree construction** — `buildFeatureTree` nests features by their slash-delimited `group` field (e.g. `backend/forge-index`), forming recursive `FeatureTreeNode` structures sorted alphabetically; ungrouped features are returned separately and rendered under an "Ungrouped" header only when real groups exist, else flat.
- **Activity badges** — `unseenCounts` counts timeline events newer than each feature's `seen` watermark (keyed by event id); selecting a feature calls `markSeen` to advance the watermark and clear the badge.
- **Collapse state** — `collapsedGroups` is a sidebar-local store keyed by a group's full slash path; absent means expanded, truthy means collapsed; toggling the chevron flips the bit.
- **Sorting** — features within a group (or ungrouped) are sorted by most recent timeline activity then name, using `latestActivity` to map each feature slug to its newest event timestamp.
- **Pinning** — clicking the pin icon (hover-reveal, amber when pinned) calls `appStore.pinFeature(slug, !pinned)`, which persists to the backend.
- **Entrance animation** — each `FeatureRow` sets `animation-delay: ${index * 20}ms` to stagger the `fade-slide-up` animation.

## Key files

- **Sidebar.tsx** — root component; builds the tree, derives activity counts, sets initial watermarks, renders the header/content/footer, and passes `TreeCtx` callbacks down.
- **FeatureRow.tsx** — one clickable row with name, unseen badge (sky), pin toggle (amber), and active-state left edge.
- **GroupNode.tsx** — recursive collapsible group header (chevron + segment + count) over child groups and feature rows.
- **tree.ts** — `buildFeatureTree` splits groups on `/` and nests them into a sorted tree; `groupSegments` cleans the input; `ungroupedNode` wraps the ungrouped remainder.
- **collapse.ts** — sidebar-local store mapping group path → boolean (collapsed or not); default is expanded.
- **activity.ts** — `latestActivity` maps slug → newest event timestamp; `unseenCounts` derives badge counts; `sortFeatures` orders by recent activity then name; `seen` watermark per slug.
- **time.ts** — `createNow` reactive signal that ticks every 30s to refresh relative times; `formatAgo` renders "3m ago", "5h ago", "2d ago".
- **tree-styles.ts** — injects `.ftg-*` group-header styles once (id-guarded) rather than per recursive `GroupNode` instance.

## Invariants & gotchas

- **Watermark baseline** — `seen[slug]` must be initialized to the current max event id (`maxEventId()`) at mount, or else all historical events will count as unseen when the timeline first loads.
- **Ungrouped bucket logic** — when `tree().roots.length === 0`, render ungrouped features flat with NO header; when `roots.length > 0`, wrap them in `ungroupedNode()` with the `UNGROUPED_KEY = " ungrouped"` collapse key (NUL byte cannot appear in a real group path).
- **Collapse key uniqueness** — a group node's `path` is its full slash path (e.g. `backend/forge-index`), not just the segment; siblings with the same segment name at different depths would collide otherwise.
- **Pin event propagation** — the pin button inside `FeatureRow` must call `e.stopPropagation()` so the row's `onClick` (which selects the feature) doesn't fire.
- **Animation-delay index** — if feature rows are re-sorted or filtered, their `index` prop must remain stable or the stagger will jitter on every activity update; the current impl passes the `For` loop index, which is fine as long as the sorted array is stable per render.
- **Seen store persistence** — `seen` is local to the sidebar component (via `createRoot`) and resets on page reload; if you want durable watermarks, migrate it to `localStorage` or the backend.
