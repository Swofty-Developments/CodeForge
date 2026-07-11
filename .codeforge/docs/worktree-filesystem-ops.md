# Worktree Filesystem Ops

## Purpose

Handles CodeForge-specific filesystem setup for newly created worktrees: copies the base repository's feature index into the worktree so it inherits the feature model immediately, and ensures `.codeforge-worktrees/` is ignored in the base repo's `.gitignore`.

## How it works

- **Feature inheritance**: `inherit_features` copies `.codeforge/features.json` from the base repo into the worktree's `.codeforge/` directory, creating parent dirs as needed. Returns `Ok(false)` if the base is not yet indexed (no `features.json` exists) — a valid state, not an error.
- **Gitignore management**: `ensure_worktrees_ignored` idempotently appends `.codeforge-worktrees/` to the base repo's `.gitignore`, creating the file if missing and preserving existing content.
- **Separation of concerns**: This module is pure filesystem ops — no git commands. Git worktree creation/removal lives in `forge_git`.
- **Contract W4**: Every worktree is created with the base's feature model already in place, avoiding a cold-start indexing delay when the worktree is first opened.

## Key files

- **crates/tauri-app/src/runtime/worktree_fs.rs** — feature inheritance, gitignore management, and OS-agnostic path handling for worktree filesystem setup.

## Invariants & gotchas

- **Idempotency**: `ensure_worktrees_ignored` must not duplicate the ignore entry on repeated calls — it checks for the line before appending.
- **No-op on unindexed base**: `inherit_features` returns `Ok(false)` when the base has no `features.json` yet; the worktree starts unindexed and will cold-start on open. This is NOT an error state to hide.
- **Newline preservation**: `ensure_worktrees_ignored` adds a trailing newline if the existing `.gitignore` doesn't end with one, preventing malformed ignore files.
- **No git calls**: This module must remain pure filesystem — no `git` commands, no `Repository` handles. Git operations belong in `forge_git::worktree`.
- **Features.json is copied verbatim**: The worktree gets the base's exact feature index at creation time. Subsequent updates in the base do not auto-propagate — the worktree's index is independent after creation.
