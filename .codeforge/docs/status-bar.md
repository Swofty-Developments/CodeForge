---
# Status Bar

**Purpose**  
Single 24px consolidated status surface at the bottom of the window. Aggregates daemon health, index freshness/progress, active session count, and terminal visibility; hover for detail popover; one reindex button.

**How it works**  
- Aggregates status by priority: indexing in progress → daemon down → index stale/outdated → all good. Renders one dot (green/amber/red/blue/gray) + label.
- Hover the left status control to reveal a detail popover: repo name, branch, daemon port, feature count, index age, freshness reason, changed-file list (first 4), reindex button.
- Right side shows session count + terminal toggle button (displays terminal count badge when any exist).
- Animated activity line (2px gradient) appears above the bar when any session is `runState === "generating"`.
- Freshness dot is driven by `store.indexStatusByPath[activeContextPath]`, which is kept current by a backend staleness poller (not just on repo open): the poller re-hashes the manifest every 10s and emits `index:status` only when the IndexStatus verdict changed since the last emission. While a reindex is running, the poller skips checks (manifest is mid-rewrite), resets its memo, and re-emits the post-reindex state (normally fresh, clearing the dot).
- Polls `store.daemon`, `store.indexProgress`, `store.indexStatusByPath[activeContextPath]`, `store.repo`, `store.sessions`, `store.terminalPanelOpen`, `store.terminals[]` from the app-store; all status is derived, never mutated locally.

**Key files**  
- `crates/tauri-app/frontend/src/components/StatusBar.tsx` — the entire component (aggregate logic, dot severity, popover detail, activity line, scoped styles)
- `crates/tauri-app/frontend/src/stores/app-store.ts` — global state: `daemon`, `indexProgress`, `indexStatusByPath`, `repo`, `sessions`, `terminalPanelOpen`, `terminals[]`; `handleIndexStatus` updates `indexStatusByPath` on every `index:status` event (so the status-bar freshness dot reacts to external edits)
- `crates/tauri-app/src/runtime/staleness.rs` — `spawn_poller(app, root, reindexing)` launches a per-repo tokio task that polls every 10s, emits `index:status` only on verdict change, skips polls while reindexing
- `crates/tauri-app/src/state.rs` — `RepoRuntime.staleness_task` holds the poller's JoinHandle; aborted on context close
- `crates/tauri-app/frontend/src/types.ts` — `DaemonState`, `IndexStatus`, `IndexProgress`, `RepoState` type definitions

**Invariants & gotchas**  
- All status lives here — title bar and sidebar must not carry their own dots; the status bar is the single source of truth.
- Severity aggregation order is load-bearing: `indexing → daemon-off → stale/outdated → ok`. Changing priority breaks the user's mental model of "what's most urgent right now."
- The `aggregate()` function must never reference `store` directly; it's a pure function accepting explicit arguments so memoization works.
- `indexing` is `store.indexProgress != null && stage !== "error"` — a progress with stage `"error"` does NOT count as busy, so the dot shows red/warn instead of pulsing blue.
- Reindex button is disabled during indexing (prevents double-trigger); the popover stays open while reindexing so progress is visible.
- The activity line animates only when `sessions.some(s => s.runState === "generating")`; respects `prefers-reduced-motion`.
- The staleness poller is PER REPO (one JoinHandle in each RepoRuntime), not global. Multi-context apps poll each repo independently; aborted when the repo is closed.
- `handleIndexStatus` updates the store's `indexStatusByPath` map on every event, but only pops the StaleModal on a state transition (`prev.state !== status.state`), so a growing `changedFiles` list re-emitted by the 10s poller updates the status-bar dot without re-prompting the user every tick.
