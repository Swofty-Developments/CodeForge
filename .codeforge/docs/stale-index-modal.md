# Stale Index Modal

## Purpose

Prompts the user to re-index when the feature index becomes **stale** (files changed outside CodeForge, e.g. after `git pull`) or **outdated** (indexed by an older CodeForge version). Separates modal presentation (show once per episode) from status tracking (always consumed, drives the status-bar dot).

## How it works

- Backend emits `index:status` events via a 10s poller whenever staleness state changes.
- `app-store.ts` consumes these events, updating `indexStatusByPath` for the status-bar dot and conditionally showing the modal.
- Modal pops when state is `stale` or `outdated`, user hasn't suppressed this repo (`localStorage`), and this episode hasn't been shown yet (`shownStaleModals` set).
- Episode key is `${repoPath}:${state}` — when state transitions back to `fresh`/`never`, the flag is cleared so the next stale cycle will re-prompt.
- User can "Re-index" (force re-index, clears episode flag), "Not now" (dismisses modal), or "Don't ask again" (persists suppression to `localStorage`).
- **Stale** vs **outdated** drives different copy: stale shows changed file list (up to 6, with overflow count); outdated shows version jump (`v{old}` → `v{new}`), no file list.

## Key files

- `frontend/src/components/StaleModal.tsx` — modal component; reads `store.staleModal`, renders the prompt, calls store actions.
- `frontend/src/stores/app-store.ts` — owns `staleModal` state, `handleIndexStatus` event consumer, `dismissStaleModal`/`suppressStale`/`reindexStale` actions.
- `frontend/src/types.ts` — `IndexStatus` type (`state: "never" | "fresh" | "stale" | "outdated"`, `changedFiles`, version numbers).

## Invariants & gotchas

- **Once-per-episode guarantee**: `handleIndexStatus` tracks `shownStaleModals` per `${repoPath}:${state}` so a single stale episode doesn't spam modals on every 10s poll. Must clear the flag when state becomes non-actionable or user re-indexes.
- **Suppression is repo-scoped**: `localStorage` key is `stale-modal-suppress-${normPath(repoPath)}`. If user suppresses, modal never shows again for that repo until they clear local storage.
- **Episode ends on fresh/never**: when `status.state` transitions away from `stale`/`outdated`, delete the episode key from `shownStaleModals` so the next stale cycle will re-prompt.
- **Modal can reference a non-active context**: `reindexStale(repoPath)` targets the modal's repo, not necessarily the active context — they differ per worktree. Don't assume `store.repo.path === store.staleModal.repoPath`.
- **File list overflow**: capped at 6 files, "+ N more" suffix; right-to-left text direction so truncation keeps the filename visible.
