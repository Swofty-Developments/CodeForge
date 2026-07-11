The changes this turn were entirely backend: adding a `check_claude_cli` Tauri IPC command and adding retry logic to the headless Claude indexer. The context slice itself was not modified — no changes to its logic, invariants, or responsibilities. The doc remains accurate.

---
# Context Slice

## Purpose

Manages the active repository context in a multi-context model (base repo + worktrees), each with its own feature index, timeline, and daemon runtime. Switching contexts reloads all per-context datasets and re-points the active session.

## How it works

- Each opened repository or worktree is a **context** with its own `RepoState`, stored in `store.contexts` and keyed by path; the store tracks `activeContextPath` to determine which context is live.
- Opening a repo (`openRepo`) creates the base context; opening a worktree (`openWorktreeContext`) adds a sibling context; each gets its own backend daemon via `ipc.openRepo(path)`.
- Switching contexts (`switchContext`) flips `activeContextPath`, resets the active feature/timeline/diff view state, re-points the active session to the most recent session tagged with that context path, and reloads the four per-context datasets (features, timeline, diff, daemon).
- Non-git folders trigger an `initRepoPrompt` signal instead of an error; the `InitRepoModal` reads it and calls `confirmInitRepo` to run `git init -b main` and adopt the result as a context.
- Closing a worktree context (`closeContext`) tears down its runtime and drops it from the store but NEVER touches disk; `removeWorktreeExplicit` is the separate, confirmed action that deletes the worktree from git/disk.
- Closing the base context closes the entire repo family (all worktree contexts); `closeRepo` tears down every runtime and resets the store to the welcome screen.

## Key files

- **`context-slice.ts`** — the entire slice; exports `createContextSlice` with `openRepo`, `switchContext`, `closeContext`, `createWorktree`, `mergeWorktree`, and `removeWorktreeExplicit`.

## Invariants & gotchas

- **Closing ≠ removing**: `closeContext` drops a context tab from the store and tears down its daemon; it NEVER deletes the worktree from disk. Only `removeWorktreeExplicit` deletes from git/disk, and it's always preceded by user confirmation.
- **Switching contexts must reset view state**: `resetContextView` wipes `selectedFeature`, `diff`, `featureActivity`, and `activeSessionId` so stale data from the previous context doesn't leak into the new one.
- **Sessions are global but context-tagged**: `activeSessionId` is re-pointed to the most recent session for the new context path via `pickContextSession`; sessions from other contexts remain in the store but are not shown.
- **`activeContextPath` is the single source of truth**: the derived `repo` getter in `app-store` returns `contexts.find(c => samePath(c.state.path, activeContextPath))`, so all views read the active context's state without passing `path` explicitly.
- **Base context is authoritative**: `baseContext()` returns the first `isBase` context or null; it's used as the merge target for `mergeWorktree` and as the primary path for `list_worktrees`. Closing the base closes the entire repo.
- **`reconcileContextBase()` refines `isBase` from git truth**: after `list_worktrees` returns, each open context's `isBase` flag is reconciled with the worktree list's `isBase` field so the authoritative base is always correct.
- **Non-git folders are not dead ends**: if `openRepo` rejects with `"not_a_git_repo:"`, the slice captures the path in `initRepoPrompt` instead of toasting an error, so the `InitRepoModal` can offer `git init` and recover.
