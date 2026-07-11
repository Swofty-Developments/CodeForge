The changes from this turn don't affect the Tab Bar feature. The edits were in the backend staleness poller and the frontend's `handleIndexStatus` handler (lines 171-178 of app-store.ts), which manage the stale-index modal — a separate concern from tab navigation. The Tab Bar component itself wasn't touched, and none of the Tab Bar's dependencies changed (tabs, active view switching, or repo name display remain unchanged).

The living doc is still accurate.

---
# Tab Bar

## Purpose

Renders the top navigation for the main content panel, allowing users to switch between Feature Detail, Graph, Timeline, and Diff Review views. Styled after Zed's tab strip, where the active tab merges visually into the content pane below.

## How it works

- Renders four static tabs (Feature, Graph, Timeline, Diff review) from the `TABS` constant, each wired to an `ActiveView` enum value.
- Clicking a tab calls `appStore.setActiveView(view)`, which updates `store.activeView` and triggers the main panel to render the corresponding view.
- The Feature tab appends a `.tab-context` suffix showing the currently selected feature slug (e.g., "tab-bar") when one is active.
- The right side displays the repo name (from `store.repo.name`) and a session-pane toggle button with a split-panel icon.
- Active tab styling: uses `--bg-tab-active` background, -1px bottom margin, and matching bottom border to visually merge with the content area below by hiding the tab-bar's bottom border.
- No close buttons or drag-to-reorder — tabs are fixed views, not closable documents.

## Key files

- `crates/tauri-app/frontend/src/components/TabBar.tsx` — tab bar component, view-switch logic, inline styles
- `crates/tauri-app/frontend/src/stores/app-store.ts` — `activeView` state and `setActiveView` action
- `crates/tauri-app/frontend/src/types.ts` — `ActiveView` union type ("welcome" | "feature" | "timeline" | "diff" | "graph")

## Invariants & gotchas

- **Fixed tab set**: the `TABS` array is hardcoded to four views; adding/removing a view requires updating both `TABS` and the `ActiveView` type.
- **Active tab overlap**: the active tab's -1px `margin-bottom` and matching `border-bottom` color must stay synchronized with `--bg-tab-active` to maintain the merge effect; changing one without the other breaks the visual.
- **"welcome" view is hidden**: the `ActiveView` type includes `"welcome"`, but TabBar does not render a tab for it — it's an initial-state view automatically replaced when a repo loads.
- **Immutable tab order**: tabs are rendered in `TABS` order; no drag-to-reorder or user customization is supported.
- **Height constraint**: hardcoded to `var(--tabbar-height)` (24px per description); increasing it breaks the compact design contract.
- **Session toggle visibility**: the session-pane toggle button appears unconditionally, even if no session is active — the button's active state reflects `store.sessionPaneOpen`.
