# Git Worktree Management

## Purpose

Manages git worktrees for the repository: creating isolated branches in separate directories, tracking ahead/behind state, merging branches back to the base, and safely removing worktrees. Prevents common pitfalls (branch-already-checked-out, dangling detached HEAD, accidental base removal) by surfacing them as named errors rather than silent failures.

## How it works

- **List** parses `git worktree list --porcelain` into enriched `Worktree` structs with ahead/behind counts (via `rev-list --left-right --count`), dirty status, and branch/detached-HEAD state.
- **Create** slugifies the name, checks for path/branch collisions, runs `git worktree add -b <slug>` under `.codeforge-worktrees/`, and returns the enriched entry.
- **Remove** refuses to delete the base worktree (checked via canonicalized path equality); otherwise forwards `git worktree remove [--force]` errors verbatim (dirty without force, locked, etc.).
- **Merge** attempts `git merge --no-edit <branch>` from the worktree into the base; on conflict, extracts conflicted paths, aborts the merge, and returns a structured `MergeResult` with `aborted: true` — the base is never left mid-merge.
- **Detached-HEAD/no-merge-base** are named states: a detached worktree is compared by HEAD sha; branches with no merge-base report `(0, 0)` ahead/behind.
- **All mutations** target the base repository root (the first `git worktree list` entry), even when invoked from a worktree path.

## Key files

- `crates/forge-git/src/worktree.rs` — core API: `list_worktrees`, `create_worktree`, `merge_worktree`, `remove_worktree`, ahead/behind logic.
- `crates/forge-git/src/worktree_parse.rs` — pure helpers: porcelain parser, slugify, ref selection (`branch` → `refs/heads/...` or `HEAD sha`).
- `crates/forge-git/src/worktree_tests.rs` — end-to-end fixtures validating collision errors, dirty detection, conflicted merge abort, and base-removal refusal.

## Invariants & gotchas

- **Never silently reuse** an existing path or branch — both are explicit errors (`WorktreePathExists`, `BranchExists`).
- **Base worktree is sacred**: removing it is blocked client-side (git would allow it but leave the repo headless); all ops resolve to the base via `base_repo_root`.
- **Conflicted merges abort**: a merge that started but hit conflicts is always aborted (base tree restored, no MERGE_HEAD left behind) and returned as `MergeResult { merged: false, aborted: true, conflicts: [...] }`. A merge that never started (dirty base, bad ref) returns an error before MERGE_HEAD is written.
- **Detached worktrees** (no branch, e.g. ephemeral checkouts) are compared by HEAD sha; attempting to merge one errors (`WorktreeDetached`).
- **Ahead/behind is relative to base**: the base's own ahead/behind is always `(0, 0)`. Worktrees are measured against the base's `branch` (or its HEAD sha when the base is detached).
- **Path canonicalization**: macOS symlinks (`/var` vs `/private/var`) are resolved before comparisons to avoid false negatives on same-path checks.
