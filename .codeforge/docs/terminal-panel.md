None of the edited files this turn touch the Terminal Panel feature. The changes are all about index staleness detection (polling, `index:status` events) — a completely different feature. The existing living doc is accurate and up-to-date for Terminal Panel.

---
# Terminal Panel

## Purpose
Bottom resizable panel (FZ-3) that hosts worktree-scoped xterm.js PTY tabs. Each terminal is bound to the active context path at open time; switching contexts affects only where *new* terminals open.

## How it works
- `TerminalPanel` owns the tab strip, drag-to-resize handle, and the global `termMap` registry (`id → Terminal`) for routing `terminal:data` / `terminal:exit` events from Tauri.
- `TerminalInstance` creates one `Terminal` with `FitAddon`, registers itself, and listens for `onData` → `write_terminal` IPC. A `ResizeObserver` fits and calls `resize_terminal` whenever the container changes.
- All instances stack absolutely (`position: absolute; inset: 0`) and swap visibility via `term-host--active` so the active one stays correctly sized when hidden tabs return to foreground.
- Base64 output from Tauri is decoded to `Uint8Array` (`b64ToBytes`) before `term.write()` so UTF-8 renders correctly; `terminal:exit` writes a styled gray "process exited" message.
- Tab bar mirrors the worktree-tab chrome: green dot (gray when exited), close button (visible on hover/active), "+" to spawn a new terminal in the current context, collapse chevron to toggle `terminalPanelOpen`.
- Backend (`commands/terminals.rs`) routes to `TerminalManager` in `AppState` which spawns the user's login shell (bash/zsh) and emits events over the Tauri event bus.

## Key files
- `crates/tauri-app/frontend/src/components/terminal/TerminalPanel.tsx` — panel container, tab strip, drag-resize, event listeners, `termMap` registry.
- `crates/tauri-app/frontend/src/components/terminal/TerminalInstance.tsx` — one xterm.js `Terminal` + `FitAddon`, `ResizeObserver`, `onData` → IPC.
- `crates/tauri-app/frontend/src/stores/terminal-slice.ts` — state slice for `terminals[]`, `activeTerminalId`, `terminalPanelHeight`, `openTerminal()` / `closeTerminal()`.
- `crates/tauri-app/src/commands/terminals.rs` — Tauri IPC commands: `open_terminal`, `write_terminal`, `resize_terminal`, `close_terminal`, `list_terminals`.
- `crates/tauri-app/src/terminal/pty.rs` — PTY spawn/resize/write logic (assumed; not shown but implied by commands).

## Invariants & gotchas
- **Base64 decoding is mandatory** — Tauri emits `terminal:data` as base64 because Tauri events don't support raw binary. Skipping `b64ToBytes` breaks UTF-8.
- **Inactive instances must stay sized (`inset: 0`)** — if hidden tabs collapse to 0×0, the `FitAddon` returns wrong dimensions when they re-activate. All instances are sized but only the active one is `visibility: visible`.
- **`fit()` after visibility toggle** — `createEffect(() => { if (props.active) fitAndResize(); term?.focus(); })` refits when a tab becomes active because xterm dimensions are stale if measured while hidden.
- **`cwd` binds at open-time** — the terminal's working directory is captured from `store.activeContextPath` when the tab is created. Switching contexts later does not move existing terminals.
- **ResizeObserver must debounce via the host div** — observing the xterm element directly can trigger resize loops. Watching the outer `term-host` container avoids this.
- **Collapsed panel retains state** — toggling `terminalPanelOpen` hides the panel but does not kill PTYs. Tabs remain live and keep accumulating scrollback.
