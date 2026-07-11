---
# Global App Store

## Purpose

The single SolidJS `createRoot(createStore)` singleton holding ALL application state — multi-context repo data, sessions, timeline, diff, feature list, UI layout (palette/sidebar/session pane widths), and daemon state. The `repo` getter is a DERIVED property that resolves to the active context's `RepoState`, allowing the app to track multiple open worktrees while existing views read `store.repo` without modification.

## How it works

- **Shape is frozen contract** — the 30+ fields in `AppStore` define the global state surface; mutations flow through slice functions, never written inline.
- **Four slices own actions** — `context-slice` (open/switch worktrees), `data-slice` (load features/timeline/diff for active context), `session-slice` (start/send/approve/interrupt/stop Claude sessions), `terminal-slice` (PTY tabs per worktree).
- **Derived `repo` getter** — reads `contexts.find(activeContextPath)?.state`, so top-level data always reflects the active worktree; non-active contexts reload on switch.
- **Event reducers** — global `initListeners` wires five Tauri IPC streams (`agent-event`, `timeline:event`, `index:progress`, `index:status`, `repo:changed`) to update the store as backend state mutates.
- **Toast lifecycle** — `pushError` / `pushSuccess` append to `toasts[]`; each auto-dismisses after 5s; `lastError` shadows the latest error toast.
- **Episode-keyed staleness** — `handleIndexStatus` ALWAYS updates `indexStatusByPath[repoPath]` (drives status-bar dot), but pops `staleModal` only ONCE per state episode (tracked in `shownStaleModals` set keyed as `${repoPath}:${status.state}`). A dismissed modal won't reappear while the backend poller re-emits the same `state` with a growing `changedFiles` list; when the state transitions to fresh/never the episode ends and the flag is cleared; when the user acts on the modal (`reindexStale`) the flag is also cleared so a future stale→fresh→stale cycle will re-prompt.

## Key files

- `crates/tauri-app/frontend/src/stores/app-store.ts` (core) — store shape, derived `repo`, slice assembly, event reducers, episode-keyed staleness logic, `createRoot` singleton export.
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — `openRepo`, `switchToContext`, `closeContext`, worktree picker state, git-init prompt.
- `crates/tauri-app/frontend/src/stores/data-slice.ts` — `refreshFeatures`, `refreshTimeline`, `refreshDiff`, `refreshDaemon`, feature mutations (reindex, pin, color, edit).
- `crates/tauri-app/frontend/src/stores/session-slice.ts` — `startSession`, `sendMessage`, `approveRequest`, **`interruptSession`** (abort in-flight turn, preserves transcript), `stopSession` (kill sidecar + clear session), `setSessionMode`, `renameSession`, `resumeSession`, `prefillComposer`, `handleAgentEvent` reducer.
- `crates/tauri-app/frontend/src/stores/terminal-slice.ts` — `createTerminal`, `closeTerminal`, `setActiveTerminal`, worktree-scoped PTY tabs.
- `crates/tauri-app/src/runtime/staleness.rs` — background poller task (`spawn_poller`), immediate first tick then every 10s, emits only when verdict changed, skips polls while reindexing, resets memo post-reindex.
- `crates/tauri-app/src/commands/sessions.rs` — backend session commands including `interrupt_session`, which sends `{"type":"abort"}` to sidecar and awaits `turn_aborted` response.

## Invariants & gotchas

- **Never write `store.repo` directly** — it's a getter; mutate `contexts[i].state` instead (or use `context-slice.handleRepoChanged`).
- **Live timeline events guard on `activeContextPath`** — `handleTimelineEvent` only appends if the event's repo matches the active context, else background worktrees leak events.
- **Stale-response guard** — async loads (feature doc hot-reload, timeline refetch) must check `store.selectedFeature === slug` or `samePath(activeContextPath, ...)` before `setStore`, or a slow response overwrites a user's navigation.
- **`initListeners` is idempotent** — called on every context open but registers IPC listeners only once (`listenersRegistered` flag), preventing duplicate handlers.
- **Episode-keyed modal suppression** — `shownStaleModals` is a runtime-only Set; it prevents re-showing the stale modal when the backend poller re-emits the same `state` with updated `changedFiles`, but does NOT prevent re-showing after a state round-trip (stale→fresh→stale) — the flag is cleared on state exit. `isStaleSuppressed` is a PERSISTENT localStorage check ("don't ask again" per repo) that survives app restarts.
- **Status consumption ≠ modal presentation** — `indexStatusByPath` is updated on EVERY `index:status` event (so the status bar dot tracks the latest `changedFiles` count); `staleModal` pops only once per state episode (episode = repoPath + status.state).
- **Session context tagging** — sessions carry `contextPath` so the UI can filter by worktree; sessions outlive context switches and are NOT auto-closed when a worktree is closed.
- **Poller lifecycle** — the `staleness_task` JoinHandle lives in `RepoRuntime` and is aborted on context close; the poller must not outlive its repo.
- **Reindexing skips polls** — while `reindexing` is true the poller skips the hash check (manifest is being rewritten), then resets its `last` memo so the first post-reindex verdict is always re-emitted (normally back to `fresh`, clearing the status-bar dot).
- **Interrupt vs Stop distinction** — `interruptSession(id)` sends `{"type":"abort"}` to the sidecar, aborting the in-flight turn only; the sidecar responds with `turn_aborted`, which finalizes the live message and resets `runState` to `ready` (transcript preserved). In contrast, `stopSession(id)` kills the sidecar and removes the session from `store.sessions` (transcript gone). SessionPane's composer stop button calls `interruptSession`; the tab strip's ✕ close button calls `stopSession` (destructive). This prevents "stop response" from wiping the chat — the abort pipeline existed but was never exposed as a UI action until now.
