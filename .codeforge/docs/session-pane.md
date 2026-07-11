# Session Pane

## Purpose

Right-hand collapsible panel embedding live Claude Code sessions. Users view streamed messages, send prompts, switch permission modes, and manage multiple concurrent sessions per worktree context — all within the same window as the repo timeline and feature tree.

## How it works

- **Tab strip** — one tab per live session in the active worktree context; displays status dot (ready/generating/starting/error), double-click to rename, close button to stop session
- **Model picker** — "New session" dropdown offering presets (Default/Opus/Sonnet/Haiku/Fable) plus custom model input; creates a fresh session scoped to the active repo context
- **Session header** — slim read-only bar showing active session's model, live permission mode, and current run state (generating/starting/error/ready); animated via `fade-slide-down` keyframe
- **Message stream** (`MessageStream.tsx`) — scroll-anchored list rendering `session.messages` (the single source of truth): user bubbles, assistant text/thinking/tool blocks, approval cards, typing indicator, and usage footer
- **Composer** (`Composer.tsx`) — autosize textarea, slash-command popover (opened by `/token`, arrow-nav + Enter to pick), file-attachment chips, mode switcher (`ModeControl`), and send/stop buttons
- **Permission modes** — four SDK modes (Ask/Accept edits/Plan/Auto) settable for pending or active sessions; "Auto" (bypassPermissions) shows an amber warning icon since it runs every tool without prompts
- **Past sessions** — when no active session, empty state lists resumable sessions fetched via `listPastSessions(repo.path)` and filtered against live session IDs

## Key files

- **`SessionPane.tsx`** — container orchestrating tabs, new-session menu, fullscreen toggle, and collapse; filters sessions to the active context
- **`session/MessageStream.tsx`** — scroll-anchored message renderer with 150px away-threshold, 80ms debounce for auto-follow
- **`session/Composer.tsx`** — textarea + slash menu + attachments + mode control; hands the finished prompt to `onSend`
- **`session/SessionHeader.tsx`** — read-only strip showing model, permission mode label, and live run state with pulse animation
- **`session/ModeControl.tsx`** — permission-mode dropdown; "Auto" (bypassPermissions) styled amber with warning triangle
- **`session/blocks.tsx`** — `ContentBlock` renderers: text (Markdown), thinking (collapsible gray card), tool cards (name + input/output), approval card, typing indicator
- **`session/styles-chrome.ts`** — chrome CSS covering pane shell, tab strip, new-session dropdown, session header, and empty states; injected once by `styles.ts`
- **`session/local.ts`** — thin wrappers over `appStore` actions (`startSessionWithModel`, `selectSession`, `closeSession`); no parallel state

## Invariants & gotchas

- **Single source of truth**: `session.messages` is authoritative; never duplicate message state — the stream reads it directly
- **Context filtering**: only sessions tagged with `store.activeContextPath` appear (worktree-scoped tabs); `samePath` comparison prevents duplicate tabs when the same logical path differs by trailing slash
- **Resume filtering**: `listPastSessions` result is filtered `!liveIds.has(s.id)` so resumed sessions don't appear twice (once as a tab, once in the past-sessions list)
- **Slash-menu lifecycle**: opens only when `input` matches `^\/(\S*)$` (a single `/token` with no space); typing a space finalizes the command and closes the menu; `slashDismissed` prevents reopening until the input changes
- **Scroll anchoring**: `scrolledAway` threshold is 150px from bottom, 80ms debounce to avoid layout thrash; switching sessions (`on(() => props.session.info.id)`) resets `scrolledAway` to false and snaps to bottom
- **Mode switch scope**: `pendingMode` governs the *next* session started via the new-session menu; an active session's mode is `session.permissionMode` and switches via `appStore.setSessionMode(sessionId, mode)`
- **Tab editing**: double-click enters rename mode (inline input); Enter commits, Escape cancels, blur commits; `ref={(el) => setTimeout(() => el.select(), 0)}` auto-selects the title text after the input mounts
- **Stop vs. close**: clicking the session header's "stop" button (`onStop`) aborts the in-flight turn via `interruptSession`; clicking the tab's close `×` (`closeSession`) stops the entire session and removes the tab
- **Layout state** — `sessionPaneOpen`, `sessionPaneWidth`, and `sessionPaneFullscreen` live in `appStore` and persist across app restarts; the pane toggle is bound to `⌘\` (kbd-hint in Welcome view); fullscreen z-index is 1000, overlaying the main layout
- **Derived repo** — `store.repo` is a getter that resolves to the active context's `RepoState`, so the session pane reads repo state reactively without hard-coupling to the context model
