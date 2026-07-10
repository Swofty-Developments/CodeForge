# Git Diff Extraction

## Purpose

Shells out to `git diff HEAD` and `git status --porcelain=v2` to extract pending changes (staged, unstaged, and untracked), parse hunks with exact line-number bookkeeping, and group files by feature via the classifier.

## How it works

- `collect_file_diffs` runs `git status --porcelain=v2` to enumerate changed files, then `git diff HEAD` (or against the empty tree on unborn HEAD) to extract tracked changes
- `parse::parse_unified_diff` hand-parses unified diff output into `FileDiff` structs with per-line old/new line numbers, handling renames, binary files, and mode-only changes
- `untracked::synthesize` reads untracked text files and presents them as all-added hunks (binary files flagged via NUL-byte sniff in first 8 KiB)
- `truncate::cap_file_diffs` applies a single authoritative 2000-line cap per file to both tracked and untracked diffs, marking capped files `truncated`
- `group::group_by_feature` buckets files by feature via the index classifier; files matching multiple features appear in each group with `shared: true`, unmapped files land in a synthetic "Unmapped" bucket ordered last
- All git commands run via `tokio::process::Command` with `.current_dir(repo_root)` and `core.quotepath=false` to preserve non-ASCII paths

## Key files

- `lib.rs` — public `diff_by_feature` entry point, `collect_file_diffs` orchestration, low-level `run_git` wrapper
- `parse.rs` — unified diff parser, hunk header (`@@ -old +new @@`) parsing, git path unquoting (C-style escapes + octal bytes)
- `group.rs` — feature grouping logic, shared-file handling, unmapped bucket synthesis, sort by changed lines desc
- `status.rs` — porcelain-v2 parser extracting path/status/orig_path from `1`/`2`/`?`/`u` lines
- `untracked.rs` — synthesize all-added diffs for untracked files, binary detection, empty file handling
- `truncate.rs` — single authoritative truncation at 2000 lines per file, preserving `additions`/`deletions` totals

## Invariants & gotchas

- **One truncation point**: `cap_file_diffs` is the sole cap; `parse` and `synthesize` never truncate, so there's no double-capping
- **Unborn HEAD**: when HEAD is unverifiable, diff against the empty tree (`git hash-object -t tree /dev/null`) so staged files parse as added
- **Shared-file semantics**: a file in N features appears as a distinct `FileDiff` clone in each group's `files` vec — the `shared` flag signals cross-feature presence, not deduplication
- **Binary detection**: both tracked (`Binary files … differ`) and untracked (NUL byte in first 8 KiB) binaries render as zero hunks with `binary: true`, never as phantom text
- **Unparseable diffs dropped**: sections without derivable paths (no `---`/`+++`/rename headers and an unparseable `diff --git` line lacking ` b/`) are dropped rather than faked
- **`core.quotepath=false`**: all git commands disable path quoting to preserve non-ASCII filenames; the parser still unquotes defensively for robustness
