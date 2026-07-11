# Terminal Slice

## Purpose

Manages the bottom terminal panel's PTY tab bookkeeping: opening, closing, switching, resizing, and tracking exit status. Each terminal is worktree-scoped — its `cwd` is bound at open time to `store.activeContextPath`, so new terminals follow the active worktree while existing ones keep their original path.

## How it works

- **Opening a terminal** calls `ipc.openTerminal(cwd)` to spawn a PTY in the backend, then adds a `TerminalTab` record (`{id, title, cwd, exited: false}`) to `store.terminals` and flips `terminalPanelOpen` to true.
- **Tab title** is the last path segment of `cwd` (e.g. `/repo/worktrees/feature-branch` → `feature-branch`).
- **Closing** kills the PTY via `ipc.closeTerminal(id)`, then removes the tab and re-points `activeTerminalId` to a surviving neighbour (or `null` if the list empties).
- **Exit events** mark `exited: true` but keep the tab around so the user can still read output; the panel view shows a visual "exited" indicator.
- **Panel height** is clamped to `[120px, 80% of viewport height]` and persists in `terminalPanelHeight`.
- **Toggle (Cmd+J)** flips `terminalPanelOpen`; if opening an empty panel, spawns the first terminal automatically (when `activeContextPath` is non-null).

## Key files

- **`stores/terminal-slice.ts`** — tab bookkeeping actions: open, close, toggle, resize, mark-exited. Does not own xterm wiring or event listeners.
- **`stores/app-store.ts`** — global state shape: `terminals[]`, `activeTerminalId`, `terminalPanelOpen`, `terminalPanelHeight`.
- **`types.ts`** — `TerminalTab` interface (`id`, `title`, `cwd`, `exited`).
- **`ipc.ts`** — `openTerminal(cwd, shell?)` and `closeTerminal(id)` invoke Tauri commands that manage the PTY lifecycle in Rust.

## Invariants & gotchas

- **Worktree-scoped `cwd`** — bound at open time to `activeContextPath`, not refreshed when the context changes. Switching worktrees changes where *new* terminals land, but existing ones stay rooted where they started.
- **Exited tabs stay in the list** — `closeTerminal` is the only way to remove a tab; `terminal:exit` events just flip `exited: true` so output remains readable. The panel view must filter or style exited tabs accordingly.
- **Auto-spawn on toggle** — toggling the panel open when `terminals.length === 0` spawns a terminal if a context is active; callers toggling the panel should expect this side-effect.
- **Active tab re-pointing** — removing the active terminal picks the neighbour at `min(idx, remaining.length - 1)`, which biases toward the tab to the right if removing a middle tab.
- **Panel height is clamped** — `setTerminalPanelHeight` clamps to `[MIN_PANEL_H, 80% of innerHeight]` every time, so the UI resize handle must enforce the same bounds client-side to avoid jitter.
