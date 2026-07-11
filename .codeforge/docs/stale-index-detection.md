# Stale Index Detection

## Purpose

Compares on-disk index metadata (version, file hashes) against the current repo state to detect whether the index is fresh, stale, outdated, or never built. When the verdict changes, emits a frontend event that triggers the stale-index modal, prompting the user to re-index.

## How it works

- A background poller spawned per open repo checks staleness every 10 seconds using `forge_index::index_status`.
- The check runs off the async runtime (blocking sha256 hash) and compares `.codeforge/index-meta.json` to the current manifest.
- Emits `index:status` event with `{ repoPath, status }` only when the verdict **changes** from the last emission.
- While `reindexing` is true, polling is skipped and `last` verdict is reset so the first post-reindex check always re-emits (typically `Fresh`).
- First tick fires immediately on repo open; subsequent ticks run every 10s (missed ticks delay rather than burst).
- Probe failures are logged but do not stop the poller or fabricate a substitute verdict.

## Key files

- `crates/tauri-app/src/runtime/staleness.rs` — spawns the per-repo 10s poller, computes status off-runtime, emits `index:status` on changes.
- `crates/forge-index/src/meta.rs` — defines `IndexStatus` (verdict: `Fresh`, `Stale`, `Outdated`, `Never`) and `index_status` hashing logic.
- `crates/tauri-app/frontend/src/stores/app-store.ts` — consumes `index:status` events, stores per-repo status, drives the status-bar freshness dot.
- `crates/tauri-app/frontend/src/ipc.ts` — typed `index:status` event listener registration.

## Invariants & gotchas

- **Never re-indexes autonomously** — only the UI decides when to re-index; backend is read-only probe.
- **Polling must skip during re-index** — mid-reindex the manifest is being rewritten, so a verdict against old meta is noise. `reindexing` flag guards this.
- **`last` must be reset when skipping** — ensures first post-reindex verdict always emits (clears status-bar dot when back to `Fresh`).
- **First tick fires immediately** — covers the on-open push; `MissedTickBehavior::Delay` prevents burst if hash overruns 10s.
- **Only emit on change** — steady state (fresh or already-stale) stays silent so UI isn't re-prompted every 10s.
- **Poller lifetime tied to `RepoRuntime`** — the `JoinHandle` is owned by the runtime and aborted on repo close; must not outlive its context.
- **No error state in `IndexStatus`** — probe failures log to tracing but don't fabricate a verdict; UI can separately call the `index_status` command for error surfacing.
