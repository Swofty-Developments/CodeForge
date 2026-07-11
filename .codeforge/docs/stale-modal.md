# Stale Modal

## Purpose

Prompts the user when the on-disk feature index for a repository is outdated or stale. Two states trigger the modal:
- **Stale** — files changed outside CodeForge (e.g. after a `git pull`); the in-memory index may be out of sync.
- **Outdated** — the index was written by an older CodeForge version; re-indexing will use the latest format.

The user can re-index now, defer ("Not now"), or suppress future prompts for this repo ("Don't ask again").

## How it works

1. **Detection**: `app-store.ts` calls `setStaleModal(repoPath, status)` when `IndexStatus.state` is `"stale"` or `"outdated"`. Checks a per-repo localStorage suppression flag (`isStaleSuppressed`) and an in-memory episode dedup set (`shownStaleModals`) before showing.

2. **Rendering**: `StaleModal.tsx` reads `store.staleModal` and renders a modal overlay with:
   - Icon + title (amber for stale, primary accent for outdated)
   - Body copy naming the repo and explaining the concrete state
   - For stale: a scrollable list of changed files (max 6 visible, "+N more" overflow)
   - For outdated: old → new version labels
   - Three actions: "Don't ask again" (suppress), "Not now" (dismiss), "Re-index" (force reindex)

3. **Actions**:
   - `suppressStale(repoPath)` writes `localStorage.setItem(staleKey(repoPath), "1")` and closes the modal
   - `dismissStaleModal()` clears `store.staleModal` (no persistence)
   - `reindexStale(repoPath)` deletes the episode key from `shownStaleModals`, closes the modal, calls `ipc.reindexRepo(repoPath, true)` with force flag

4. **Dedup**: The episode key is `"${repoPath}:${status.state}"` — if the same repo goes stale twice in one session (e.g. two pulls), the modal shows again. Clicking "Re-index" clears the flag so a future stale episode can re-prompt.

## Key files

- `StaleModal.tsx` (core) — modal UI, scoped styles, action handlers
- `app-store.ts` (state) — `staleModal` slice, `setStaleModal`/`dismissStaleModal`/`suppressStale`/`reindexStale`, localStorage suppression, episode dedup
- `types.ts` — `IndexStatus` shape (state, changedFiles, indexVersion, currentVersion)
- `App.tsx` — renders `<StaleModal />` in the overlay layer

## Invariants & gotchas

- **Episode dedup**: `shownStaleModals` is an in-memory `Set<string>` — survives across worktree switches within one session, resets on app restart. A repo that goes stale → fresh → stale again will re-prompt (different episode key).
- **Per-repo suppression**: `localStorage.getItem(staleKey(repoPath))` is checked before showing. "Don't ask again" persists across sessions; "Not now" does not.
- **Worktree isolation**: `reindexStale` targets the modal's `repoPath`, not necessarily the active worktree's context. A worktree's base repo can show the modal even when a different worktree is active.
- **File list direction**: Changed file paths render with `direction: rtl` so the filename (rightmost segment) stays visible when truncated.
- **Only two states trigger**: `state === "never"` (no index yet) and `state === "fresh"` (current) never show the modal. The backend must call `setStaleModal` explicitly when status is `"stale"` or `"outdated"`.
