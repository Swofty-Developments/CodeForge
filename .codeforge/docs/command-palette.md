The changes this turn were to backend Rust code (Tauri commands, indexing retry logic) and don't affect the command palette frontend at all. The doc is still accurate.

---
# Command Palette

## Purpose

A `Cmd+K` fuzzy-search modal that exposes all high-level actions (open repo, reindex, start session, toggle terminal), view switches, and feature navigation. Acts as the keyboard-first entry point for every core workflow.

## How it works

- **Greedy subsequence fuzzy match** — `fuzzyMatch(query, label)` walks the label left-to-right collecting matching character indices; empty query matches everything.
- **Merged command registry** — combines static `baseCmds` (actions + view switches) with dynamically generated feature commands from `store.features`.
- **Keyboard navigation** — arrow keys cycle through filtered results (modulo wrapping), `Enter` executes the selected command, `Escape` closes the palette (handled by App.tsx global listener).
- **Auto-scroll** — a `createEffect` watches `selected()` and calls `scrollIntoView({ block: "nearest" })` on the highlighted row so it stays visible during keyboard navigation.
- **Overlay dismissal** — clicking the backdrop closes the palette; the inner `.cmd-palette` div stops propagation so clicks on results stay captured.
- **Three visual categories** — commands are tagged `action`/`view`/`feature` with color-coded badges (green/amber/primary) to help users scan context.
- **Session pane toggle** — the "Toggle session pane" command calls `appStore.toggleSessionPane()` (app-store.ts:253), which flips `sessionPaneOpen` state. The pane itself reads `store.sessionPaneOpen` to conditionally mount (SessionPane.tsx:119 defaults to `true`).

## Key files

- **CommandPalette.tsx** — the modal component: fuzzy filtering, keyboard nav, matched-char highlighting, and the 520px × 420px max centered modal with blur backdrop.
- **app-store.ts** — `paletteOpen` boolean state and `setPaletteOpen(bool)` action (line 237); `toggleSessionPane()` action (line 253) that flips `sessionPaneOpen`.
- **App.tsx** — global `Cmd+K` listener that toggles `paletteOpen`; global `Escape` handler that closes the palette if open (priority 1 before session approval or session pane).

## Invariants & gotchas

- **No global shortcuts registered by the palette itself** — `App.tsx` owns the `Cmd+K` and `Escape` listeners; the palette only handles arrow/enter navigation.
- **Escape priority cascade** — `App.tsx` closes palette → denies pending approval → closes session pane in that order; the palette must not try to handle `Escape` itself.
- **Auto-scroll depends on `.selected` class** — the effect queries `.cmd-palette-item.selected`; renaming or removing that class breaks keyboard-scroll sync.
- **Feature commands are ephemeral** — rebuilt from `store.features` on every query change; the palette never holds stale feature references.
- **Empty query matches all** — `fuzzyMatch("", text)` returns `[]` (not `null`), so the unfiltered list is the full registry.
- **Session pane toggle is a boolean flip** — `toggleSessionPane()` does not check repo state; the composer itself disables when `!store.repo`. The session pane can be open with no repo (shows empty state).
