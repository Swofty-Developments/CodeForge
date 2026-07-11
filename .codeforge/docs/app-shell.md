---
# App Shell

## Purpose

Top-level UI harness that assembles the entire desktop app layout: title bar, worktree tabs, sidebar (feature tree), main panel (tab bar + view switcher), session pane, terminal panel, status bar, modals, command palette, and toast stack. Manages global keyboard shortcuts and resize-handle dragging for sidebar and session pane widths. Shows the Welcome view as a fallback when no repo is open.

## How it works

- **Layout structure**: title bar → worktree tabs → three-column workspace (sidebar | main panel | session pane) → status bar, with modals and toasts stacked on top via z-index. When `store.repo` is null, the workspace shows `<Welcome />` instead of the three-column layout.
- **Resize-handle dragging**: `onSidebarDragStart` / `onSessionDragStart` set flags + cursor, `onMouseMove` updates store widths, `onMouseUp` clears flags; session pane width is calculated as `window.innerWidth - e.clientX` since it hugs the right edge.
- **Global shortcuts**: Cmd+K (toggle palette), Cmd+\ (toggle session pane), Cmd+J (toggle terminal), Cmd+1-4 (switch views), Esc (priority close: palette → deny pending approval → session pane).
- **View switcher**: keyed `<Show when={store.activeContextPath}>` triggers cross-fade + slide animation when active repo context changes; `<Switch>` renders FeatureDetail / GraphView / TimelineView / DiffReview based on `store.activeView`.
- **Conditional rendering**: sidebar, worktree tabs, main panel, and terminal only mount when `store.repo` is set; otherwise shows `<Welcome />`, which checks Claude CLI installation/authentication on mount and displays a warning banner if missing or unauthenticated.
- **Session pane fullscreen mode**: `toggleSessionPaneFullscreen()` toggles `sessionPaneFullscreen` flag; when enabled, the pane is fixed-position covering the entire app below the title bar (z-index 1000), ignoring its configured width.
- **Toast stack**: bottom-right fixed stack rendering `store.toasts` with semantic red/green tint, click-to-dismiss.

## Key files

- **crates/tauri-app/frontend/src/App.tsx** — shell component assembling all UI panels, resize logic, keyboard shortcuts, view switcher, toast stack.
- **crates/tauri-app/frontend/src/main.tsx** — entry point loading fonts, calling `appStore.initListeners()` once, mounting `<App />` into `#app`.
- **crates/tauri-app/frontend/src/views/Welcome.tsx** — pre-repo landing screen with "Open repository" CTA, keyboard hints, and Claude CLI status check.
- **crates/tauri-app/frontend/src/stores/app-store.ts** — global store; exports `toggleSessionPane()` and `toggleSessionPaneFullscreen()` for keyboard shortcuts and UI buttons.
- **crates/tauri-app/frontend/src/components/SessionPane.tsx** — right pane rendering session tabs, message stream, composer, fullscreen + collapse buttons.
- **crates/tauri-app/frontend/src/components/session/styles-chrome.ts** — session pane CSS module; `.session-pane--fullscreen` rule applies fixed positioning and width override.
- **crates/tauri-app/frontend/src/ipc.ts** — typed wrapper for all Tauri IPC commands and event listeners, including `checkClaudeCli()`.
- **crates/tauri-app/src/main.rs** — Tauri entry point; initializes AppState (db, repos, sessions, terminals), registers all IPC command handlers via `invoke_handler`, loads plugins.
- **crates/tauri-app/src/commands/mod.rs** — IPC command module index; exports `repo`, `features`, `sessions`, `worktrees`, `terminals`, `timeline` submodules; every command returns `Result<T, String>`.
- **crates/tauri-app/src/commands/repo.rs** — backend IPC commands for repo operations; includes `check_claude_cli` which probes PATH and tests authentication.
- **crates/forge-index/src/headless.rs** — headless `claude -p --output-format json` invocation; retries up to 3 times on transient failures (timeout, network, rate-limit) with exponential backoff.

## Invariants & gotchas

- **One global event listener registration**: `main.tsx` calls `appStore.initListeners()` once at boot; do not duplicate in `App` or child components — events demux internally via the store.
- **Session pane width is right-anchored**: when dragging the session resize handle, width = `window.innerWidth - e.clientX`, not `e.clientX` directly; breaking this inverts the drag direction.
- **Fullscreen mode overrides width**: when `sessionPaneFullscreen` is true, the pane ignores `sessionPaneWidth` and spans 100% of the window (fixed positioning). The width slider is hidden in this state; toggling fullscreen off restores the previous width.
- **Esc key priority order**: palette → pending approval denial → session pane close; later handlers must early-return if an earlier case fires, or Esc will close multiple layers at once.
- **View switcher must be keyed on `activeContextPath`**: without the `keyed` attribute, SolidJS won't trigger the cross-fade animation when switching repos/worktrees — the view content updates but the transition is lost.
- **Resize handles must call `preventDefault()`**: otherwise text selection triggers during drag and breaks the cursor-tracking loop.
- **All shortcuts require repo**: Cmd+J and Cmd+1-4 check `store.repo` before firing; adding new repo-dependent shortcuts must gate the same way or they'll throw when no repo is open.
- **Claude CLI check is frontend-initiated**: Welcome calls `checkClaudeCli()` on mount (not pushed from the backend); the check is non-blocking so the UI stays responsive even if the probe times out.
- **Headless indexing retries transient failures**: `run_headless_claude` in `forge-index/src/headless.rs` retries up to 3 times on timeouts, network errors, rate-limits, 503/504 responses; non-transient errors (auth, bad prompt, ENOENT) fail immediately. Each retry uses exponential backoff (1s, 2s, 4s).
