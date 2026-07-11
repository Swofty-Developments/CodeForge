---
# Stale Index Modal

## Purpose

Prompts the user to re-index when the feature index is out of date — either **stale** (files changed outside CodeForge hooks, e.g. `git pull`) or **outdated** (indexed by an older CodeForge version). The user can re-index now, defer, or suppress the prompt for that repo.

## How it works

- Backend spawns a **per-repo background poller** via `spawn_poller(app, root, reindexing)` when a repo context opens. The poller computes `IndexStatus` (SHA256 manifest hash vs. `.codeforge/index-meta.json` version) every 10s on a tokio interval.
- First tick fires **immediately** (covers the on-open push); subsequent ticks run every `POLL_INTERVAL` (10s).
- Emits `index:status` event **only when the verdict changes** (via `last != current` comparison) — a steady state (fresh or already-reported stale) stays silent, preventing re-prompts on every tick.
- While `reindexing` is set, the poll is skipped entirely (mid-reindex the manifest is being rewritten) and `last` is reset to `None` so the first post-reindex verdict is always re-emitted (normally fresh, clearing the status-bar dot).
- Frontend `handleIndexStatus` updates `indexStatusByPath[repoPath]` (drives status-bar dot) on every event, but pops the modal **only on a state transition** (`prev.state !== status.state`) so a dismissed prompt doesn't reappear while the user keeps editing.
- Modal displays staleness reason: for **stale**, lists up to 6 changed files; for **outdated**, shows old vs. new version numbers.
- User actions: **Re-index** calls `reindexRepo(repoPath, force: true)`; **Don't ask again** writes to `localStorage` keyed by repo path; **Not now** dismisses without suppression.

## Key files

- `crates/tauri-app/src/runtime/staleness.rs` — `spawn_poller(app, root, reindexing)` returns `JoinHandle`, `compute(root)` hashes off async runtime, emits `index:status` on verdict change
- `crates/tauri-app/src/runtime/repo_open.rs` — `open_context` spawns the poller at line 132 and stores its `JoinHandle` in `RepoRuntime.staleness_task`; `close_context` aborts it
- `crates/tauri-app/frontend/src/components/StaleModal.tsx` — modal UI, renders staleness reason and action buttons
- `crates/tauri-app/frontend/src/stores/app-store.ts` — `handleIndexStatus` filters (transition-only, non-suppressed), stores status per repo, pops modal

## Invariants & gotchas

- **Poller must not outlive repo**: `JoinHandle` lives in `RepoRuntime.staleness_task` and is aborted in `close_context`.
- **Modal is keyed to repo path, not active context**: a worktree can trigger a modal for its base repo; `reindexStale` targets the modal's `repoPath`, not `activeContextPath`.
- **Suppress is per-repo, persisted in localStorage**: `suppressStale` writes `stale:<normalized-path>: "1"` and survives restarts.
- **Only emit on verdict transitions**: poller compares `last: Option<IndexStatus>` to current; re-emitting the same state would re-open dismissed modals.
- **Skip polling during reindex**: `reindexing` flag prevents hashing a half-written manifest; `last` is reset to `None` so the post-reindex verdict always emits (usually fresh).
- **Frontend transition filter**: `handleIndexStatus` pops the modal only if `prev.state !== status.state` so a growing `changedFiles` list (re-emitted every 10s by the poller) updates the status-bar dot without re-prompting.
- **File list is capped at 6 visible + count**: `MAX_LISTED` prevents unbounded DOM growth; direction is RTL so the basename (right side) shows when paths are ellipsized.
- **First tick fires immediately**: `ticker.set_missed_tick_behavior(Delay)` ensures on-open push and correct interval cadence even if a hash overruns 10s.
