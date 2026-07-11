The changes this turn were about staleness detection for the **index**, not the timeline view itself. The notes describe a 10-second background poller for index staleness (`staleness::spawn_poller`) and modal suppression logic. The only timeline-view-adjacent change is that `handleIndexStatus` now checks for state transitions before re-popping the modal (lines 170-178 in app-store.ts).

This does NOT affect the Timeline View feature's design or behavior. The timeline is still a virtualized event feed; nothing about event fetching, pagination, filtering, or rendering changed. The existing doc is accurate.

I'll return the doc unchanged:

---
# Timeline View

## Purpose

Displays a virtualized, reverse-chronological feed of immutable repository events (file edits, commands, notes, index runs) filtered by feature, actor, or kind. Live updates stream in via `timeline:event` and preserve stable session-to-color mapping; pagination via `beforeId` cursor loads older events on demand.

## How it works

- **Merge & dedupe**: `use-timeline-events` merges the global `store.timeline` (live-updated via `timeline:event`) with paginated `extra` events, deduped by `id` and sorted newest-first (rowid is append-only, so id order ≡ timestamp order).
- **Baseline animation**: on mount, the highest event `id` becomes the baseline; any event with `id > baseline` gets the `tlr--live` streaming-in animation.
- **Client-side filter + refetch**: when the user narrows filters (feature/actor/kinds), the view refetches from the backend with `toIpcFilter(filters)` to discover older events that match; widening or clearing also refetches to re-settle exhaustion under the new scope.
- **Exhaustion probe**: a short page (`< PAGE_SIZE`) definitively means no older events exist for the current filter; the "Load older" button only appears when `!exhausted()` or more client-side rows remain.
- **Lazy rendering**: visible rows are capped at `renderLimit` (default 500); clicking "Load older" either pages in from the backend or just bumps `renderLimit` by `RENDER_STEP` if local rows exceed the limit.
- **Expandable payloads**: each row toggles open to reveal the full JSON payload (`<pre>` lazy-rendered via a latch) and metadata (actor, kind label, full timestamp).

## Key files

- `TimelineView.tsx` — top-level view: filter bar, event list, "Load older" pagination, empty/no-match states.
- `TimelineEventRow.tsx` — immutable row: session lane dot, kind glyph, kind-specific title, feature chips, relative time, expandable JSON payload.
- `TimelineFilterBar.tsx` — single-select feature/actor, multi-select kinds, clear button when narrowed.
- `timeline-utils.ts` — pure helpers: `laneColor` (stable djb2 hash onto accent palette), `eventTitle` (kind-specific formatting), `relativeTime`, filter matching, IPC filter translation.
- `use-timeline-events.ts` — merge/dedupe/pagination logic: maintains `extra` paged events, exposes `loadOlder` and `refetch`, tracks `exhausted`.

## Invariants & gotchas

- **Rowid = id**: event `id` is the SQLite `rowid`, append-only, so `id` order is timestamp order — never sort by parsed `ts` string.
- **Baseline must not reset**: `setBaseline(null)` mid-session re-animates all rows; the baseline latch is set once on first non-empty `events()` and never cleared.
- **Refetch on every filter change**: widening or clearing a filter can reveal older events the prior scoped fetch never saw; always call `refetch(toIpcFilter(...))` when filters change, not just when narrowing.
- **Exhaustion is scoped to the active filter**: `exhausted()` applies to the last backend fetch under the current filter; changing the filter invalidates the flag, hence the `setExhausted(false)` at the top of `refetch()`.
- **"Load older" guards both sources**: show the button when `filtered().length > visible().length` (local cap) OR `!exhausted()` (backend has more); clicking first tries to bump `renderLimit`, then calls `loadOlder` only if already at the local end.
- **Session lane colors are stable**: `laneColor(sessionId)` must return the same CSS var for the same session across renders; the djb2 hash is deterministic, but changing `LANE_VARS` order breaks existing session colors.
- **No edit affordances**: rows are immutable by design; the row click only toggles payload expansion, not inline editing or deletion.
