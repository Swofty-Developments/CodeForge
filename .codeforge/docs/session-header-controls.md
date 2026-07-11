# Session Header Controls

## Purpose

Displays read-only status for the active Claude Code session in a slim horizontal bar: model identifier, current permission mode, and live run-state indicator ("generating…", "starting…", "error", "ready").

## How it works

- Receives a `SessionUi` object with `info.model`, `permissionMode`, and `runState` fields
- Renders a fixed-height (26px) bar with a terminal icon, model name (monospace, max 130px), dot separator, mode label (from `ModeControl.modeLabel`), flexible spacer, and run-state pill
- Run-state pill is animated with a pulsing dot for transient states (`generating`, `starting`), static text for terminal states (`ready`, `error`)
- Color-codes each state: sky for generating, amber for starting, red for error, muted tertiary for ready
- Non-interactive — mode switching happens in the `Composer`, not here
- Sits below the tab strip and above the message stream in the session pane layout

## Key files

- `SessionHeader.tsx` — the header component itself; renders model/mode/run-state in one flex row
- `SlashMenu.tsx` — filterable slash-command popover shown from the composer (separate from header controls but lives in the same `session/` directory)
- `styles-chrome.ts` — `.sp-header` layout, `.sp-h-state` pulsing dot animations, color variants, and fade-slide-down entrance

## Invariants & gotchas

- **Read-only context only** — the header displays session state but does not mutate it; interactive controls (mode switcher, stop button, rename) live in `SessionPane` and `Composer`, not here
- **Run-state is ephemeral** — `runState` changes mid-turn; the component must react to every update without debouncing or caching
- **Model can be null** — falls back to "default" string when `session.info.model` is undefined
- **Pulsing animation requires CSS keyframes** — `@keyframes dot-pulse` must exist in `global.css`; the component only references it via class
- **Permission mode label** — must call `modeLabel(...)` from `ModeControl.tsx` to stay consistent with the mode switcher's display text
