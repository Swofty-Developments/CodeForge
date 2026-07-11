The edited files this turn (app-store.ts additions, forge-index.rs, SessionPane.tsx, and session styles) are unrelated to the merge result panel. The component and its behavior remain unchanged. The existing doc is still accurate.

---
# Merge Result Panel

## Purpose

Displays the outcome of a worktree branch merge into the base branch: a clean success triggers a transient toast and auto-dismisses; conflicts/aborted merges render a modal overlay listing conflicted files with a one-click hand-off to resolve them in a Claude session.

## How it works

- **Clean merge (merged=true)**: `createEffect` fires a success toast via `pushSuccess` and immediately clears `mergeResult` from the store — no panel is shown.
- **Conflict/abort (merged=false)**: renders a full-screen modal overlay with a warning icon, conflict message, list of conflicted files, raw git stderr, and two actions: Dismiss or "Resolve in a session."
- **"Resolve in a session" flow**: switches to the base context, prefills the composer with a message explaining the abort + listing conflict files, and dismisses the panel — the user can then submit to Claude.
- **Honest reporting**: the panel only shows what git actually did (clean success, conflicts, abort). Never fabricates success or hides failure.
- **Dismissal**: clicking the overlay backdrop or Dismiss button clears `store.mergeResult` via `dismissMergeResult()`.
- **State wiring**: `mergeResult` is set by `context-slice.ts:mergeWorktree()` after calling the IPC merge command; the panel observes this reactive store field.

## Key files

- `crates/tauri-app/frontend/src/components/MergeResultPanel.tsx` — the panel component, effect-driven clean-merge toast, and conflict UI
- `crates/tauri-app/frontend/src/stores/context-slice.ts:mergeWorktree()` — calls IPC merge, sets `store.mergeResult`
- `crates/tauri-app/frontend/src/types.ts:MergeResult` — outcome shape: `{merged, conflicts, aborted, message, sourceBranch, targetBranch}`

## Invariants & gotchas

- **Never show panel on clean merge**: the `Show when={!merged}` guard ensures only failures render the panel; success toasts silently.
- **Auto-dismiss timing**: clean merge dismissal happens in `queueMicrotask` to avoid React-like batching issues — don't move it to a setTimeout or the toast may appear after the result is cleared.
- **Base branch is always clean after abort**: the message explicitly states the target branch was left untouched — the merge was aborted, not partially committed.
- **Conflict file list can be empty**: git may report a merge failure without listing specific files (rare edge cases); the panel conditionally shows the file list only when `conflicts.length > 0`.
- **Prefill composer, don't auto-submit**: the "Resolve in a session" button prefills the composer but does not auto-send — the user reviews the generated prompt before submitting to Claude.
