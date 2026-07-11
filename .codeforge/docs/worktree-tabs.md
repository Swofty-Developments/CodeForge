# Worktree Tabs

## Purpose

Horizontal tab strip below the title bar that displays all open repo contexts (base + git worktrees). Each tab shows branch name, ahead/behind counts vs. base, dirty state indicator, and project identity badges when multiple projects are open simultaneously.

## How it works

- Tabs are sorted by project name (alphabetical), then base-first within each project
- Project identity badges (two-letter initials with color-coded borders) appear only when tabs span multiple distinct projects
- Each tab pulls metadata from `store.worktrees` (refreshed via `list_worktrees` on window focus)
- Active tab uses Zed-inspired styling: merges into content surface via matching background and border-bottom color
- Closing a tab removes only the context (C3) from memory — never deletes the worktree from disk, so no confirmation is needed
- The "+" button opens `WorktreeSwitcher`, a Portal-rendered fixed-position popover (to avoid clipping by the tab strip's `overflow-x: auto`)

## Key files

- `crates/tauri-app/frontend/src/components/WorktreeTabs.tsx` — tab strip component, sorting, project badge logic, close/activate handlers
- `crates/tauri-app/frontend/src/components/worktree/WorktreeSwitcher.tsx` — branch/worktree picker popover (Portal-rendered, fixed positioning)
- `crates/tauri-app/frontend/src/components/worktree/switcher-data.ts` — pure section derivation (open contexts, on-disk worktrees, local branches, remote branches — all disjoint)
- `crates/tauri-app/frontend/src/stores/app-store.ts` — `contexts: RepoContext[]`, `worktrees: Worktree[]`, and derived `repo` getter

## Invariants & gotchas

- **Close is NOT remove**: closing a tab via the "×" button only removes the context from `store.contexts` — the worktree remains on disk. Actual worktree removal (with dirty-state confirm) is an explicit action in `WorktreeSwitcher`.
- **Project fallback**: `ctx.state.project` is set by the backend (base-repo basename), but older `repo:changed` events may lack it — fallback is `ctx.state.name` so tabs still group with themselves.
- **Multi-project detection**: badges appear when `new Set(contexts.map(projectOf)).size > 1`. If all open tabs share one project, badges are hidden.
- **Metadata join**: tab counts/dirty state come from `store.worktrees.find(w => samePath(w.path, ctx.state.path))`, not from `ctx.state` — `list_worktrees` is the authoritative source for ahead/behind/dirty.
- **Base is unmutable**: the base context cannot be closed (no "×" button rendered). At least one context must remain open.
- **Active tab border trick**: active tab sets `border-bottom-color: var(--bg-base)` and `margin-bottom: -1px` to visually merge with the content pane below.
