The edited files (session manager, Tauri commands, frontend session pane, agent sidecar) do not touch `crates/forge-git/src/branches.rs` or the branch-listing feature. The changes are about fixing mid-turn reasoning streams in the Claude session protocol, not branch enumeration. The living doc should remain unchanged.

---
# Branch Listing

## Purpose

Lists all local and remote branches in a repository, enriches each with its checkout location (base or worktree), and fetches remote refs to keep the list current. Powers the worktree switcher popover UI.

## How it works

- **Enumerates branches** via `git for-each-ref refs/heads refs/remotes --format=%(refname)%00%(HEAD)%00%(symref)`, parsing local heads (no remote prefix) and remote-tracking branches (`refs/remotes/<remote>/<name>`), skipping symbolic refs like `origin/HEAD`.
- **Cross-references worktrees** by fetching `git worktree list` and joining on branch name to populate `checked_out_at` for local branches (the base checkout included).
- **Marks the base HEAD** using the `%(HEAD)` `*` marker from `for-each-ref` to flag which branch is currently checked out in the base worktree.
- **Sorts deterministically**: locals alphabetically first, then remotes sorted by `(remote, name)`, so the UI always shows the same order.
- **Fetches remotes** with `git fetch --all --prune` in a separate call; a repo with no remotes logs and returns `Ok` rather than erroring.
- **Creates worktrees for branches** via `worktree_for_branch`, which either checks out an existing local branch (no `-b`) or creates a tracking branch from a remote ref (`--track -b`), surfacing named errors when the branch is already checked out elsewhere or when a local branch with that name already exists.

## Key files

- **`branches.rs`** — core implementation: `list_branches`, `fetch_remotes`, `worktree_for_branch`, and the `BranchInfo` model.
- **`branches_tests.rs`** — integration tests against real git repos: verifies checkout state, remote tracking, error conditions, and parse invariants.

## Invariants & gotchas

- **Symbolic refs must be skipped**: `origin/HEAD` and similar symrefs are not branches — `parse_ref_lines` checks `%(symref)` is empty, otherwise silently drops the line. If a non-symref line is malformed, it's a named parse error, never silent.
- **Remote-tracking branches are never marked as checked out**: `checked_out_at` is `None` for any `BranchInfo` with `remote: Some(_)` — worktrees check out *local* branches, not `refs/remotes/` directly.
- **Worktree creation is fail-fast on name collisions**: if the target path `.codeforge-worktrees/<slug(branch)>` exists, or the branch is already checked out elsewhere (for local mode), or a local branch already exists (for remote mode), return a named error rather than silently overwriting or deleting.
- **No remotes is a valid state**: `fetch_remotes` treats an empty `git remote` list as normal (logs at `info!` and returns `Ok`), not a failure.
- **Checked-out-at paths are absolute**: `BranchInfo.checked_out_at` is always an absolute `PathBuf` derived from `git worktree list`, never a relative or symbolic path.
- **Remote mode creates a tracking branch**: `worktree_for_branch(..., Some(remote))` runs `git worktree add --track -b <branch> <path> <remote>/<branch>`, so the new local branch tracks the upstream ref.
- **Slugification can fail**: if `slugify(branch)` returns an empty string (e.g., branch name is all invalid characters), return `Error::InvalidName` before attempting to create the worktree path.
