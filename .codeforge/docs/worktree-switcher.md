# Worktree Switcher

## Purpose

Unified branch/worktree picker shown from the tab-strip "+" or title-bar "Worktrees…". Provides fuzzy search over open tabs, on-disk worktrees, and local/remote branches, plus a flow to create new worktrees from novel branch names.

## How it works

- Renders as a fixed-position portal (not inside the scrolling tab bar) to avoid clip bugs; anchor placement recomputes on mount and window resize.
- Four named sections: **Open** (existing tabs, base first), **Worktrees** (on-disk checkouts not yet opened), **Branches** (local not checked out), **Remote** (remote-tracking whose short name has no local branch). Sections are disjoint — a branch checked out in a worktree never appears as a branch row.
- Typing a novel name (not matching any branch, worktree, or open context) shows a **Create** row; Enter creates a worktree branching from the current branch.
- Branch row click → `addWorktreeForBranch(name, remote)` → on success close, on named failure (already checked out) show inline error and reload so the covering worktree row appears.
- Worktree trash button → confirm dialog (warns if `dirty`), then explicit remove via `removeWorktreeExplicit`.
- Footer "Fetch" button → `fetchRemotes` + reload branches and worktrees.

## Key files

- **WorktreeSwitcher.tsx** — portal root, input, sections, error/loading states, row actions.
- **SwitcherRows.tsx** — presentational CreateRow, ContextRow, WorktreeRow, BranchRow (icons, status, trash).
- **switcher-data.ts** — pure `buildSections` (query filter + disjoint partitioning), `hasExactBranch` (create-row suppression).
- **switcher-css.ts** — inline `<style>` block (Zed-styled elevated popover, mono branch names, instant hover).

## Invariants & gotchas

- **Sections must remain disjoint.** A worktree row must never re-appear as a branch row; `checkedOutAt` in `BranchInfo` is the single source of truth. The `buildSections` filter chain enforces this: local/remote branches filtered by `checkedOutAt === null`, onDisk worktrees filtered by `!open.some(samePath)`.
- **Fixed positioning required.** The tab bar is `overflow-x:auto`; absolute children clip. The portal's `.ws-pop` has `position: fixed` and `z-index: 100` above the `z-index: 99` backdrop.
- **Safe-close warning on dirty worktrees.** The remove-worktree confirm checks `w.dirty` and shows a LOUD warning that uncommitted changes will be lost. `removeWorktreeExplicit` takes a `force` param that must match `w.dirty`.
- **Create row only appears for novel names.** `hasExactBranch` checks branches array, worktrees array, and open contexts. False positive suppression (showing create when the name exists) breaks the UX; false negative (hiding create when it's novel) is rare but recoverable via CLI.
- **Inline error shown after action failure.** On `addWorktreeForBranch` named error (e.g. "already checked out at X"), reload branches + worktrees so the user sees the worktree row that's blocking them — that row IS the affordance to open it.
- **Remote-tracking names show `remote/name`** in the UI but pickBranch receives `(name, remote)` separately. BranchRow's `busy` keying is `${remote}:${name}` to disambiguate `origin/main` vs `upstream/main`.
