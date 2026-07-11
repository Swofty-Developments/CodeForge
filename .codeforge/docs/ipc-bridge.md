---
# IPC Bridge

## Purpose

Typed TypeScript wrappers around Tauri's `invoke()` and `listen()` for all backend commands and event channels. Components never call Tauri APIs directly — this module is the single source of truth for the frontend/backend contract.

## How it works

- Wraps every command registered in `main.rs` (`open_repo`, `get_features`, `start_session`, etc.) as a typed async function that calls `invoke(command_name, args)`.
- Tauri auto-converts camelCase JS arguments to snake_case Rust parameters (`repoPath` → `repo_path`).
- All functions return `Promise<T>` and throw on error (Tauri serializes Rust `Result::Err` into a JS rejection).
- Event listeners (`listenAgentEvent`, `listenTimelineEvent`, etc.) wrap Tauri's `listen<T>(channel)` and return an unlisten function.
- Type imports come from `types.ts`, which mirrors Rust structs/enums from `forge-core` and `forge-session`.
- The command set is frozen per major version — adding a new command requires updating both `main.rs`'s `invoke_handler!` macro and this file.

## Key files

- **crates/tauri-app/frontend/src/ipc.ts** (core) — All 30+ command wrappers and 7 event listeners.
- **crates/tauri-app/frontend/src/types.ts** — TypeScript mirrors of Rust IPC types (Feature, TimelineEvent, SessionInfo, etc.).
- **crates/tauri-app/src/main.rs** — Registers the backend command handlers via `tauri::generate_handler!`.
- **crates/tauri-app/src/commands/** — Rust command implementations that these wrappers invoke.
- **crates/tauri-app/src/runtime/staleness.rs** — Background poller that drives `index:status` events.

## Invariants & gotchas

- **Never call `invoke()` or `listen()` directly from components** — always add a typed wrapper here first.
- **Command names are the frozen contract** — renaming one breaks all clients until they update. The list in the file header is the authoritative registry.
- **Event payloads must stay in sync** — if Rust changes a payload shape, update `types.ts` and this file's listener signatures simultaneously.
- **Tauri's camelCase→snake_case is automatic** — manually snake_casing arg keys (`repo_path: x`) will double-convert to `repo__path` and fail.
- **Optional params use `null`, not `undefined`** — Rust serde expects `Option<T>` to deserialize from JSON `null`; `undefined` is not serialized and the param vanishes, breaking commands that distinguish absent vs. null.
- **Terminal data is BASE64-encoded** — `terminal:data` payloads carry `data: string` that must be decoded before writing to xterm.
- **Timeline events are per-context** — `timeline:event` payloads include `repoPath` so the frontend can filter events for the active repo.
- **`index:status` is a POLLING event** — emitted immediately on `open_repo` (by the first tick of the 10s poller), then re-emitted whenever the staleness verdict changes. Frontend handlers that pop modals must guard on state transition (`prev.state !== status.state`) so a dismissed stale-index modal isn't re-shown every 10s while `changedFiles.length` grows.
- **`index:status` is silent during reindex** — the poller skips ticks while `reindexing` is set; the first post-reindex tick always re-emits (normally fresh, clearing the UI dot). Don't assume silence means fresh.
