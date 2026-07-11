# Worktree Lifecycle

## Purpose

Manages git worktree add/remove/list operations with named error states for every failure mode. Creates worktrees under `.codeforge-worktrees/<slug>`, refuses to remove the base, prevents double-checkouts, and detects dirty trees.

## How it works

**List:** `git worktree list --porcelain` → parsed into `RawWorktree` → enriched with ahead/behind (via `git rev-list --left-right --count base...branch`), dirty state (`git status --porcelain`), and is-base flag. Base worktree is always first.

**Create:** slugifies the name (lowercase alphanumeric, `-` separator), checks the `.codeforge-worktrees/<slug>` path doesn't exist, verifies the `refs/heads/<slug>` branch doesn't exist, resolves the base ref (current branch or HEAD sha when detached), then runs `git worktree add -b <slug> <path> <base-ref>`. Returns enriched worktree with path canonicalized.

**Remove:** refuses if the path is the base worktree (via `same_path` canonicalization check), otherwise runs `git worktree remove [--force] <path>`. Git's own errors (dirty without force) surface verbatim.

**Merge:** locates the worktree by path, extracts its branch (errors if detached), extracts base's current branch (errors if detached), runs `git merge --no-edit <source>`. On conflict: captures conflicted paths via `git diff --name-only --diff-filter=U`, runs `git merge --abort` to restore the base, and returns `MergeResult{merged: false, aborted: true, conflicts}`. If merge never started (no `MERGE_HEAD` — dirty base or bad ref), surfaces the error without calling abort.

**Ahead/behind:** when base and branch share no merge-base, `(0, 0)` is returned — a named, honest state, not papered over.

## Key files

- `crates/forge-git/src/worktree.rs` — core ops: `list_worktrees`, `create_worktree`, `remove_worktree`, `merge_worktree`, `base_repo_root`
- `crates/forge-git/src/worktree_parse.rs` — pure helpers: `parse_worktree_porcelain`, `compare_ref`, `slugify`, `dir_name`
- `crates/forge-git/src/worktree_tests.rs` — end-to-end tests against real git repos in tempdirs

## Invariants & gotchas

- Base worktree is always first in `list_worktrees` and cannot be removed (explicit error).
- Paths are canonicalized via `std::fs::canonicalize` for macOS symlink quirks (`/var` vs `/private/var`).
- Creating a worktree when the path OR branch already exists is an error — never silently reused.
- Merge conflicts are reported AND aborted — the base is never left mid-merge. If the merge can't start (dirty base, bad ref), the error surfaces without calling `git merge --abort`.
- Slugify returns empty string for non-ASCII names → `InvalidName` error. Runs of non-alphanumeric chars collapse to a single `-`.
- `ahead_behind` returns `(0, 0)` when two refs share no merge-base — the caller sees it as a named state, not a hidden fallback.
- Dirty detection runs `git status --porcelain` per worktree; bare worktrees are never dirty.
