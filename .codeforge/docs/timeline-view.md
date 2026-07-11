The edits don't touch the Timeline View feature. The changes are to Welcome view (Claude CLI checks), session pane UI state, and repo command consolidation. The timeline feature remains unchanged.

---
# Timeline View

**Purpose**

Displays a reverse-chronological, append-only feed of all repo activity (file edits, command runs, agent sessions, index events, notes, doc refreshes). Filters by feature, actor, or event kind, with live subscription and infinite scroll.

**How it works**

- `useTimelineEvents` merges the live store's `timeline` slice (fed by `timeline:event` via IPC) with locally paged-in older events, deduplicated by append-only `id` (rowid).
- On mount and when filters narrow, calls `getTimeline` IPC with the current filter and `limit: 200` to settle exhaustion early; a short page means no older events exist.
- Infinite scroll via "Load older" button: either bumps the local render limit (`RENDER_STEP = 500`) or fetches the next page using `beforeId: oldest.id` cursor.
- Client-side filter matching when filters are narrowed (`matchesFilters`); backend refetch on every filter change to re-settle exhaustion for the new scope.
- Each row gets a stable session lane color (djb2 hash onto 5 accent vars), a mono kind glyph, kind-specific title (e.g. basename for `file_edited`, truncated command for `command_run`), feature chips, relative time, and expandable JSON payload (grid 0fr→1fr).
- Events with `id > baseline` (the highest id at first load) are flagged `live` and animate in via `streaming-line-in`.

**Key files**

- `views/TimelineView.tsx` — root view; composes filter bar + event rows, manages render limit, calls `useTimelineEvents` and `refetchScope()` on filter changes.
- `components/timeline/use-timeline-events.ts` — merges live store timeline with locally paged extra events, deduped by id; provides `loadOlder(filter)` and `refetch(filter)`.
- `components/timeline/TimelineEventRow.tsx` — immutable row: session lane dot, kind glyph, kind-specific title, feature chips, relative time, expandable JSON payload; no edit/delete affordances.
- `components/timeline/TimelineFilterBar.tsx` — single-select feature/actor, multi-select kinds, "clear filters" button when narrowed.
- `components/timeline/timeline-utils.ts` — pure helpers: `laneColor` (djb2 session-id hash), `KIND_GLYPH`/`KIND_LABEL`/`KIND_SHORT`, `eventTitle` (kind-specific payload extraction), `matchesFilters`, `toIpcFilter`, `relativeTime`.

**Invariants & gotchas**

- **Exhaustion is scoped to the fetched filter**: widening or clearing filters must call `refetch()` to re-settle exhaustion for the new scope — never assume an exhausted narrow filter means the unfiltered feed is exhausted.
- **Id order == ts order**: the backend rowid is strictly append-only, so sorting by `id` descending gives reverse-chron without parsing timestamps.
- **Baseline is latched on first load**: `baseline` is set to the highest id in the initial `events()` array and never reset; anything with a larger id is flagged `live` and animated in.
- **"Load older" only appears when older events exist**: the backend returns `limit: 200` pages, so a short page (`< 200`) definitively settles exhaustion — never show the button when `exhausted()` is true and the local render limit has caught up.
- **Client-side match, backend refetch**: filtering is client-side for fast UX, but every filter change triggers a backend refetch to keep exhaustion accurate for the new scope.
- **Payload keys are defensive**: `eventTitle` reads via `strField([...aliases])` because backend payloads are not strongly typed — never assume a single canonical key name.
