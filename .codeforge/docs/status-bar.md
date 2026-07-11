The changes this turn added `sessionPaneFullscreen` to the app store (line 74) but didn't modify StatusBar.tsx. The status bar remains unchanged.

---
# Status Bar

## Purpose

24px bottom chrome showing consolidated repo status (branch, daemon, index freshness), session count, and terminal toggle. Hover reveals a detail popover with one-click reindex.

## How it works

- **Aggregates status into one dot** — priority order: indexing in progress → daemon down → index stale/outdated → all good. Dot color/pulse driven by `aggregate()` function (ok/busy/warn/bad/idle).
- **Hover popover** shows repo name, branch, daemon state, feature count, index age, freshness state, changed files (up to 4), and a Reindex button. Button disabled while indexing.
- **Index status** pulled from `store.indexStatusByPath[activeContextPath]`; maps four states (fresh/stale/outdated/never) to human labels. Version mismatch shown when outdated.
- **Right side** displays session count (`store.sessions.length`) and terminal toggle button with active-state styling and badge count (`store.terminals.length`).
- **Activity line** (1px → 2px animated gradient) sits above the bar, visible only when any session has `runState === "generating"`.
- **Reindex action** calls `appStore.reindex()` when clicked, disabled during indexing.

## Key files

- **StatusBar.tsx** — entire status bar component (24px chrome); aggregates daemon + index + session state; hover popover with reindex button; terminal toggle.

## Invariants & gotchas

- **Aggregation priority must be strict** — indexing beats daemon-down beats stale beats ok. Reordering breaks the severity model.
- **Popover only shows when `store.repo` exists** — no repo means no popover, no reindex button.
- **Index age** calculated client-side from `indexedAt` timestamp; "never" if null. Not live-updated (no interval), so age only refreshes on re-render.
- **Terminal toggle disabled** when no repo loaded (`disabled={!store.repo}`).
- **Changed files capped at 4** in popover to prevent overflow; "+N more" shown if > 4.
- **Activity line respects `prefers-reduced-motion`** — no animation if user opts out.
