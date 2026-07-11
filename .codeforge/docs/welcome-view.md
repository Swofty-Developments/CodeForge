The Welcome View file is unchanged. The work this turn was entirely about the session/approval system (AskUserQuestion flow) and did not touch the Welcome View component, its dependencies, or its behavior. The existing doc already correctly states this at the top.

---
# Welcome View

**Purpose**  
Empty-state screen displayed when no repository is open. Provides branding, onboarding copy, a folder-picker CTA to open a repository, and keyboard-shortcut hints.

**How it works**  
- Renders centered welcome UI with FeatureForge branding (SVG logo + title + tagline).
- "Open repository" button triggers Tauri's native folder-picker dialog via `@tauri-apps/plugin-dialog`.
- On folder selection, calls `appStore.openRepo(path)` which invokes the backend `open_repo` IPC command.
- If `open_repo` rejects with a non-git folder, the InitRepoModal appears to offer `git init` instead of showing an inline error.
- Displays `store.lastError` inline if `open_repo` fails for other reasons (permission denied, daemon crash, etc.).
- Hints for `⌘K` (command palette) and `⌘\` (session pane) are always visible; they work even without a repo.

**Key files**  
- `crates/tauri-app/frontend/src/views/Welcome.tsx` — the complete view component; self-contained UI + folder-picker wiring.
- `crates/tauri-app/frontend/src/stores/context-slice.ts` — `openRepo` action that calls the backend and handles the `not_a_git_repo:` prefix to trigger InitRepoModal.
- `crates/tauri-app/frontend/src/App.tsx` — mounts `<Welcome />` when `!store.repo` (no active context).

**Invariants & gotchas**  
- `store.lastError` is the ONLY error channel shown inline; the non-git-folder case clears `lastError` and raises `initRepoPrompt` instead, so the InitRepoModal owns that flow.
- The welcome view is *never* shown if `store.repo` is truthy, even if zero features exist — the app shell switches to feature/timeline views instead.
- Keyboard hints (`⌘K`, `⌘\`) are static and do not reflect custom keybindings.
- The view has no recent-repos list yet (description mentions it, but the current implementation omits it).
- Component unchanged this turn — session approval system work (AskUserQuestion fixes) did not touch the Welcome View.
