The changes are in backend Tauri commands and indexing logic (retry logic for headless Claude calls) — they don't affect the global app store's shape, slices, or event handling. The doc remains accurate.

---
# Global App Store

## Purpose

Singleton SolidJS store (`createRoot(createStore)`) that holds all global application state: open repo contexts (base + worktrees), UI layout (sidebar/session-pane/terminal-panel visibility and sizing), active view, feature list, toasts, and modals. Orchestrates four functional slices (context, data, session, terminal) and registers global event listeners (agent-event, timeline-event, index-progress) once per app run.

## How it works

- **Derived `repo` getter** — returns the active context's `RepoState`, so existing views keep working while the store tracks N open contexts keyed by path
- **Slices** — context-slice (multi-context model, worktree switching), data-slice (per-context loads: features/timeline/diff/daemon), session-slice (Claude session lifecycle + agent-event reducer), terminal-slice (PTY tab bookkeeping)
- **Event listeners** — `initListeners()` registers Tauri IPC listeners once; `handleTimelineEvent` appends live events only when they belong to the active context (prevents leaking events across worktrees); `handleRepoChanged` syncs external branch switches back into the store
- **Index staleness (FZ-2)** — `handleIndexStatus` tracks per-repo index state in `indexStatusByPath` (drives status-bar dot) and triggers the re-index modal once per state episode (stale/outdated), respecting per-repo suppression in localStorage
- **Toast surface** — `pushToast`/`pushError`/`pushSuccess` create auto-dismissing toasts with a 5s lifetime; sequence counter prevents ID collisions
- **Navigation** — `setActiveView` switches the main view (welcome/timeline/graph/diff/feature-detail) and triggers corresponding data refreshes

## Key files

- `crates/tauri-app/frontend/src/stores/app-store.ts` — store shape (frozen contract), slice orchestration, event reducers, singleton export
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — multi-context model (base + worktrees), worktree switching, git-init prompt
- `crates/tauri-app/frontend/src/stores/data-slice.ts` — per-context data loads (features/timeline/diff/daemon) and feature mutations (reindex/pin/edit/color)
- `crates/tauri-app/frontend/src/stores/session-slice.ts` — Claude session start/send/approve/stop, permission-mode switching
- `crates/tauri-app/frontend/src/stores/terminal-slice.ts` — PTY tab bookkeeping (open/close/activate), worktree-scoped terminal cwd

## Invariants & gotchas

- **Store shape is frozen** — adding new top-level state requires updating the `AppStore` interface. Actions live in slices, not the store object itself.
- **`repo` is derived, never written** — it's computed from `activeContextPath` and `contexts`. Mutating it directly will fail.
- **Live events are context-scoped** — `handleTimelineEvent` only appends events matching `activeContextPath`, preventing cross-worktree leaks. Background contexts reload their timeline on switch.
- **Index status has two concerns** — (1) status tracking (always consume), (2) modal triggering (once per episode). The `shownStaleModals` set prevents duplicate prompts until the state transitions or the user acts.
- **Toast auto-dismiss** — toasts vanish after 5s. If you need persistent errors, surface them in the UI separately (e.g., `lastError`).
- **Living-doc hot reload** — when a `doc_updated` event lands for the open feature (`selectedFeature`), the store refetches the doc and updates `selectedFeatureDoc` in place, guarded against stale responses.
