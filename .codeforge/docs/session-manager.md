The edited files touch frontend (SessionPane UI, app-store) and headless indexing (`forge-index/headless.rs`), none of which affect the session-manager doc. The session manager is backend Rust (`forge-session/src/manager.rs`), and none of the changes altered its API, lifecycle, or contracts.

The existing doc is already accurate. No update needed.

---
The existing doc is accurate. The only change was adding `rename_session` to the Tauri command handler list in `main.rs`, which is already covered by the "update_title" mention in the doc's manager description. No structural changes to the session manager itself.

---
# Purpose

Manages all live Claude Code sessions: spawns, stops, queues input, approves requests, switches permission modes, and maintains a registry of running sessions. Each session is a sidecar process; the manager bridges Tauri IPC commands to session lifecycle calls and forwards events to the frontend.

# How it works

- **Registry**: Tracks running sessions in a `HashMap<Uuid, SessionEntry>` with display metadata (title, model, resume hint).
- **Start**: Allocates a UUID, spawns a `ClaudeSession` (sidecar + protocol tasks), persists thread/session rows in the DB (CONTRACT-2), and spawns a forwarder that re-emits `AgentEvent`s on the `agent-event` Tauri channel.
- **Control**: Exposes `send` (queue prompt), `approve` (answer tool approval/question), `set_mode` (switch permission mode mid-session), `abort` (kill in-flight turn), and `stop` (kill sidecar + remove from registry).
- **Resume**: Session mode (Fresh/Resume/Continue) is DB-authoritative, resolved at start from `sessions.claude_session_id` — never inferred sidecar-side. A failed resume surfaces as an event, not a silent fresh start.
- **Persistence**: The event forwarder persists assistant text, usage, and the confirmed `claude_session_id` to the DB; failures are surfaced as `session_persistence_degraded` events, never swallowed warnings.
- **List**: Snapshots all sessions as `Vec<SessionInfo>`, distinguishing pre-/post-handshake via whether the confirmed SDK session ID is set.

# Key files

- **crates/forge-session/src/manager.rs** — `SessionManager` registry: start, send, approve, set_mode, abort, stop, list, update_title.
- **crates/forge-session/src/lib.rs** — Module root: types, errors, re-exports.
- **crates/forge-session/src/claude.rs** — `ClaudeSession`: one sidecar process + 4 protocol tasks (stdin writer, stdout parser, stderr collector, augmenter).
- **crates/forge-session/src/mode.rs** — `SessionMode` enum (Fresh/Resume/Continue) and permission-mode validation.
- **crates/tauri-app/src/commands/sessions.rs** — Tauri commands: `start_session`, `send_to_session`, `approve_session`, `interrupt_session`, `stop_session`, `set_session_mode`, `list_sessions`, `rename_session`.
- **crates/tauri-app/src/runtime/session_forward.rs** — Per-session forwarder: persists assistant text/usage/resume id to DB, emits events to frontend, surfaces persistence failures as named state.

# Invariants & gotchas

- **CONTRACT-1**: Every emitted event is stamped with the forge `sessionId` (manager UUID), never the SDK session ID — frontend demuxes by the manager ID.
- **CONTRACT-2**: Started sessions have a thread row by construction; if DB writes fail at start, the session is torn down before returning (never run headless).
- **Session mode**: DB-authoritative, resolved once at start from `sessions.claude_session_id`. A resume id that isn't recorded in the DB is a named error, not a silent fresh start.
- **Permission mode**: Validated before session lookup in `set_mode` — an invalid mode is `Error::InvalidMode`, never a silent no-op. The four valid modes: `default`, `acceptEdits`, `plan`, `bypassPermissions`.
- **Stop vs abort**: `stop()` kills the sidecar and removes the session from the registry (destructive). `abort()` kills only the in-flight turn; the session and its transcript stay alive.
- **Manager ownership**: The manager lives behind a `tokio::sync::Mutex` in `AppState`; all commands lock it briefly, never across an `.await`.
- **Persistence failures**: Never swallowed — emitted as `session_persistence_degraded` events so the UI can warn that stored history/usage may be incomplete.
