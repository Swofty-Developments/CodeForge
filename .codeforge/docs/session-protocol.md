# Session Protocol

## Purpose

Bidirectional NDJSON protocol between the Rust daemon and a Node.js agent sidecar. Rust sends `query` and approval response commands over stdin; the sidecar emits 17+ streaming event types over stdout to drive the UI.

## How it works

- **One-time init augmentation**: The first `{"type":"query",...}` command gets `cwd`, `model`, `permissionMode`, and engagement mode (`fresh`/`resume`) injected from Rust state; subsequent queries in the same sidecar process are stamped `mode: continue` to keep the live SDK conversation going.
- **Parse sidecar stdout**: Each NDJSON line maps to zero or more `AgentEvent`s (text/thinking deltas, turn lifecycle, tool calls, usage, errors). Unparseable lines are skipped, never fatal—the SDK may print stray non-JSON.
- **Flat Tauri payload**: `AgentEvent` enums convert to `AgentEventPayload` (discriminated by `eventType` string, optional fields) for frontend demux by `sessionId`.
- **Session resume**: `SessionMode::Resume` stamps `resumeSessionId` on the first query; if the SDK can't honor it, `SessionResumeFailed` fires—no silent downgrade.
- **Empty delta filtering**: Zero-length text/thinking deltas are dropped at parse time to avoid no-op renders downstream.

## Key files

- `crates/forge-session/src/protocol.rs` — augments first query with init params, parses sidecar NDJSON into `AgentEvent`s
- `crates/forge-session/src/payload.rs` — flat Tauri wire format for frontend event demux
- `crates/forge-session/src/types.rs` — `AgentEvent` enum (17 variants for deltas, tool lifecycle, usage, errors)

## Invariants & gotchas

- **Init consumed exactly once**: `SidecarInitParams` wrapped in `Mutex<Option<_>>` fires on the first `query`, then becomes `None`; follow-up queries get `mode: continue` with no cwd/model.
- **Explicit fields win**: User-supplied `model`/`permissionMode` in a query command override init params (`entry().or_insert`); `cwd` and `mode` always stamped by Rust.
- **Non-query passthrough**: Commands of any type other than `query` (e.g., `abort`) are not augmented and do not consume the init slot.
- **Turn lifecycle guarantee**: `turn_completed` always fires (sidecar's `finally` block), even on errors or aborts.
- **Rust-originated events**: `session_persistence_degraded` exists only in Rust (failed DB write); it is never a sidecar out-event.
