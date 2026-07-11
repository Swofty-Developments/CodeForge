# Activity Badges

## Purpose

Displays a numeric badge on each sidebar feature row showing the count of unseen timeline events for that feature. The badge appears only when the count is non-zero, pulses when new events arrive, and clears when the user selects the feature.

## How it works

- **Watermark per feature** — each feature slug tracks the highest timeline event ID the user has "seen"; stored in a global reactive `seenStore`.
- **Baseline on repo open** — when a repo loads, every feature's watermark initializes to the current timeline head so only *new* activity (events arriving during this session) counts toward badges.
- **Unseen count derivation** — `unseenCounts()` walks the timeline and increments each feature's count for events with `id > seenMap[slug]`.
- **Clear on selection** — when the user clicks a feature row, `markSeen(slug, maxEventId())` advances the watermark to the current timeline head, zeroing the badge.
- **Sort by recency** — `sortFeatures()` uses `latestActivity()` (newest event timestamp per feature) to surface recently-touched features at the top of the sidebar tree.

## Key files

- `crates/tauri-app/frontend/src/components/sidebar/activity.ts` — watermark store (`seen`), unseen-count derivation, activity-based feature sort.
- `crates/tauri-app/frontend/src/components/sidebar/FeatureRow.tsx` — renders the sky badge when `activity > 0`; caps display at "99+".
- `crates/tauri-app/frontend/src/components/Sidebar.tsx` — wires `unseenCounts()` and `markSeen()` into the feature tree; initializes watermarks on mount.

## Invariants & gotchas

- **Watermark must match timeline semantics** — event IDs are sequential and monotonic; a watermark of `-1` (the default for a slug not yet in `seenStore`) treats every event as unseen.
- **Baseline *after* repo opens** — if the watermark initialized before the first timeline load, old events would count as "new"; the `createEffect` in `Sidebar.tsx` runs once features and timeline are both loaded.
- **Event IDs span all features** — an event with `featureSlugs: ["a", "b"]` increments both features' counts; the watermark is per-feature but the event-ID space is global.
- **Badge updates are reactive** — `unseenCounts()` is a `createMemo` over `store.timeline` and `seen`, so new events (from file watches, hook ingestion, or agent forwarding) immediately update all affected badges.
- **No persistence** — the `seen` watermark lives only in the current session; reopening the app resets all badges. This is intentional — badges signal *this session's* activity, not a cumulative unseen count.
