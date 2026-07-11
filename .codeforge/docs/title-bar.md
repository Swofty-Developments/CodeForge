# Title Bar

## Purpose

Custom draggable window chrome displaying project identity badges (repo name, branch, feature count) as interactive pills, with a dropdown menu for worktree/branch operations and Claude-assisted git workflows. Shows project badges only when a repo is open; falls back to "CodeForge" appname otherwise.

## How it works

- **72px left inset** reserves space for macOS traffic-light window controls; entire bar is draggable via `data-tauri-drag-region` except interactive elements (`-webkit-app-region: no-drag`).
- **Project pill** (folder icon + repo name) opens the command palette; **branch pill** (git icon + branch name + caret) toggles a dropdown menu for worktree switching and Claude-assisted commit/PR flows.
- **Feature count pill** displays `store.repo.featuresCount` as a read-only badge showing index coverage.
- **Multi-project context switching**: reads `store.contexts` (array of `RepoContext` with `state` and `isBase` flag) and highlights the active context via `store.activeContextPath`; branch menu shows "Commit & push with Claude" and "Open a PR with Claude" only when the active context is NOT the base worktree (`!activeIsBase()`).
- **Agent hand-offs** via `appStore.prefillComposer()` — branch menu actions inject task prompts (commit/push, open PR) that reference the current branch and base branch, then open the session pane.
- **Derived repo getter**: `store.repo` is computed from `store.contexts.find(c => samePath(c.state.path, store.activeContextPath))`, so title-bar pills always reflect the active worktree's state.

## Key files

- **`crates/tauri-app/frontend/src/components/TitleBar.tsx`** — entire title-bar UI: traffic-light inset, identity pills, branch menu, drag region.

## Invariants & gotchas

- The 72px traffic-light inset is macOS-specific and must remain fixed width; on other platforms the inset is unused space but safe to keep.
- `data-tauri-drag-region` on the root `.titlebar` enables window dragging; any interactive element (buttons, menus) must set `-webkit-app-region: no-drag` or clicks will move the window instead of activating the control.
- `store.repo` is a **derived getter**, never written directly — mutations go to `store.contexts[i].state`; the title bar reads the getter so it auto-updates when `activeContextPath` or context state changes.
- Branch menu actions call `setBranchMenu(false)` before opening other UI (switcher, composer) to close the dropdown, preventing overlapping modals.
- The "Commit & push" / "Open a PR" menu items appear only when `!activeIsBase()` (the active worktree is not the base checkout) — exposing them on the base worktree would confuse users since those flows assume feature-branch work.
