Now I have enough context to write the design doc.

# Mode Control

## Purpose
Dropdown UI near the composer that lets users switch between four permission modes: Ask (default), Accept edits, Plan, and Auto. The selected mode controls how the session handles tool execution and applies to the next turn.

## How it works
- Four modes: `default` (approve each tool), `acceptEdits` (auto-accept file edits), `plan` (plan only, no changes), `bypassPermissions` (run everything, no prompts).
- Renders a pill-shaped button with an icon (shield or warning triangle for Auto mode) and the current mode label.
- Clicking opens a dropdown menu with all four modes; selecting one calls `onSelect` with the new mode.
- The parent (`SessionPane`) calls `appStore.setSessionMode(sessionId, mode)` to persist the change in the backend DB and apply it to the sidecar on the next turn.
- Auto mode displays an amber warning style (border, background, icon) since it bypasses all permission checks.
- Optimistic UI: the mode updates immediately in the frontend store, with rollback if the backend call fails.

## Key files
- `crates/tauri-app/frontend/src/components/session/ModeControl.tsx` — dropdown component, mode definitions, and styling.
- `crates/tauri-app/frontend/src/stores/session-slice.ts` — `setSessionMode` action that calls the backend IPC and handles rollback.
- `crates/tauri-app/frontend/src/ipc.ts` — `setSessionMode` wrapper for the `set_session_mode` Tauri command.
- `crates/tauri-app/src/commands/sessions.rs` — `set_session_mode` command that validates the mode string and persists it to the session DB.

## Invariants & gotchas
- Mode changes apply to the **next turn**, not mid-turn — the sidecar reads the mode at turn start.
- The `PermissionMode` type is a union of four string literals; unknown values must error explicitly, not silently no-op.
- Auto mode (`bypassPermissions`) must always display a warning affordance (amber styling, triangle icon) to signal the security implication.
- The dropdown is disabled when no session is active; `pendingMode` in `SessionPane` holds the mode for the next session start.
- Optimistic updates must roll back on IPC failure — never leave the UI showing a mode that the backend rejected.
