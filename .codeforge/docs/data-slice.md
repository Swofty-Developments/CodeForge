The data-slice file was not modified in this turn — the changes were all in session-slice. The existing doc remains accurate. No update needed.

---
# Data Slice

## Purpose

The data slice holds all actions for loading per-context data (features, timeline, diff, daemon status) and mutating feature state (pin, color, edit, select). Every action reads the store's derived `repo` getter, so it always operates on the active context—whether that's the base checkout or a worktree.

## How it works

- **Refresh functions** (`refreshFeatures`, `refreshTimeline`, `refreshDiff`, `refreshDaemon`) each call one IPC endpoint scoped to `store.repo.path` and update their corresponding store slice. They bail early when `store.repo` is null.
- **Feature mutations** (`pinFeature`, `setFeatureColor`, `updateFeature`) apply optimistic UI updates first, then call the backend IPC. On error, they roll back the optimistic change and push a toast.
- **`selectFeature(slug)`** clears the detail state (`selectedFeatureTimeline`, `selectedFeatureDoc`), then fans out three parallel IPC calls (`getFeature`, `getTimeline`, `getFeatureDoc`) and applies the results only if that feature is still selected (stale-response guard).
- **`openFeatureDetail(slug)`** calls `selectFeature` and switches `activeView` to `"feature"` in one step—the sidebar's Open shortcut.
- The slice is instantiated once per app run by `createAppStore`, wired to the global `store`/`setStore`, and its actions are re-exported at the top level of `appStore` so components can call them directly.
- All errors are surfaced via the `pushError` callback, which appends to `store.toasts` and sets `store.lastError`.

## Key files

- **`crates/tauri-app/frontend/src/stores/data-slice.ts`** (core) — all refresh and mutation actions; the single slice that touches features/timeline/diff/daemon.
- **`crates/tauri-app/frontend/src/stores/app-store.ts`** — defines `AppStore`, wires slices together, registers the `index:status` listener (`handleIndexStatus`), and exports the singleton `appStore`.
- **`crates/tauri-app/frontend/src/types.ts`** — TypeScript mirrors of Rust IPC types (`Feature`, `FeaturePatch`, `TimelineEvent`, `DiffByFeature`, etc.).
- **`crates/tauri-app/src/runtime/staleness.rs`** — spawns the per-repo background poller that emits `index:status` events every 10s when the verdict changes; lives in `RepoRuntime.staleness_task`.

## Invariants & gotchas

- **Never write to `store.repo` directly**—it's a getter that derives from `store.activeContextPath` and `store.contexts`. Write to those instead, or call context-slice actions.
- **Stale-response guard in `selectFeature`**—only apply fetched data if `store.selectedFeature === slug` at arrival time, so slow responses from a previous selection don't clobber the new one.
- **Optimistic updates must roll back on error**—`pinFeature` and `setFeatureColor` both capture the previous value before applying the change, then restore it in the catch block. Forgetting this leaves the UI out of sync with the backend.
- **Living-doc hot reload** is handled by the global `handleTimelineEvent` reducer in app-store, not by this slice—when a `doc_updated` event arrives for the selected feature, the store refetches `getFeatureDoc` directly rather than delegating to `selectFeature`.
- **No data slice actions run if `store.repo` is null**—every function guards early. The UI must ensure `activeContextPath` is set before calling these, or the calls silently no-op.
- **Index staleness is now polled, not one-shot**—the backend runs a 10s ticker per open repo (`staleness::spawn_poller`) that re-hashes the manifest and emits `index:status` only when the verdict *changes*. The frontend's `handleIndexStatus` pops the stale modal only on a state transition (`prev.state !== status.state`), so a dismissed modal isn't re-shown every poll while the user keeps editing and changedFiles grows.
