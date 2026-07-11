# Terminal Panel

## Purpose

Bottom-docked resizable panel hosting one or more xterm.js terminal instances, each bound to a spawned PTY. Tabs mirror the shell CWD at spawn time, routing terminals to worktree contexts. Opened with Cmd+J.

## How it works

- **TerminalPanel** mounts global `terminal:data` / `terminal:exit` listeners and routes output to the matching xterm instance via a `Map<id, Terminal>` maintained by `TerminalInstance` callbacks.
- Each **TerminalInstance** owns one xterm.js `Terminal` + `FitAddon`, stacks absolutely via `.term-host`, and is hidden when inactive. The active instance calls `fit()` + `resizeTerminal()` on mount, ResizeObserver ticks, and when it becomes visible.
- Tabs display `{cwd-basename}` + a green dot (gray when exited); the `+` button spawns a new PTY rooted at `activeContextPath`.
- The top edge is draggable (3px hit zone) to resize panel height; clamped to `[120px, 80vh]`.
- xterm writes user input to the PTY via `onData → writeTerminal(id, data)`. Base64-decoded output (`b64ToBytes`) preserves UTF-8 multibyte sequences.
- Exited PTYs display a gray "exited" tag and remain in the tab strip so their scrollback is readable; the instance is kept alive until the tab is manually closed.

## Key files

- **TerminalPanel.tsx** — panel chrome, tab strip, resize drag logic, event listeners for `terminal:data` / `terminal:exit`, `id → Terminal` registry
- **TerminalInstance.tsx** — one xterm.js instance + FitAddon per tab; theme, font, ResizeObserver auto-fit, onData → IPC write
- **terminal-slice.ts** — bookkeeping for PTY tabs (`openTerminal`, `closeTerminal`, `markTerminalExited`), panel open/height state, CWD-to-title mapping

## Invariants & gotchas

- **Instance stacking**: all instances render simultaneously (stacked `position: absolute; inset: 0`); only the active one is `visibility: visible`. This keeps their DOM alive so fit dimensions are correct when they return to the foreground.
- **Fit timing**: `fitAndResize()` MUST guard `host.clientWidth/Height === 0` or the addon throws. `createEffect(() => { if (active) queueMicrotask(fitAndResize) })` delays the call until the instance is visible.
- **Base64 → bytes**: PTY output arrives as base64; write `b64ToBytes(data)` not `atob(data)` or multibyte UTF-8 glyphs will corrupt.
- **CWD binding**: a terminal's `cwd` is frozen at spawn time to `store.activeContextPath`. Switching worktrees does NOT re-root existing terminals; only NEW terminals open in the new context.
- **Exit semantics**: `terminal:exit` calls `markTerminalExited(id)` and writes a gray `[process exited — code N]` line; the tab stays in the strip (output readable) until manually closed with the `×`.
- **Panel height clamp**: top-edge drag is clamped to `[120px, 80vh]` in `setTerminalPanelHeight` so the panel cannot consume the entire window or collapse below readable size.
