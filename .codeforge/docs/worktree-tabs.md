# Worktree Tabs

## Purpose

A horizontal tab strip showing all open worktree contexts for the current repo family (base worktree first, then linked worktrees). Clicking a tab switches the app to that worktree's context; closing a tab tears down the frontend context but never removes the worktree from disk.

## How it works

- **Tab order**: contexts are grouped alphabetically by project identity (`state.project` basename), with the base worktree first within each project group, then remaining worktrees in open order.
- **Project badges**: two-letter initials appear only when multiple projects are open; color-coded via `hueForSlug` for visual grouping.
- **Metadata overlay**: each tab shows branch name, a dirty dot (uncommitted changes), ahead/behind counts from `list_worktrees`, and a "base" badge for the main worktree. The active tab uses Zed-style highlighting with a merged bottom border.
- **Close behavior**: the "×" button calls `closeContext(path)`, which tears down the frontend runtime and drops the context from the store but leaves the worktree on disk. Explicit removal is a separate action in the WorktreeSwitcher.
- **Refresh cadence**: `refreshWorktrees()` is invoked on mount, on window focus, and after every context/worktree mutation (open, create, close, remove) to keep dirty/ahead/behind counts current.
- **"+" button**: opens the WorktreeSwitcher popover for creating new worktrees, switching branches, and explicit worktree removal.

## Key files

- `crates/tauri-app/frontend/src/components/WorktreeTabs.tsx` (core) — the tab strip UI and ordering logic.
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — `switchContext`, `closeContext`, and `refreshWorktrees` actions that the tabs invoke.
- `crates/tauri-app/frontend/src/components/worktree/WorktreeSwitcher.tsx` — the modal opened by the "+" button.

## Invariants & gotchas

- **Closing a tab is never destructive**: `closeContext` tears down the frontend context but never calls `remove_worktree` on the backend. Disk removal is always explicit via the switcher.
- **Base worktree cannot be closed via "×"**: the close button only renders for `!ctx.isBase`. Closing the base context closes the entire repo family.
- **Tab activation is path-based**: `samePath` comparisons handle symlink-resolved paths so tabs remain consistent with backend worktree identity.
- **Worktree metadata is eventually consistent**: `refreshWorktrees()` is debounced by the backend, so dirty/ahead/behind indicators lag slightly after git operations.
- **Project identity fallback**: if `state.project` is missing (older backend emit), `projectOf` falls back to `state.name` so tabs still group correctly.
