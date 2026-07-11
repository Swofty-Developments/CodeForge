---
# App Shell

## Purpose

The root React component that orchestrates FeatureForge's layout: sidebar feature tree, main panel with view tabs, right session pane for embedded Claude Code sessions, command palette (Cmd+K), and global keyboard shortcuts.

## How it works

- **Global keyboard handlers** — Cmd+K toggles the command palette, Cmd+\ toggles the session pane, Cmd+J toggles the terminal panel, Cmd+1-4 switch main views, and Escape closes in priority order (palette → deny pending approval → session pane).
- **Three-column layout** — sidebar (feature tree, resizable via drag handle), main panel (TabBar + view switcher showing FeatureDetail/GraphView/TimelineView/DiffReview), and session pane (resizable, collapsible, showing embedded Claude sessions).
- **Conditional rendering** — when no repo is open, renders a centered Welcome view; otherwise renders the full feature-driven UI.
- **Resize handles** — mousemove/mouseup event listeners manage sidebar and session pane widths, persisted in app-store.
- **Modals and overlays** — CommandPalette, InitRepoModal, StaleModal, MergeResultPanel, and WorktreeTabs render as overlays when their store flags are true.
- **Toast notifications** — bottom-right stack for errors and success messages, dismissible on click, auto-expire via store.
- **Global event listener registration** — main.tsx calls appStore.initListeners() once at startup to wire index:status, agent:event, timeline:event, index:progress, and repo:changed listeners; reducers inside the store demux events to the correct context.

## Key files

- **crates/tauri-app/frontend/src/App.tsx** — root shell component, layout, keyboard shortcuts, resize handlers, modal orchestration.
- **crates/tauri-app/frontend/src/main.tsx** — entry point, font registration, initListeners call, renders App into #app.
- **crates/tauri-app/frontend/src/stores/app-store.ts** — singleton SolidJS store holding all app state (contexts, sessions, features, timeline, UI flags); handleIndexStatus reducer with episode-based modal logic, initListeners setup.
- **crates/tauri-app/frontend/src/components/CommandPalette.tsx** — Cmd+K fuzzy-search overlay for actions, views, and features.
- **crates/tauri-app/frontend/src/components/SessionPane.tsx** — right pane showing Claude sessions, tab strip, new-session dropdown, composer.
- **crates/tauri-app/frontend/src/components/Sidebar.tsx** — left pane showing feature tree, unseen badges, index progress footer.

## Invariants & gotchas

- **Escape priority order** must remain palette → deny pending approval → session pane; other orderings break user flow assumptions.
- **Resize event listeners** are registered once on mount and never re-registered; attach them to window, not the component, to avoid double-listeners.
- **View keys (Cmd+1-4)** are hard-coded in VIEW_KEYS; changing them requires updating both the map and any user-facing docs.
- **store.repo is a derived getter** (active context's state) — never write to it directly; mutate contexts and activeContextPath instead.
- **All keyboard shortcuts require mod** (Cmd/Ctrl), except Escape — adding non-mod shortcuts risks colliding with text input.
- **Session pane width calculation** uses `window.innerWidth - e.clientX` because the handle hugs the right edge, not the left.
- **Toast auto-dismiss** is handled in the store slice (TOAST_DISMISS_MS), not in the component — do not duplicate timers.
- **handleIndexStatus uses episode-based modal tracking** (shownStaleModals set keyed by `${repoPath}:${status.state}`) so the backend's 10s polling loop can update indexStatusByPath without re-prompting the user for the same stale/outdated episode. The flag is cleared when the state transitions back to fresh/never or when the user acts on the modal (reindexStale), so a future stale→fresh→stale cycle will re-prompt.
- **indexStatusByPath is per-repo** (keyed by repoPath) so multi-context scenarios (base + worktrees) track staleness independently; handleIndexStatus always consumes status updates, but pops the modal once per episode.
- **handleTimelineEvent appends live events only to the active context** (checks `samePath(repoPath, store.activeContextPath)`) — background contexts reload their timeline on switch, so appending their events here would leak across worktrees.
