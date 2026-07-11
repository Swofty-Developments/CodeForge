# Worktree Filesystem Ops

## Purpose

Pure filesystem setup for CodeForge worktrees: copies `.codeforge/features.json` from base repo to new worktree (so it inherits the feature model immediately), and ensures `.codeforge-worktrees/` is ignored in the base repo's `.gitignore`.

## How it works

- `inherit_features(base, worktree)` copies `.codeforge/features.json` from base into the worktree's `.codeforge/` directory, creating parent dirs as needed. Returns `Ok(false)` (not an error) when the base has no `features.json` yet — the worktree starts unindexed and gets its own cold-start on open.
- `ensure_worktrees_ignored(base)` appends `.codeforge-worktrees/\n` to the base repo's `.gitignore` if not already present. Idempotent: repeated calls do not duplicate the entry. Creates `.gitignore` if missing.
- Both are called by `commands/worktrees.rs::open_new_worktree()` immediately after `git worktree add` and before opening the worktree as its own `RepoRuntime`.
- This module is **pure fs** — no git operations, no IPC. Git worktree primitives live in `forge_git`.

## Key files

- `crates/tauri-app/src/runtime/worktree_fs.rs` (core) — the two pure-fs setup functions, no side effects beyond disk writes.
- `crates/tauri-app/src/commands/worktrees.rs` — IPC commands that call `inherit_features` and `ensure_worktrees_ignored` after creating a worktree.

## Invariants & gotchas

- **Base-not-indexed is not an error**: `inherit_features` returns `Ok(false)` when the base has no `features.json` yet. The worktree starts unindexed and gets its own cold-start on open — do not treat this as a failure case.
- **Idempotent ignore rule**: `ensure_worktrees_ignored` must not duplicate the `.codeforge-worktrees/` line on repeated calls. It scans existing lines before appending.
- **No git coupling**: this module must remain pure fs — no `git` commands, no `forge_git` imports. Git worktree primitives belong in `forge_git`, not here.
- **Call order matters**: the base repo's `.codeforge-worktrees/` ignore must be added **before** creating worktrees, or the worktree dir may briefly be visible to git. The IPC layer (`commands/worktrees.rs`) enforces this by calling `ensure_worktrees_ignored` in `open_new_worktree`, which runs immediately after `git worktree add`.
