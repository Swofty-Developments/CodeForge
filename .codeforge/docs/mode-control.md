The edited files don't touch Mode Control — they're all about staleness detection and the index-status poller. The feature doc is already accurate; no changes needed.

---
# Mode Control

## Purpose

A compact dropdown UI in the composer that switches a running Claude Code session's permission mode (default/acceptEdits/plan/bypassPermissions). The "Auto" mode (bypassPermissions) shows a warning affordance since it auto-runs every tool without prompting.

## How it works

- Renders a pill-shaped button showing the current mode label (Ask, Accept edits, Plan, Auto) with a shield or warning icon.
- Clicking the button opens a dropdown menu listing all four modes with descriptions ("Approve each tool", "Auto-accept file edits", "Plan only — no changes", "Run everything, no prompts").
- When the user picks a new mode, calls `onSelect` (wired to `SessionPane.selectMode`) which (1) updates local `pendingMode` state for the next session and (2) if a session is running, calls `appStore.setSessionMode` → `ipc.setSessionMode` → Tauri `set_session_mode` command → `SessionManager::set_mode`.
- The backend applies the mode on the next query turn; the frontend optimistically updates `SessionUi.permissionMode` and rolls back on error.
- `SessionHeader` displays the current mode badge read-only; the interactive control lives in the `Composer`.

## Key files

- `crates/tauri-app/frontend/src/components/session/ModeControl.tsx` — the dropdown component (MODES array, button + menu, amber styling for `bypassPermissions`)
- `crates/tauri-app/frontend/src/components/session/Composer.tsx` — embeds `ModeControl` in the composer meta row; receives `currentMode` and `onSelectMode` props from `SessionPane`
- `crates/tauri-app/frontend/src/components/SessionPane.tsx` — wires `selectMode` to update both `pendingMode` (for next session) and live session via `appStore.setSessionMode`
- `crates/tauri-app/frontend/src/stores/session-slice.ts` — `setSessionMode` function: optimistic update of `sessions[i].permissionMode`, calls IPC, rolls back on error
- `crates/tauri-app/frontend/src/ipc.ts` — `setSessionMode(sessionId, mode)` Tauri invoke wrapper
- `crates/tauri-app/src/commands/sessions.rs` — `set_session_mode` Tauri command validates and delegates to `SessionManager::set_mode`
- `crates/tauri-app/frontend/src/components/session/SessionHeader.tsx` — imports `modeLabel` helper to display current mode (read-only badge)

## Invariants & gotchas

- The four modes are authoritative in `ModeControl.MODES` and must stay in sync with Rust `PermissionMode` enum (contract W5).
- `bypassPermissions` ("Auto") must always render with amber warning affordance — users need a visual cue that it runs without prompts.
- Mode changes are optimistic on the frontend but validated backend; unknown modes error rather than silently no-op.
- `pendingMode` (local state in `SessionPane`) persists for the *next* session when no session is active; switching modes mid-session updates the running session live.
- The backend applies mode on the *next* query turn, not immediately — there's no per-message mode snapshot.
- `SessionHeader` is read-only context (shows current mode); the interactive dropdown lives in `Composer.tsx`, not the header.
