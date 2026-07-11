These files don't touch the git diff extraction feature at all. This appears to be an agent working on a different feature (session pane UI). The living doc should remain unchanged.

---
# Git Diff Extraction

## Purpose

Shells out to `git` to extract the full pending diff via `status --porcelain=v2` and `diff HEAD`, parses unified diffs into structured hunks with line-number bookkeeping, synthesizes all-added diffs for untracked text files, and caps oversized files at 2000 rendered lines per file.

## How it works

- **Status parsing**: runs `git status --porcelain=v2 --untracked-files=all` with `core.quotepath=false` and parses the structured output into modified/added/deleted/renamed/untracked entries
- **Diff extraction**: runs `git diff --no-color --unified=3 --find-renames HEAD` (or against the empty-tree hash when HEAD is unborn) and hand-parses the unified diff format into hunks
- **Untracked synthesis**: reads untracked files directly from disk, detects binary via NUL-byte sniff in the first 8 KiB, and presents text files as a single all-added hunk (`@@ -0,0 +1,N @@`)
- **Truncation**: applies a single 2000-line-per-file cap to all diffs (tracked + untracked) after collection, marking capped files `truncated: true` while preserving true `additions`/`deletions` counts
- **Feature grouping**: delegates path-to-feature classification to `FeatureIndex`, placing each file in N groups when it spans features (marked `shared: true`) and unmapped files in an "unmapped" group
- **Line number tracking**: maintains exact old/new line numbers per `DiffLine` by counting additions/deletions/context as the hunk is parsed

## Key files

- **`crates/forge-git/src/lib.rs`** — public API: `diff_by_feature`, `changed_files`, `init_repo`, `current_head`/`current_branch`; shells out via `tokio::process::Command` with `.current_dir(repo_root)`
- **`crates/forge-git/src/status.rs`** — parses `git status --porcelain=v2` into `StatusEntry` (path, orig_path for renames, status string)
- **`crates/forge-git/src/parse.rs`** — hand-rolled unified diff parser: splits on `diff --git`, feeds lines through a state machine tracking open hunks, decodes git's C-style path quoting (octal escapes), and derives status from headers (`new file mode`, `deleted file mode`, `rename from/to`)
- **`crates/forge-git/src/untracked.rs`** — reads untracked files from disk, sniffs first 8 KiB for binary detection, synthesizes all-added hunks for text
- **`crates/forge-git/src/truncate.rs`** — single authoritative truncation at 2000 lines per file; `FileDiff::truncated` is the source of truth for the frontend (no re-truncation downstream)

## Invariants & gotchas

- **No libgit2**: always shell out to `git` CLI; `current_dir` must be repo root or a subdirectory
- **Unborn HEAD**: `diff HEAD` against the empty-tree hash (`git hash-object -t tree /dev/null`) when HEAD is unborn so staged files parse as added
- **Binary detection**: NUL byte in first 8 KiB → `binary: true`, zero hunks; applies to both tracked (from `git diff` output) and untracked (via sniff)
- **One truncation pass**: `truncate::cap_file_diffs` runs once in `collect_file_diffs` over all files; `synthesize` and `parse_unified_diff` do not cap
- **Path quoting**: git can C-style quote paths (`"pa\ttern"`, `"\303\270"`); `parse::unquote` handles octal byte escapes and backslash sequences
- **Empty diffs are real**: mode-only changes, empty untracked files, and unparseable `diff --git` headers (missing ` b/` marker) all produce valid states — mode-only changes keep the path via the `b/` marker, empty files have zero hunks, unparseable headers are dropped with a warning rather than synthesizing phantom paths
- **Renames report new path**: `changed_files` returns the destination path for renames; `StatusEntry::orig_path` holds the old path when needed
- **Line-number bookkeeping**: `OpenHunk` tracks `old_no`/`new_no` and counts down `old_left`/`new_left` as it parses; a hunk closes when counts reach zero or a new `@@` header appears
- **Shared files duplicate across groups**: a file in N features appears in each group's `files` array with `shared: true`; no deduplication by the grouping layer
