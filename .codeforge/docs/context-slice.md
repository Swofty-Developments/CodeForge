---
## Purpose

Manages the multi-context model for FeatureForge: each open repository path (base repo + worktrees) is its own **context** with a separate backend runtime, feature index, timeline, and staleness poller. The active context determines what the UI displays; switching contexts reloads all per-context views.

## How it works

- **Contexts** are stored as `{state: RepoState, isBase: boolean}[]`; each context owns a distinct backend daemon keyed by its filesystem path. The base context is the main repository; worktrees are secondary contexts.
- **Opening a repo** (`openRepo`) or worktree (`openWorktreeContext`) calls `ipc.openRepo(path)`, spawns a backend runtime for that path (daemon, timeline/event forwarder, **staleness poller**), adopts it as a new context, and loads its features/timeline/diff/daemon state.
- **Staleness is polled per-context**: `spawn_poller(app, root, reindexing)` starts a `10s` tokio task immediately (covers on-open), re-hashes the manifest every `POLL_INTERVAL`, emits `index:status` only when the verdict changed (stale→fresh or fresh→stale), and skips polls while reindexing (resets its last-emit memo when reindexing completes so the new verdict is always surfaced). The `JoinHandle` lives in `RepoRuntime.staleness_task` and is aborted in `close_context`.
- **Frontend deduplicates stale modals per-episode**: `handleIndexStatus` separates **status consumption** (always updates `indexStatusByPath` and the status-bar dot) from **modal triggering** (show once per state episode). A modal pops when `state` is `stale` or `outdated`, the repo isn't suppressed, and this `repoPath:state` episode hasn't been shown yet (tracked in a local `Set`). When state transitions to `fresh`/`never`, the episode ends and the flag is cleared; when the user acts on the modal (reindex), the flag is cleared so a future stale→fresh→stale cycle will re-prompt. Growing `changedFiles` within one episode doesn't re-pop.
- **Switching contexts** (`switchContext`) is a cheap no-op if already active; otherwise it resets the view state (selected feature, diff, timeline) and reloads the four per-context datasets for the new active path. The `store.repo` getter always returns the active context's `RepoState`.
- **Closing a context** (`closeContext`) stops every session mapped to that path (a session can't outlive its context), aborts the timeline forwarder **and staleness poller**, shuts the daemon down, and removes the runtime. Closing the base context closes the entire repo family. Closing a worktree context never touches disk — `removeWorktreeExplicit` is the separate confirmed action that deletes a worktree from git/disk.
- **Sessions are global but tagged** with `contextPath`, so each context has its own session history. On context switch, `activeSessionId` is re-pointed to the most recent session for the new active path.
- **Worktree refresh** (`refreshWorktrees`) calls `ipc.listWorktrees(repo.path)` and reconciles each context's `isBase` flag from the git truth. Worktree creation/deletion actions refresh the worktree list to keep the tab strip current.

## Key files

- **`context-slice.ts`** (core) — all context lifecycle actions: open/close/switch contexts, create/remove worktrees, merge worktrees, git-init flow for non-git folders.
- **`app-store.ts`** — defines the store shape; `repo` is a derived getter that returns `contexts.find(active).state`, so existing views work transparently. Wires the context slice's refresh hooks to data-slice loaders. **Manages `indexStatusByPath`, `staleModal`, and the per-episode modal-trigger guard** (`handleIndexStatus` + `shownStaleModals` Set).
- **`repo_open.rs`** — backend `open_context` / `close_context` pair; spawns `staleness_task` in `RepoRuntime`, aborts it on close.
- **`staleness.rs`** — `spawn_poller(app, root, reindexing)`: tokio task that ticks every 10s, skips while reindexing (resets memo), emits only when verdict changed.
- **`state.rs`** — `RepoRuntime` struct owns `staleness_task: JoinHandle<()>`.

## Invariants & gotchas

- **The `repo` getter is DERIVED** — never set it directly; all writes go to `contexts[i].state`. Any code reading `store.repo` gets the active context's state.
- **`activeContextPath` determines visibility** — live timeline events (`handleTimelineEvent` in app-store) are appended only when their `repoPath` matches the active context; background contexts reload their timeline on switch. Appending events from every context would leak across worktrees.
- **Staleness poller is per-context, aborted on close** — the `staleness_task` JoinHandle is owned by `RepoRuntime` and aborted in `close_context`. Without the abort, a closed worktree would keep emitting `index:status` events forever.
- **Modal shows once per state episode** — `handleIndexStatus` tracks `repoPath:state` episodes in `shownStaleModals` (a local Set). First `stale`/`outdated` event in an episode pops the modal; subsequent events in the same episode update the dot but don't re-pop. When state transitions to `fresh`/`never`, the episode flag is cleared. When the user reindexes, the flag is cleared so future episodes re-prompt. This separates "consuming status" (always update `indexStatusByPath`) from "presenting the modal" (once per episode).
- **Context close ≠ worktree remove** — `closeContext` tears down a runtime tab but never touches disk. `removeWorktreeExplicit` is the separate, confirmed action that calls `ipc.removeWorktree` to delete from git/disk, then closes the context if it was open.
- **Base context close cascades** — closing a context where `isBase === true` triggers `closeRepo()`, which tears down every open context's runtime and resets the app to the welcome view.
- **Sessions are context-scoped** — on context switch, `activeSessionId` is re-pointed to `pickContextSession(path)` (the last session tagged with the new active path), so each context has independent session history.
- **Non-git folders trigger a git-init prompt** — `openRepo` catches the `"not_a_git_repo:..."` rejection and sets `initRepoPrompt` (a slice-local signal) instead of toasting an error. `InitRepoModal` reads this signal and lets the user confirm `ipc.initRepo(path)`, which creates a git repo and adopts it as a context exactly like a successful `openRepo`.
