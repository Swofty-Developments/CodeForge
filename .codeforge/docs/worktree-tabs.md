# Worktree Tabs

## Purpose

Top tab bar that displays all open repository contexts (base repo + worktrees) with project identity badges, branch labels, git status indicators, and focus-driven metadata refresh. Enables switching between worktrees and closing tabs without touching the on-disk worktree.

## How it works

- **Multi-project grouping**: tabs sort alphabetically by project name (from `RepoState.project`), base-first within each project; project identity badges (`projectInitials()`) appear only when multiple projects are open simultaneously
- **Focus-driven refresh**: window `focus` event listener triggers `refreshWorktrees()` to update ahead/behind counts and dirty state from git; initial refresh runs on mount
- **Tab state indicators**: each tab shows branch name (or context name fallback), a "base" badge for the base repo, a dirty dot (amber) for uncommitted changes, and ahead/behind counts (green ↑ / orange ↓) derived from the `Worktree` metadata joined by path
- **Context lifecycle**: clicking a tab calls `switchContext(path)` to reload features/timeline/diff for that worktree; close button (X) invokes `closeContext()` which tears down the runtime but **never removes the worktree from disk** (C3 invariant — removal requires explicit user confirmation via `WorktreeSwitcher`)
- **Active tab styling**: Zed-inspired active tab merges into content surface via `border-bottom-color: var(--bg-base)` and negative bottom margin; active state determined by `samePath(store.activeContextPath, ctx.state.path)`
- **Switcher integration**: "+" button toggles `WorktreeSwitcher` modal for creating/checking out/removing worktrees; switcher anchored to the button ref

## Key files

- `crates/tauri-app/frontend/src/components/WorktreeTabs.tsx` — tab strip component: rendering, focus listener, tab activation/close, project badge logic
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — `switchContext()`, `closeContext()`, `refreshWorktrees()`, multi-context state management
- `crates/tauri-app/frontend/src/types.ts` — `RepoContext`, `Worktree`, `RepoState` types (base/worktree distinction, project field)
- `crates/tauri-app/frontend/src/components/worktree/WorktreeSwitcher.tsx` — modal opened by "+" button (referenced but not read)

## Invariants & gotchas

- **Closing a tab ≠ removing a worktree**: `closeContext()` destroys the in-memory `RepoContext` and calls `close_repo(path)` but leaves the worktree on disk intact; removal requires explicit `removeWorktreeExplicit()` with user confirmation
- **Project fallback**: `projectOf()` falls back to `ctx.state.name` when `ctx.state.project` is missing (older `repo:changed` payloads) to prevent badge mismatch; rely on backend populating `project` for correct grouping
- **Base context is never closable**: base tabs render no close button (`<Show when={!ctx.isBase}>`); closing the base triggers `closeRepo()` which tears down the entire repo family
- **Path identity**: all context lookups use `samePath()` for case/normalization-insensitive matching; direct string equality will break on macOS case-folded paths
- **Metadata staleness window**: `Worktree` metadata (ahead/behind/dirty) is stale between focus events; do not treat counts as real-time — they reflect the last `list_worktrees` call
