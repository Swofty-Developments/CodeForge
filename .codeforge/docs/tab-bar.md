---
# Tab Bar

## Purpose

Top navigation strip that switches between the four main views (Feature, Graph, Timeline, Diff review) and displays the active repository name. Controlled via mouse clicks or keyboard shortcuts Cmd+1/2/3/4. The "Welcome" view has no visible tab — it is the default state before a repo is opened.

## How it works

- Renders four static tabs (Feature, Graph, Timeline, Diff) with click handlers that call `appStore.setActiveView(view)`.
- Active tab is highlighted with `.active` class by comparing `store.activeView` to each tab's view ID.
- The Feature tab displays the currently selected feature slug (if one is selected) via `.tab-context` span.
- Keyboard shortcuts (Cmd+1/2/3/4) map to view IDs in `App.tsx:56-76` and fire the same `setActiveView` action. The shortcuts only fire when `store.repo` is non-null.
- Switching to `diff` or `timeline` views triggers a data refresh for that view (data-slice).
- Right side of the bar displays the active repo name (`store.repo.name`) and a session pane toggle button.
- The session pane toggle button calls `appStore.toggleSessionPane()` which flips `store.sessionPaneOpen`. The button is highlighted (`.active` class) when the session pane is open.
- The active tab visually merges into the content area below via `margin-bottom: -1px` and matching bottom-border color (`--bg-tab-active`).

## Key files

- `crates/tauri-app/frontend/src/components/TabBar.tsx` — Tab strip UI, click handlers, Zed-inspired active-tab styling, session pane toggle button.
- `crates/tauri-app/frontend/src/App.tsx:56-76` — Keyboard shortcut listener (`Cmd+1/2/3/4` → `setActiveView`, guarded by `store.repo`).
- `crates/tauri-app/frontend/src/stores/app-store.ts:253-255` — `toggleSessionPane` action, flips `store.sessionPaneOpen`.
- `crates/tauri-app/frontend/src/types.ts:432` — `ActiveView` type definition (`"welcome" | "feature" | "graph" | "timeline" | "diff"`).

## Invariants & gotchas

- The `TABS` array order must match the keyboard shortcut map in `App.tsx:56-61` (1→feature, 2→graph, 3→timeline, 4→diff).
- The keyboard shortcuts only activate when `store.repo` is non-null — they do nothing on the welcome screen.
- `setActiveView` must remain the single entry point for view changes — direct `setStore("activeView", ...)` bypasses the refresh triggers for `diff` and `timeline` views.
- The active tab's bottom border color must match `--bg-tab-active` to achieve the seamless merge effect; changing one requires changing the other.
- Session pane toggle button is part of this component for layout consistency, but its state (`sessionPaneOpen`) lives in `appStore`.
- The Feature tab shows the selected feature slug when `store.selectedFeature` is non-null; the context text is styled with `--text-accent` and monospace font.
- The session pane toggle uses the `⌘\` shortcut (shown in tooltip) which is registered elsewhere in the app.
