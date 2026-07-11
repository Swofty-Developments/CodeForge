# IPC Bridge

## Purpose

Type-safe TypeScript wrappers around all Tauri `invoke` commands and `listen` event channels. Components never call `invoke()` directly; the bridge enforces that every Rust IPC command has a matching TypeScript function with the correct signature.

## How it works

- Tauri auto-converts camelCase JS args to snake_case Rust params (`repoPath` → `repo_path`).
- Every IPC command is a named function (e.g., `openRepo`, `startSession`) that calls `invoke` with the command string and typed arguments.
- Event listeners are wrapped functions (e.g., `listenAgentEvent`, `listenTimelineEvent`) that call `listen<Payload>(channel, handler)` and return the unlisten function.
- The `types.ts` file mirrors Rust serde types with camelCase structs and snake_case enums, kept manually in sync with `forge-core` and `forge-session`.
- All registered commands are documented in the top comment as a frozen contract, mapping to their `main.rs` registration in `tauri-app`.

## Key files

- `crates/tauri-app/frontend/src/ipc.ts` — typed invoke wrappers for all commands; typed listen helpers for all event channels.
- `crates/tauri-app/frontend/src/types.ts` — TypeScript mirrors of Rust IPC types (Feature, RepoState, AgentEventPayload, etc.); discriminated unions for timeline events.

## Invariants & gotchas

- **Frozen command contract**: the list of registered commands in `main.rs` is the single source of truth; adding a command requires updating both Rust and TypeScript.
- **Manual type sync**: `types.ts` must be kept in sync with `forge-core` and `forge-session` Rust types by hand; there's no codegen.
- **Never call `invoke` or `listen` directly**: all Tauri IPC must go through `ipc.ts` to preserve type safety and maintain the contract.
- **Base64 terminal data**: `terminal:data` events carry BASE64-encoded PTY output; decode before passing to xterm.
- **Null vs undefined**: optional Rust `Option<T>` fields become `T | null` in TypeScript; pass `null` (not `undefined`) for absent values.
- **Event demultiplexing**: `agent-event` is a single channel multiplexed by `sessionId`; frontend reducers must filter on it. `timeline:event` and `index:status` carry `repoPath` to route events to the correct context.
