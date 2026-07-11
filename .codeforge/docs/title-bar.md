The changes this turn were all backend (staleness polling) — the Title Bar component itself wasn't modified. The frontend changes were in `app-store.ts` (the `handleIndexStatus` logic) and `ipc.ts` (updated event-listener comments), but those don't affect the Title Bar's behavior or structure. The Title Bar reads `store.repo` and renders pills; none of its sources changed. The doc is already accurate.

---
# Title Bar

## Purpose

Custom macOS window chrome with draggable region, traffic-light inset, and repo identity pills (project name, branch name, feature count). Replaces the native window title bar with an IDE-styled control surface.

## How it works

- Entire bar is a Tauri drag region (`data-tauri-drag-region`) for window movement; interactive elements opt out via `-webkit-app-region: no-drag`.
- **72px left inset** reserves space for macOS traffic lights (minimize/maximize/close), which are overlaid by the OS.
- **Pill buttons** show project name (opens command palette), current branch (opens Worktrees menu), and feature count — all read from `appStore.repo`.
- **Branch menu dropdown** offers "Worktrees…", "New worktree from `<branch>`…", and (when not on base worktree) Claude-assisted commit/push/PR actions that prefill the composer.
- Base branch is computed from the first context marked `isBase` in the contexts array.
- Falls back to "CodeForge" app name when no repo is loaded.

## Key files

- **`TitleBar.tsx`** — Entire component; self-contained with inline styles.
- **`app-store.ts`** — Source of `repo`, `contexts`, `activeContextPath`; exposes `setPaletteOpen`, `setWorktreeSwitcherOpen`, `prefillComposer`.
- **`global.css`** — Defines `--titlebar-height: 34px` for layout consistency.

## Invariants & gotchas

- **Traffic-light inset is macOS-only**: the 72px left spacer assumes window controls are in the top-left; non-macOS builds would need conditional layout or a different window decoration strategy.
- **Drag region is app-region-aware**: any interactive child must explicitly set `-webkit-app-region: no-drag` or clicks will be consumed by the window drag handler.
- **Branch menu backdrop**: uses a fixed-position backdrop (`z-index: 99`) to close the menu on outside clicks; the menu itself is `z-index: 100` to float above.
- **Claude hand-off actions** (`commitPushWithClaude`, `openPrWithClaude`) only appear when `!activeIsBase()` — they assume you're on a non-base worktree where commits and PRs make sense.
- **Active context detection**: relies on `store.activeContextPath` matching a context's `state.path` via `samePath` util; if contexts fall out of sync the wrong branch name may display.
