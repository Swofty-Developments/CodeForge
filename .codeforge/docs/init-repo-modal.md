# Init Repo Modal

## Purpose

Intercepts `open_repo` failures when the user tries to open a non-git directory. Offers to run `git init -b main` instead of surfacing a dead-end error, then opens the newly initialized repo exactly like a normal repo-open success.

## How it works

- `open_repo(path)` rejects with the machine-matchable prefix `"not_a_git_repo:"` when the target is a plain folder
- The context slice catches this prefix, suppresses the toast, and sets `initRepoPrompt` signal to the path instead
- `InitRepoModal` renders when the signal is non-null; displays path (dir-truncated), explains why git is required, offers Cancel / Initialize buttons
- Confirm → `confirmInitRepo()` calls `init_repo(path)` IPC, which runs `git init -b main` via `forge_git::init_repo` then hands the path to the SAME `repo_open::open_context` flow that `open_repo` uses
- Cancel → `dismissInitRepoPrompt()` clears the signal; the dialog closes with no side effects (path is NOT opened)
- After successful init, the repo is adopted as a context (base if first, worktree-context otherwise) and all per-context datasets (features, timeline, diff, daemon) load automatically

## Key files

- `crates/tauri-app/frontend/src/components/InitRepoModal.tsx` — modal UI; reads `initRepoPrompt` signal from context-slice, calls `confirmInitRepo` or `dismissInitRepoPrompt`
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — owns the `initRepoPrompt` signal, defines `confirmInitRepo` / `dismissInitRepoPrompt`, catches the `"not_a_git_repo:"` prefix in `openRepo`
- `crates/tauri-app/src/commands/repo.rs` — `open_repo` command returns the prefixed error; `init_repo` command runs `git init -b main` then opens via `repo_open::open_context`
- `crates/forge-git/src/lib.rs` — `init_repo(&Path)` spawns `git init -b main`

## Invariants & gotchas

- **`"not_a_git_repo:"` is the only trigger** — the prefix must match EXACTLY in `open_repo` and `context-slice.ts`. Do not generalize the error or the modal will never appear.
- **Busy state blocks double-init** — the Initialize button is disabled while `busy()` is true; prevents overlapping `git init` calls on slow filesystems.
- **No partial state** — either the full flow (init → open context → load datasets) succeeds, or the prompt is dismissed and the store is unchanged. Do not leave `initRepoPrompt` dangling after a failed init.
- **Path display uses RTL truncation** — `direction: rtl; text-align: left` shows the tail of long paths (most meaningful part), not the head.
- **Already-initialized repos are named errors** — `init_repo` rejects with a human-readable message when `is_git_repo` is already true; this is an invariant violation (the frontend should never call `init_repo` on a git repo), but the command handles it gracefully.
- **The modal is global** — mounted in `App.tsx`, shown via `.overlay` system (same layer as `StaleModal` / `MergeResultPanel`). Only one can be active at a time; `initRepoPrompt` being non-null renders it on top of everything except the command palette.
