The merge result panel feature was not touched this turn — the changes were to staleness detection. The existing doc remains accurate.

---
# Merge Result Panel

## Purpose

Displays the outcome of a git merge attempt honestly: clean merges auto-dismiss with a success toast, conflicted merges show a persistent overlay listing the files that conflict and offering a one-click hand-off to resolve them in a Claude session. The panel never fabricates success or hides conflicts.

## How it works

- **Outcome detection** — React effect watches `store.mergeResult`. If `merged: true`, a success toast fires and the result clears automatically (lines 14–23).
- **Conflict UI** — When `merged: false`, the panel renders: a warning icon, a summary explaining the merge was *aborted* and the target branch is still clean, the list of conflicted files (if any), and the raw git message (lines 39–77).
- **"Resolve in a session" button** — Switches to the base context, pre-fills the composer with a prompt explaining which files conflict, then dismisses the panel (lines 25–36).
- **State storage** — `mergeResult: MergeResult | null` in the global store (app-store.ts:50). Backend writes it after merge attempts; the panel reads and clears it. Structure: `{ merged, conflicts, aborted, message, sourceBranch, targetBranch }`.
- **No automatic resolution** — The panel explicitly states the merge was aborted and the target branch is clean (line 54). User must re-run the merge and fix conflicts manually or via an agent; the app never silently applies a conflict resolution strategy.

## Key files

- **MergeResultPanel.tsx** — The overlay component. Watches `store.mergeResult`, auto-dismisses clean merges, surfaces conflicts with a file list and session hand-off.
- **app-store.ts** — Global store shape; `mergeResult: MergeResult | null` at line 50.
- **context-slice.ts** — Provides `dismissMergeResult()` action (line 335).
- **types.ts** — `MergeResult` interface (lines 201–208): `merged`, `conflicts`, `aborted`, `message`, `sourceBranch`, `targetBranch`.

## Invariants & gotchas

- **Never fabricate success** — Only the backend writes `merged: true`; the frontend must never infer or fake a successful merge state.
- **Aborted = clean target** — If `merged: false`, git aborted the merge and the target branch is unchanged. The panel's copy must remain accurate: "aborted" and "left clean" (line 54).
- **Auto-dismiss is for clean merges only** — The effect (lines 14–23) dismisses *only* when `merged: true`. Conflicted merges (`merged: false`) persist until the user clicks Dismiss or Resolve.
- **Conflicts array may be empty** — If the backend can't parse the conflict list, `conflicts: []` is valid. The UI shows "the reported files" as a fallback (line 30).
- **Hand-off switches context** — The "Resolve in a session" flow switches to the base context *first* (line 29), so the prefilled prompt runs against the correct worktree, not a detached worktree that might have been active.
