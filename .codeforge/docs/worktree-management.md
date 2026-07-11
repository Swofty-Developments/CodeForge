# Worktree Management

## Purpose

Provides git worktree lifecycle operations (list, create, remove, merge) for isolating parallel work streams. Creates worktrees under `.codeforge-worktrees/` with slugified branch names and surfaces all errors explicitly — detached HEAD, merge conflicts, no merge-base — never papered over with silent fallbacks.

## How it works

- **List**: Parses `git worktree list --porcelain` into enriched `Worktree` structs with ahead/behind counts (measured against the base worktree's branch via `git rev-list --left-right --count`) and dirty-tree detection (`git status --porcelain`).
- **Create**: Checks for path/branch collisions, resolves `base_ref` (explicit, current branch, or HEAD sha), runs `git worktree add -b <slug>` under `.codeforge-worktrees/<slug>`, then returns the enriched entry after canonicalizing the path.
- **Remove**: Refuses to remove the base worktree; delegates to `git worktree remove [--force]` and surfaces git's errors (dirty tree, etc.) verbatim.
- **Merge**: Runs `git merge --no-edit <source_branch>` in the base worktree. On conflict, extracts conflicted paths via `diff --name-only --diff-filter=U`, aborts the merge with `git merge --abort` to leave the base clean, and returns a `MergeResult` with `{merged: false, aborted: true, conflicts: […]}`. Pre-flight failures (dirty base, bad ref) error before MERGE_HEAD exists and skip the abort.
- **Base repo resolution**: All ops key off the first entry of `git worktree list --porcelain` (the main worktree), not the CWD — allows invoking from any worktree or subpath.
- **Path canonicalization**: Freshly created worktrees canonicalize symlinks (macOS `/var` → `/private/var`) before comparison with `list_worktrees` to avoid phantom-missing entries.

## Key files

- `worktree.rs` — public API: list, create, remove, merge, base_repo_root; ahead/behind measurement; merge conflict detection & abort
- `worktree_parse.rs` — pure helpers: parse `--porcelain` output, slugify names, extract compare-ref (branch or HEAD sha)
- `worktree_tests.rs` — end-to-end tests against real git repos in tempdirs

## Invariants & gotchas

- **Base worktree is first**: `git worktree list --porcelain` always emits the main worktree first; the code relies on this ordering.
- **No silent reuse**: Existing path → `WorktreePathExists`; existing branch → `BranchExists`. The caller must explicitly handle collisions; create_worktree never deletes or reuses.
- **Merge conflicts abort atomically**: A conflicted merge extracts `conflicts`, runs `git merge --abort`, and returns `MergeResult{merged: false, aborted: true, conflicts}`. The base is never left mid-merge.
- **MERGE_HEAD probe distinguishes pre-flight failure from conflict**: If `git merge` fails but MERGE_HEAD doesn't exist, the merge never started (dirty base / bad ref), so surfacing the error without calling `merge --abort` avoids a spurious command.
- **Detached worktrees error on merge**: Merge requires both base and source to have branches. A detached worktree returns `WorktreeDetached`; a detached base returns `BaseDetached`.
- **Canonical path comparison**: `same_path(a, b)` canonicalizes both sides to handle symlinks; raw `PathBuf` equality would miss macOS `/var` vs `/private/var`.
- **No commits share no merge-base**: `ahead_behind` returns `(0, 0)` when `git merge-base` fails — an unborn or disjoint history is a named state, not an error.
