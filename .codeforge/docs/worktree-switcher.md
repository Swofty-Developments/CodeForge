The changes are about staleness detection and indexing, not the worktree switcher. The living doc is current and accurate. No update needed.

---
# Worktree Switcher

## Purpose
A fixed-positioned popover that lists all contexts, worktrees, and branches in four named sections (Open / Worktrees / Branches / Remote), allowing users to switch to existing worktrees, create new worktrees from branches, fetch remotes, and remove worktrees. Triggered from the tab strip "+" button or TitleBar "Worktrees…" menu.

## How it works
- Renders through a `Portal` with `position: fixed` to escape the tab strip's `overflow-x: auto` clip boundary
- On mount, queries `listBranches()` and `refreshWorktrees()`, then filters all four sections by the user's query string
- Four disjoint sections: **Open** shows active contexts (base first), **Worktrees** shows un-opened disk worktrees, **Branches** shows local branches not checked out, **Remote** shows remote-tracking branches with no local equivalent
- Create row appears when query doesn't exactly match an existing branch — clicking it spawns a new worktree from the current branch
- Branch rows call `appStore.addWorktreeForBranch()` which returns `null` on success or an error string (e.g. "already checked out at \<path>"); on error, the switcher shows it inline and reloads so the covering worktree row appears as the next affordance
- Worktree rows have a trash icon that confirms before calling `appStore.removeWorktreeExplicit()`; dirty worktrees get a warning about permanent data loss
- Footer has a "Fetch" button that runs `fetchRemotes()` and reloads branches/worktrees

## Key files
- **WorktreeSwitcher.tsx** — popover container, filter input, section rendering, branch/worktree/fetch actions, error state
- **SwitcherRows.tsx** — presentational row components: `CreateRow`, `ContextRow`, `WorktreeRow`, `BranchRow` (all actions passed in)
- **switcher-data.ts** — pure derivation of four sections, `hasExactBranch()` suppression, fuzzy filtering
- **switcher-css.ts** — Zed-styled popover CSS with hairline borders, instant hovers, mono branch names

## Invariants & gotchas
- **Portal-based positioning**: switcher MUST render through `Portal` because the tab strip is `overflow-x: auto` and clips absolutely-positioned descendants
- **Sections are disjoint by construction**: a branch with `checkedOutAt` set never re-appears as a branch row; an open context's worktree never re-appears under "Worktrees"
- **Error-driven UX**: when `addWorktreeForBranch()` fails with "already checked out", the switcher shows the error and reloads — the now-visible worktree row IS the "open it" affordance
- **Explicit remove confirmation**: dirty worktrees warn that uncommitted changes will be permanently lost; clean worktrees confirm but emphasize the branch is kept
- **Create row suppression**: `hasExactBranch()` checks branches, worktrees, and contexts — a query matching any of them hides the create row to prevent duplicates
- **Remote section filter**: only shows remote-tracking branches whose short name has no local branch (avoids redundant `origin/main` when `main` exists locally)
---
