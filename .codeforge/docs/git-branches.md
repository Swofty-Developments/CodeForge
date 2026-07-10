# Branch Listing (git-branches)

## Purpose
Lists all local and remote-tracking branches in a git repo family (base + worktrees), cross-referencing worktree checkouts to show where each branch is currently checked out and which branch is HEAD.

## How it works
- Calls `git for-each-ref refs/heads refs/remotes --format=%(refname)%00%(HEAD)%00%(symref)` to enumerate all branches and HEAD markers; symbolic refs (e.g., `origin/HEAD`) are skipped.
- Parses the `\0`-delimited output: strips `refs/heads/` for locals, splits `refs/remotes/<remote>/<branch>` for remote-tracking refs.
- Cross-references local branches against `git worktree list` to populate `checked_out_at` (including the base repo itself).
- Returns locals first (alphabetical), then remotes (sorted by remote+name), with `is_head` marking the base's current branch.
- `fetch_remotes` runs `git fetch --all --prune` if remotes are configured (no-op if none exist).
- `worktree_for_branch` checks out an existing branch into `.codeforge-worktrees/<slug(branch)>`: local branches via `git worktree add`, remote branches via `git worktree add --track -b`.

## Key files
- `crates/forge-git/src/branches.rs` — listing logic, fetch, worktree-for-branch checkout
- `crates/forge-git/src/branches_tests.rs` — integration tests against real git repos

## Invariants & gotchas
- **Symbolic refs must be filtered**: `origin/HEAD` and similar symrefs appear in `for-each-ref` output but represent aliases, not branches; they're dropped via the `%(symref)` field check (line 46).
- **Remote branches never have `checked_out_at`**: only local branches can be checked out; remote-tracking refs are read-only pointers (lines 99–102).
- **Worktree checkout detects collisions**: attempting to check out an already-checked-out branch returns `Error::BranchCheckedOut` with the path, not silently failing (lines 152–158).
- **Remote mode requires no local branch**: `worktree_for_branch` with `remote: Some(r)` fails if a local branch of that name exists, preventing accidental divergence from the tracking branch (lines 162–171).
- **Malformed `for-each-ref` output is a parse error**: any line that doesn't match the expected `\0`-delimited format or ref namespace is a named error, never silently dropped (lines 42–58).
