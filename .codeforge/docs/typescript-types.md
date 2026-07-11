---
# TypeScript Types

**Purpose**

Frontend mirrors of Rust IPC types from `forge-core` and `forge-session`, marshaled via Tauri's serde-json bridge. The contract guarantees TypeScript can parse Rust serialized output (camelCase structs, snake_case enums) without coercion.

**How it works**

- **Frozen contract**: Rust `#[serde(rename_all = "camelCase")]` on structs, `snake_case` on enums; TypeScript mirrors the exact output shape (object keys camelCase, enum/union literals snake_case).
- **Discriminated unions**: `TimelineEvent` and `AgentEventPayload` are discriminated on `kind`/`eventType` so consumers can narrow to one concrete payload shape per event.
- **Explicit nullability**: `Option<T>` in Rust becomes `T | null` in TypeScript (never `undefined`); every optional field is documented, never guessed.
- **RFC3339 timestamps**: Rust `DateTime<Utc>` serializes to string; TypeScript leaves as `string`, UI code parses at render.
- **Payload as `serde_json::Value`**: `TimelineEvent.payload` is untyped on the Rust side; TypeScript exports one interface per event kind (`FileEditedPayload`, `NotePayload`, etc.) and uses discriminated unions to enforce the mapping.
- **Session agent events**: `AgentEventPayload` is a flat union from `forge-session`; the frontend demuxes by `sessionId` and `eventType`, building `SessionMessage.blocks` incrementally.

**Key files**

- `crates/tauri-app/frontend/src/types.ts` (core) — 412-line single-file mirror of all Rust domain types crossing the Tauri bridge.

**Invariants & gotchas**

- **Breaking sync**: any Rust field rename, enum case, or payload shape change breaks the frontend. Check both sides when modifying `Feature`, `TimelineEvent`, `SessionInfo`, `RepoState`, `AgentEventPayload`, or diff types.
- **Snake-case enums**: `FileRole = "core" | "support" | ...` (not `"Core"`); `Actor = "agent" | "human" | "system"` (not `"Agent"`). The Rust test suite asserts serialized form.
- **Never `undefined` for optionals**: Rust `Option<T>` serializes `null`, not JavaScript `undefined`. Use `?:` for object fields, `| null` for union types.
- **Payload per-kind shapes**: `FileEditedPayload` is `{ tool, path }`; `IndexCompletedPayload` is `{ features: number } | { error: string }`. Each timeline kind reads exactly one shape — the backend hook writes the matching keys.
- **UI shapes vs. wire shapes**: `SessionUi`, `ContentBlock`, `RunState` live only in the frontend (the reducer builds them from `AgentEventPayload` stream). Do not mirror these to Rust.
- **No schema validation**: the IPC layer does not validate payloads at runtime. A Rust-side payload key typo (e.g., `"path"` → `"filepath"`) silently becomes `undefined` in the frontend reducer — grep both sides when debugging missing fields.
- **Event wrapper payloads**: `index:status` and `timeline:event` are wrapped as `{ repoPath, status/event }` (ipc.ts:18-19, 223-225) so the frontend can filter events by which repo/context emitted them. Other channels ship unwrapped (e.g., `IndexProgress` alone on `index:progress`).
