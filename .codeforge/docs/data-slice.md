All commands are still registered. The changes are orthogonal to the data-slice: retry logic is now in `headless.rs` (which `reindex` indirectly triggers), and the agent was just exploring command registration. The existing doc remains accurate.

---
# Data Slice

## Purpose

Manages per-repo data fetching (features, timeline, diff, daemon status) and feature mutations (pin, color, edit). All operations read from the active context, and live timeline events trigger activity badges and frontend notifications for affected features.

## How it works

- **Load on demand** — `refreshFeatures`, `refreshTimeline`, `refreshDiff`, `refreshDaemon` fetch via IPC and write into the store; invoked on repo open, context switch, and after indexing completes.
- **Live event stream** — `handleTimelineEvent` (in `app-store.ts`) consumes backend-pushed timeline events, appends them to `store.timeline` for the active context, and increments `featureActivity[slug]` counters for affected features.
- **Activity badges** — `featureActivity` is a per-feature counter bumped by each live event; sidebar reads these to show unread-style badges.
- **Optimistic mutations** — `pinFeature`, `setFeatureColor` apply changes locally before the IPC call, then rollback on failure; `updateFeature` waits for the backend response before committing.
- **Feature selection** — `selectFeature(slug)` clears prior detail state, then fetches feature metadata, timeline slice (limit: 50), and living-doc markdown in parallel; `openFeatureDetail` wraps this and switches to the feature view.
- **Living-doc hot reload** — when a `doc_updated` event arrives for the selected feature, the slice refetches `getFeatureDoc` so the open detail panel updates in place.

## Key files

- `crates/tauri-app/frontend/src/stores/data-slice.ts` — all refresh / mutation actions (the exported slice).
- `crates/tauri-app/frontend/src/stores/app-store.ts` — `handleTimelineEvent` reducer that bumps `featureActivity` and appends live events; session-pane layout state (`sessionPaneFullscreen`).
- `crates/tauri-app/frontend/src/ipc.ts` — thin wrappers over Tauri `invoke` commands (`get_features`, `get_timeline`, `get_diff_by_feature`, `listenTimelineEvent`); added `checkClaudeCli` for session setup.

## Invariants & gotchas

- **Active-context only** — every data-slice method reads `store.repo` (the derived active-context getter); calling these when no repo is open is a no-op.
- **Stale-response guard** — `selectFeature` and `handleTimelineEvent` check that `store.selectedFeature` still matches before applying fetched data, since async loads can land after the user has switched features.
- **featureActivity is append-only** — counters are never cleared except on context switch (via `resetContextView` in `context-slice.ts`); the UI shows "new activity since you last looked" semantics.
- **Timeline event filtering** — `handleTimelineEvent` ignores events from background contexts (only appends when `repoPath === activeContextPath`); background timelines are refreshed wholesale on `switchContext`.
- **Living-doc refetch is conditional** — only `doc_updated` events with `outcome: "updated"` trigger a reload; failed refreshes stay visible on the timeline without updating the doc.
- **approveSession now accepts answers** — the `answers` param (question → selected option) is passed through for multi-choice prompts; remains optional for simple approve/deny flows.
- **Indexer retry is transparent** — `reindex` triggers backend indexing, which now retries transient failures (network, rate limits) up to 3 times with exponential backoff; the frontend sees only success or final failure.
