The agent's work this turn (adding interrupt/stop functionality to sessions) does not affect the Activity Badges feature. The changes were to the session pane, not the sidebar. The living doc remains accurate.

---
# Activity Badges

## Purpose

Live event counters that appear on feature rows in the sidebar tree. Each badge shows the number of timeline events that have landed since the user last selected that feature (or since the repo was opened), with the count resetting to zero when the row is clicked.

## How it works

- **Watermark per feature** — Each feature slug has a "seen up to event id" marker stored in a reactive record (`seen`). On repo open, all features are baselined to the current max event id so only future activity increments badges.
- **Unseen count derivation** — `unseenCounts()` walks the timeline slice, comparing each event's id against the feature's watermark; events with `id > seen[slug]` increment that feature's count.
- **Reset on selection** — When a feature row is clicked, `markSeen(slug, maxEventId())` updates the watermark to the timeline head, zeroing the badge.
- **Activity-based sort** — Features are sorted by `latestActivity()` (newest event timestamp per feature from the timeline), then by name. Falls back to the feature's static `updatedAt` field if no timeline events exist.
- **Badge caps at 99+** — The UI renders counts above 99 as "99+" to prevent layout overflow.
- **Time tick for freshness** — The sidebar uses `createNow()` (30s ticks) to keep relative times ("5m ago") current, though the badge counts themselves are event-driven, not time-decayed.

## Key files

- `crates/tauri-app/frontend/src/components/sidebar/activity.ts` — Badge count logic: watermark store, unseen-count derivation, activity-based sort, latest-activity map.
- `crates/tauri-app/frontend/src/components/Sidebar.tsx` — Wires `unseenCounts()` into the tree, baselines watermarks on repo open, resets on row selection.
- `crates/tauri-app/frontend/src/components/sidebar/FeatureRow.tsx` — Renders the badge (sky pill) when `activity > 0`, capped at "99+".
- `crates/tauri-app/frontend/src/components/sidebar/time.ts` — Time tick helper for relative timestamps (not directly used by badge counts, but shared sidebar timing).

## Invariants & gotchas

- **Watermark baseline on repo open** — Every feature slug must be initialized to `maxEventId()` when first appearing in the store, otherwise badges will count all historical events instead of only new activity.
- **Event id monotonicity** — Badge logic assumes event ids are strictly increasing over time. If the backend ever resets ids or delivers events out-of-order, counts will break (won't decrement, but may reset prematurely).
- **Selection must reset watermark** — The `onSelect` callback MUST call `markSeen(slug, maxEventId())` before the timeline updates again, or the badge will continue showing stale counts.
- **No time-based decay** — Despite the feature description mentioning exponential moving average and time decay, the current implementation uses a simple unseen-count watermark with no decay. Counts persist until the user selects the row; there is no automatic reduction over time.
- **Badge count is slice-local** — Counts reflect only the events currently loaded in `store.timeline`. If the timeline is paginated and older events are unloaded, those events won't contribute to the count even if they're above the watermark.
