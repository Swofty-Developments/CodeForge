Looking at the edited files and the agent's notes, I can see this turn was entirely about fixing session streaming (assistant message diffing in the sidecar), not about the cold-start indexing feature. None of the cold-start indexing files (`crates/forge-index/src/{indexer,headless,prompt,parse}.rs`) were touched.

The doc should remain unchanged. Removing the stale meta-note from the previous update:

# Cold-Start Indexing

## Purpose

Headless `claude -p` orchestrator that decomposes a repository into features from scratch. Spawns a read-only Claude session with a decomposition prompt, validates the returned JSON array, clamps file paths to existing files inside the repo, then generates concise living docs for each feature in parallel.

## How it works

- **Decomposition pass**: spawns `claude -p --output-format json` in the repo root with a prompt requesting a strict JSON feature array; the model explores the repo (Read/Glob/Grep only) and returns features with slugs, names, descriptions, entry points, file lists, and optional hierarchical groups.
- **Validation**: strips markdown fences from the reply, parses the JSON (falling back to the outermost `[...]` slice if wrapped in prose), sanitizes slugs to kebab-case, canonicalizes all paths relative to the repo root, drops paths that escape the repo or don't exist, and rejects features left with no files. Empty result sets are hard errors (named failure, not an empty success).
- **Group resolution**: uses the model's explicit `group` field when present (trimmed), otherwise derives a hierarchical group from the common directory prefix of the feature's files — skips monorepo containers (`crates/<name>` → `<name>`) and structural noise (`src`), keeps up to 3 meaningful segments (e.g., `atomix-web/pages/markets`).
- **Parallel doc generation**: fans out one headless `claude -p` call per non-pinned feature (capped at `DOC_CONCURRENCY=3`), each receiving a prompt with the feature's metadata and key files, instructed to return a ~60-line living doc (Purpose / How it works / Key files / Invariants); the model returns markdown (fences stripped), which this crate writes to `.codeforge/docs/<slug>.md` via atomic temp-file rename.
- **Progress streaming**: emits `IndexProgress` events (`scan` → `decompose` → `validate` → per-feature `docs` → `done`) on an mpsc channel; the caller forwards these to the frontend as real-time updates.
- **Timeout & error handling**: each headless invocation times out after 6 minutes; the child is killed on drop. Doc tasks that fail (model error, timeout, panic) are logged and reported in `DocReport.failed` so the caller can surface them rather than silently claiming success.

## Key files

- **`indexer.rs`** — public `Indexer` orchestrator with `cold_start()` (decomposition + validation) and `write_feature_docs()` (parallel doc pass); emits progress, enforces non-empty feature set, returns `DocReport` with written/failed counts.
- **`headless.rs`** — spawns `claude -p --output-format json` with a 6-minute timeout, pipes the prompt on stdin, parses the JSON envelope (`{"type":"result","subtype":"success","result":"..."}`) or bare array, extracts the result text, and returns errors on timeouts / non-zero exits / error envelopes.
- **`prompt.rs`** — `decomposition_prompt()` (instructs the model to explore the repo and return a strict JSON feature array, emphasizing granularity and multi-level groups) and `doc_prompt(feature)` (requests a concise living doc under 60 lines with Purpose / How it works / Key files / Invariants sections).
- **`parse.rs`** — `parse_features()` strips markdown fences and parses the JSON (with prose-wrapped fallback); `validate_features()` sanitizes slugs, canonicalizes paths, drops non-existent files and `../` escapes, dedupes slugs, filters out features with no files, and sorts by slug; `derive_group()` computes hierarchical groups from common directory prefixes with monorepo/noise skipping and depth capping.

## Invariants & gotchas

- **Read-only exploration**: headless invocations are constrained to `Read,Glob,Grep` (`--allowedTools`) — no Bash, no edits. Breaking this would let the model mutate the repo during indexing.
- **Atomic file writes**: docs and the index are written via temp-file + rename to prevent readers from observing half-written content; skipping the temp would break concurrent readers.
- **Non-empty validation**: a decomposition pass that yields zero features after validation is a hard `Error::Indexer`, not an empty success — merging an empty result set would drop the entire index. The code assumes a real repo always decomposes into ≥1 feature.
- **Group depth cap**: `derive_group()` clamps to `MAX_GROUP_DEPTH=3` meaningful segments; removing the cap would bloat the sidebar tree for deeply-nested features.
- **Pinned docs are skipped**: `write_feature_docs()` filters `features.iter().filter(|f| !f.pinned)` so human-edited docs survive re-indexing verbatim; a filter-removal would overwrite pinned docs.
- **Failed doc tasks are surfaced, not swallowed**: each doc task returns `Option<String>` (the failed slug or None), and `DocReport.failed` accumulates them; a silent swallow (always returning None) would let progress claim "docs complete" over docs that never wrote.
