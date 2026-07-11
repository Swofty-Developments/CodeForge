The changes this turn didn't touch the Session Pane feature at all — they modified the staleness detection system (backend poller + frontend modal suppression logic). The edited files were backend Rust (staleness.rs, repo_open.rs, state.rs) and frontend store logic (app-store.ts, ipc.ts) for handling `index:status` events, not the SessionPane component or its key files.

The existing doc is still accurate. No update needed.

---
# Session Pane

## Purpose

Right-side collapsible pane hosting embedded Claude Code sessions for the active repository context. Sessions are displayed in tabs, with each showing a live message stream, session header (model/mode/run-state), and composer for sending prompts.

## How it works

- **Context-filtered tabs**: Only sessions tagged with the active context path are shown; switching repos/worktrees filters the tab list (W1: context-tagged sessions).
- **Multi-session UI**: Tab strip with status dots (ready/generating/starting/error), clickable tabs to switch active session, close buttons, and a model-picker dropdown for starting new sessions.
- **Session header**: Slim read-only bar showing active session's model, permission mode, and run-state (generating/starting/error/ready) with a pulsing dot.
- **Message stream**: Scroll-anchored list rendering `session.messages` — user bubbles, system pills, assistant content blocks (text/thinking/tool cards), typing indicator, approval cards, and usage footer.
- **Composer**: Auto-resizing textarea (max ~8 lines) with slash-command popover, file-attachment picker (drag-drop + paperclip), permission-mode control, and send/stop buttons. Starts a session on first send if none active.
- **Pane width**: Controlled by `store.sessionPaneWidth`; toggle collapse/expand via `appStore.toggleSessionPane()`.

## Key files

- **SessionPane.tsx** — pane shell, tab strip, new-session dropdown, layout orchestration; wires SessionHeader + MessageStream + Composer.
- **SessionHeader.tsx** — slim read-only bar showing model/mode/run-state for the active session.
- **styles-chrome.ts** — CSS for pane shell, tabs, model dropdown, session header, body, and empty state.
- **MessageStream.tsx** — scroll-anchored renderer for `session.messages`; renders user/assistant/system messages, typing indicator, approval cards.
- **Composer.tsx** — autosize textarea, slash-command popover, file-attachment chips, mode control, send/stop actions.
- **local.ts** — thin helpers over `appStore` actions (`startSessionWithModel`, `selectSession`, `closeSession`) with no parallel state.

## Invariants & gotchas

- **No parallel state**: All session data (`messages`, `runState`, `permissionMode`, `info`) lives in `appStore.sessions`; local.ts and components read/dispatch but never cache.
- **Context-path filtering**: `visibleSessions()` filters `store.sessions` by `samePath(s.contextPath, store.activeContextPath)` — switching contexts hides unrelated sessions, not deletes.
- **Pending mode**: Before a session starts, `pendingMode` holds the mode for the next session; once active, the live `session.permissionMode` overrides it.
- **First-send auto-start**: `Composer.onSend` checks `!active()` and calls `appStore.startSession(undefined, pendingMode())` before sending — the user need not explicitly create a session.
- **Active tab border**: `.sp-tab--active` uses `border-bottom: 1px solid var(--bg-surface)` to bleed into the pane body, hiding the visual seam.
- **Status dots**: `dotClass()` maps `runState` → CSS class; "ready" = green, "generating" = blue, "starting" = amber, "error" = red.
