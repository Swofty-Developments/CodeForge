The changes are entirely in `app-store.ts` and deal with staleness detection and modal triggering logic for index status. The terminal slice itself was not touched at all.

---
# Terminal Slice

## Purpose

Manages state for the bottom terminal panel: a tab list of PTY instances, the active terminal ID, panel visibility, and height. Each terminal is worktree-scoped — its `cwd` is locked to the active context path at creation time, so switching contexts changes where **new** terminals spawn without moving existing ones.

## How it works

- `openTerminal()` spawns a PTY via IPC (`ipc.openTerminal(cwd)`), adds a tab to `terminals[]`, sets it active, and shows the panel.
- `closeTerminal(id)` kills the PTY backend-side and removes the tab; if closing the active tab, re-points to the nearest neighbour.
- `markTerminalExited(id)` is called on `terminal:exit` event — keeps the tab visible (output readable) but flips `exited: true`.
- `toggleTerminalPanel()` shows/hides the panel; if opening an empty panel and a repo is loaded, auto-spawns the first terminal.
- Panel height is clamped to `[120px, 80% of window height]` and persists in `terminalPanelHeight`.
- Each tab's title is the basename of its `cwd` (last path segment).

## Key files

- **`terminal-slice.ts`** — state shape (`TerminalTab[]`, active ID, panel visibility/height) and actions (open, close, mark exited, toggle panel).

## Invariants & gotchas

- **Worktree-scoped at creation**: a terminal's `cwd` is read from `store.activeContextPath` when `openTerminal()` runs and never updated — switching contexts doesn't move existing terminals.
- **Tab survives exit**: when a PTY process dies (`terminal:exit`), the tab stays in the strip with `exited: true` so the user can read the final output; explicitly calling `closeTerminal` removes it.
- **Active-tab repair**: removing the active terminal repoints `activeTerminalId` to the neighbour at `min(idx, remaining.length - 1)`, or `null` if the strip is empty.
- **120px floor**: panel height cannot go below `MIN_PANEL_H = 120` to keep the terminal usable.
- **Auto-spawn on toggle**: opening the panel when no terminals exist and a repo is loaded (`activeContextPath` is set) spawns one immediately — otherwise the user sees an empty panel.
- **xterm wiring lives elsewhere**: the actual xterm.js instances and `terminal:data`/`terminal:exit` listeners are in `components/terminal/**`; this slice is the tab bookkeeping only.
