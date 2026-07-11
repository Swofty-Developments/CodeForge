# Branch Listing

## Purpose

Parses `git for-each-ref refs/heads refs/remotes` output into structured `BranchInfo` records, enriching each with checkout state (`is_head`, `checked_out_at`) by cross-referencing `git worktree list`. Powers the branch picker UI and enables safe worktree creation from existing local or remote-tracking branches.

## How it works

- `list_branches` runs `git for-each-ref` with NUL-delimited format `%(refname)%00%(HEAD)%00%(symref)` to enumerate all local heads (`refs/heads/*`) and remote-tracking refs (`refs/remotes/*/*`).
- Symbolic refs (e.g. `origin/HEAD → origin/main`) are silently skipped via the `%(symref)` field; any malformed line is a named parse error (never silently dropped).
- Local branches are cross-referenced against `git worktree list` to fill `checked_out_at` with the absolute worktree path; the base checkout's branch gets `is_head: true`.
- Returns locals first (alphabetical), then remotes (sorted by remote name, then branch name).
- `fetch_remotes` runs `git fetch --all --prune` on the base repo; a repo with no remotes is logged and reported as `Ok` (not an error).
- `worktree_for_branch` checks out an existing branch into a new worktree at `.codeforge-worktrees/<slug(branch)>`, with two modes: local branch (no `-b`) or remote-tracking branch (`--track -b` to create local tracking ref).

## Key files

- **crates/forge-git/src/branches.rs** — core parsing (`parse_ref_lines`), `list_branches`, `fetch_remotes`, `worktree_for_branch`.
- **crates/forge-git/src/branches_tests.rs** — end-to-end tests against real git repos with remotes, worktrees, and symrefs.

## Invariants & gotchas

- **Symbolic refs MUST be skipped silently** — `origin/HEAD` is not a branch, it's an alias; listing it would duplicate `origin/main` in the UI.
- **Malformed `for-each-ref` lines are named errors** — any ref outside `refs/heads/` or `refs/remotes/`, or a remote ref without a branch name (e.g. `refs/remotes/lonely`), is an explicit parse failure (never silently dropped).
- **`worktree_for_branch(remote: None)` errors if the branch is already checked out** — `Error::BranchCheckedOut` carries the existing worktree path so the caller can offer to open it instead of creating a duplicate.
- **`worktree_for_branch(remote: Some(r))` errors if a local branch of that name exists** — `Error::BranchExists` tells the caller to pick the local branch instead of `r/branch` (a tracking branch would conflict with the existing local).
- **`fetch_remotes` treats no-remotes as success** — a repo with no remotes configured is a real, valid state; the function logs it and returns `Ok(())` (not an error).
- **Slugged worktree paths** — `worktree_for_branch` slugifies the branch name (`feat/login` → `feat-login`) for the worktree directory, never the branch name itself.
