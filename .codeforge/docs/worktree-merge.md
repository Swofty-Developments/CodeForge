Now I have everything needed to write the living doc:

# Worktree Merge

**Purpose**  
Merges a worktree's branch into the base repo's current branch via `git merge --no-edit`. Returns a named outcome: clean success, conflict (with paths + auto-abort), or detached-HEAD rejection. Never leaves the base repo mid-merge.

**How it works**  
1. **Resolve base + validate worktree**: Finds the repo's main worktree (`git worktree list --porcelain` first entry), confirms the target worktree exists in that list, extracts its branch name. Rejects if the worktree is in detached HEAD or base repo is detached.
2. **Attempt merge**: Runs `git merge --no-edit <source_branch>` in the base worktree. Success (exit 0) → returns `MergeResult { merged: true, conflicts: [], aborted: false }`.
3. **Conflict path**: On non-zero exit, checks for `MERGE_HEAD` to distinguish:
   - **No MERGE_HEAD**: merge never started (dirty working tree, bad ref) → surface as `Error::Git(message)` with base untouched.
   - **MERGE_HEAD exists**: conflict → runs `git diff --name-only --diff-filter=U` to collect conflicted paths, then `git merge --abort` to revert, returns `MergeResult { merged: false, conflicts: [...], aborted: true }`.
4. **Naming**: Always returns `source_branch` and `target_branch` so callers can report what merged into what.

**Key files**  
- `crates/forge-git/src/worktree.rs:188-251` — `merge_worktree()`  
- `crates/forge-core/src/worktree.rs:29-41` — `MergeResult` schema

**Invariants & gotchas**  
- **Detached HEAD rejection is explicit**: Both base and worktree must be on a named branch. Detached worktree → `Error::WorktreeDetached`, detached base → `Error::BaseDetached`. This is load-bearing: git can't create a sensible merge commit when HEAD isn't symbolic.
- **Auto-abort on conflict**: Conflicts are reported but never left for the user to resolve — the tool calls `git merge --abort` immediately so the base stays clean. Callers get the conflict list to show the user *what would conflict*, not to hand them a broken state.
- **Dirty base is an error before merge starts**: If the base has uncommitted changes, `git merge` exits non-zero *before* writing MERGE_HEAD. The no-MERGE_HEAD path catches this and surfaces it as `Error::Git(...)` rather than pretending it's a conflict.
- **path_str canonicalization**: Uses `same_path()` with fallback canonicalization for macOS symlink issues (`/var` vs `/private/var`).
- **The `--no-edit` flag**: Merge commits use git's auto-generated message ("Merge branch '...'"). No interactive editor, no custom message.
