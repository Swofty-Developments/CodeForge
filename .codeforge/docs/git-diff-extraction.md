# Git Diff Extraction

## Purpose

Shells out to `git` CLI to capture working-tree state (`git status --porcelain=v2` + `git diff HEAD`), parses unified diff format into structured hunks with exact line numbers, and synthesizes all-added diffs for untracked text files.

## How it works

- Runs `git status --porcelain=v2 --untracked-files=all` (with `core.quotepath=false`) to collect staged, unstaged, and untracked files
- Runs `git diff --no-color --unified=3 --find-renames HEAD` (or diffs against the empty tree when HEAD is unborn) to extract hunks
- Parses unified diff output with a hand-rolled parser (`parse.rs`) that tracks old/new line numbers per hunk, detects renames/copies/mode-only changes via headers, and handles C-style path quoting
- Synthesizes full-file-added diffs for untracked text files; binary files (NUL byte in first 8 KiB) are flagged `binary: true` with zero hunks
- Caps every file at 2000 rendered lines (one authoritative truncation in `truncate::cap_file_diffs`) while preserving true `additions`/`deletions` counts
- Groups files by feature via `index.classify_paths`; shared files (N features) appear in each group; unmapped files land in a synthetic "unmapped" group

## Key files

- `lib.rs` — public API: `diff_by_feature`, `changed_files`, `init_repo`, `current_head/branch`, git command runner (`run_git`)
- `status.rs` — porcelain-v2 parser: extracts `(path, orig_path, status)` triples from `git status` output
- `parse.rs` — unified diff parser: hand-rolled state machine for `git diff` output, exact line-number tracking, binary/rename detection
- `untracked.rs` — synthesizes all-added `FileDiff`s for untracked text files (NUL-byte sniff for binary detection)
- `truncate.rs` — single authoritative cap at 2000 lines per file, marks `FileDiff::truncated`, preserves true totals
- `group.rs` — groups `FileDiff`s by feature via `classify_paths`, flags shared files, creates "unmapped" bucket
- `tests.rs` — end-to-end tests against real git repos in tempdirs

## Invariants & gotchas

- **One truncation point**: `truncate::cap_file_diffs` is the only place diff lines are capped; parsers must not cap inline
- **Unborn HEAD**: `current_head` returns `Ok(None)` on unborn branch; diff falls back to empty tree (`git hash-object -t tree /dev/null`) so staged files parse as added
- **Detached HEAD**: `current_branch` returns `Ok(None)` on detached HEAD or unborn branch — a real state, not an error
- **Binary detection**: NUL byte in first 8 KiB → `binary: true`, zero hunks; tracked binaries get "Binary files differ" from git, same result
- **Rename paths**: `StatusEntry::path` is the new path, `StatusEntry::orig_path` is old; renames with content changes produce hunks against the new path
- **Line-number bookkeeping**: each `DiffLine` carries `(old_no, new_no)` — deletions have `old_no` only, additions have `new_no` only, context has both
- **Shared files**: a file in N features appears in each group's `files` with `shared: true` set on all affected groups
- **Unmapped bucket**: always last in `DiffByFeature::groups`, slug "unmapped", name "Unmapped", `shared: false`
- **Quotepath off**: all git commands pass `-c core.quotepath=false` so paths with spaces/UTF-8 come back unquoted
- **Unparseable diffs**: sections without a derivable path (no `---`/`+++`/rename header and unparseable `diff --git` line) are dropped with a warning, not faked
