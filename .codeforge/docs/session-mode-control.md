# Session Mode Control

## Purpose

Defines the engagement modes (fresh/resume/continue) and permission modes (default/acceptEdits/plan/bypassPermissions) that govern how the Rust app interacts with the Claude Agent SDK, replacing the old resume-fallback cascade with explicit DB-authoritative decisions.

## How it works

- **SessionMode** enum distinguishes three SDK engagement modes: `Fresh` (new session, no resume), `Resume` (re-engage a recorded SDK session id), `ContinueInProcess` (subsequent queries in the same live sidecar).
- **Permission modes** are validated against a four-element allowlist (`PERMISSION_MODES`) at session start and mid-session `set_mode` calls; unknown values raise `Error::InvalidMode` rather than silently failing.
- The `SessionManager` decides the engagement mode once (by querying the app DB for the SDK session id) before spawning the sidecar — no hidden resume→fresh fallback.
- First `query` command receives `cwd`, `model`, `permissionMode`, and the DB-decided engagement mode; all subsequent queries in the same sidecar get `mode: "continue"` stamped by `augment_query_if_needed`.
- Mid-session `set_mode` changes the permission mode by forwarding a `set_mode` command to the sidecar; the change applies immediately without restarting the session.
- Wire protocol uses string literals (`"fresh"`, `"resume"`, `"continue"`) exposed via `SessionMode::wire()` to ensure 1:1 sidecar parity.

## Key files

- **crates/forge-session/src/mode.rs** — defines `SessionMode` enum, `PERMISSION_MODES` allowlist, and validation logic.
- **crates/forge-session/src/manager.rs** — validates permission mode at `start_session` and `set_mode`, carries mode through session lifecycle.
- **crates/forge-session/src/protocol.rs** — stamps `mode`/`resumeSessionId` onto sidecar `query` commands via `augment_query_if_needed`.
- **crates/tauri-app/src/commands/sessions.rs** — resolves the DB-authoritative `SessionMode` from `resume_session_id` before spawning the session.

## Invariants & gotchas

- **Permission modes are never carried across resume**: a resumed session always starts with `permission_mode: None` in Rust (CodeForge parity); the sidecar's recorded session state governs permissions from that point.
- **No silent fallback**: an unrecorded `resume_session_id` is a named error, never a silent downgrade to fresh; the sidecar emits `session_resume_failed` if the SDK resume fails.
- **Init params consumed exactly once**: `SidecarInitParams` is held in a `Mutex<Option<_>>` and taken by the first `query`; subsequent queries get `mode: "continue"` without re-injecting cwd/model/permissionMode.
- **Validation before lookup**: `set_mode` validates the mode string against `PERMISSION_MODES` before retrieving the session, so a typo fails fast with a clear error rather than silently no-op.
- **Wire values are literals**: `SessionMode::wire()` returns `&'static str` to prevent divergence between Rust and the sidecar's expected `mode` field values.
