None of the edited files relate to the init-repo-modal feature. The changes touched:
- `app-store.ts`: Added `toggleSessionPane` and `toggleSessionPaneFullscreen` functions (session pane UI controls)
- `SessionPane.tsx`: Session pane UI improvements (editing tabs, past sessions, fullscreen toggle)
- `forge-index/src/headless.rs`: Headless Claude CLI invocation logic (indexing)
- `session/styles-chrome.ts`: Session pane styling

The init-repo-modal feature remains unchanged. The existing doc is accurate.

---
# Init Repo Modal

## Purpose

Modal that offers to initialize a plain (non-git) folder as a git repository when the user tries to open it. Eliminates opening a non-git directory as a dead end by calling `git init -b main` and proceeding with the normal repo-open flow.

## How it works

- `openRepo` attempts to open a path; if the backend rejects with the machine-matchable prefix `"not_a_git_repo:"`, the error is suppressed and the path is stored in `initRepoPrompt` signal.
- `InitRepoModal` renders when `initRepoPrompt()` is non-null, showing the path and folder name with a two-button choice.
- Clicking **Initialize repository** calls `confirmInitRepo()`, which invokes the Tauri `init_repo` command (`git init -b main` on the backend).
- On success, the backend returns a `RepoState` via the same `repo_open::open_context` flow used for normal opens, and the modal is dismissed.
- The newly-initialized repo is adopted as a context (features/timeline/daemon/worktrees refresh) exactly like a normal repo open.
- Clicking **Cancel** dismisses the prompt without action.

## Key files

- **crates/tauri-app/frontend/src/components/InitRepoModal.tsx** — modal UI reading `initRepoPrompt` signal, calling `confirmInitRepo` or `dismissInitRepoPrompt`.
- **crates/tauri-app/frontend/src/stores/context-slice.ts** — `openRepo` catches `"not_a_git_repo:"` rejection and sets `initRepoPrompt`; `confirmInitRepo` invokes backend `init_repo` and adopts the result.
- **crates/tauri-app/src/commands/repo.rs** — `init_repo` command runs `git init -b main` via `forge_git::init_repo`, then calls `repo_open::open_context`.

## Invariants & gotchas

- **Machine-matchable prefix**: the backend's `"not_a_git_repo: {path}"` error format is the contract between `open_repo` and the modal trigger; changing it breaks detection.
- **Single-shot prompt**: `initRepoPrompt` is a slice-local signal (not part of `AppStore`), so it's wiped on context close or navigation away.
- **Already-initialized rejection**: calling `init_repo` on an already-initialized repo is a named error — the frontend should call `openRepo` instead.
- **Busy state**: the modal disables both buttons while `busy()` is true, preventing double-submission during the init + open sequence.
---
