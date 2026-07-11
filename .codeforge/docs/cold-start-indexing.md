# Cold-Start Indexing

## Purpose

Spawns headless `claude -p` with a decomposition prompt to map an unindexed repository into a feature set, validates candidate features (rejecting those with no existing files), then runs concurrency-capped (3 at a time) per-feature doc passes. Returns the validated features and a doc-write report distinguishing successes from failures.

## How it works

- `cold_start` emits progress stages (`scan` → `decompose` → `validate`), runs headless claude with `--output-format json` (sonnet, read-only tools: Read/Glob/Grep, 6-minute timeout), parses strict JSON into raw features, clamps paths to files that exist inside the repo, rejects empty slugs / duplicates / features left with zero files, and fails explicitly (never returns empty) if decomposition produced nothing
- `write_feature_docs` skips pinned features (human-edited docs survive), spawns up to `DOC_CONCURRENCY` (3) concurrent per-feature doc tasks, each running headless claude with a file-grounded prompt, stripping markdown fences, clamping to 60 lines, and writing atomically via `.tmp` rename
- Each doc task reports its outcome (success or the failed slug), so the caller surfaces failed docs explicitly — a swallowed failure would claim "complete" over an unwritten doc
- Retries transient failures (network, rate-limits, 503/504) up to 3 times with exponential backoff; timeouts and non-transient errors fail immediately
- Validation canonicalizes paths to reject `../` escapes, derives groups from the longest shared directory prefix (collapsing monorepo `crates/<x>` to `<x>`, stripping `src/` noise, nesting up to 3 levels), and maps unrecognized file roles to the named `Unknown` state (not conflated with the deliberate `Support` default)

## Key files

- `crates/forge-index/src/indexer.rs` — orchestrates cold-start / doc-writing phases, emits progress, caps concurrency, reports failures explicitly
- `crates/forge-index/src/headless.rs` — spawns `claude -p --output-format json` in the repo, extracts result from the JSON envelope, retries transient failures
- `crates/forge-index/src/prompt.rs` — decomposition prompt (strict JSON feature array) and per-feature living-doc prompt
- `crates/forge-index/src/parse.rs` — parses/validates model reply: strips fences, recovers arrays from prose, clamps paths, derives groups, sanitizes slugs

## Invariants & gotchas

- **Empty is failure**: decomposition returning zero features is an explicit error (`Error::Indexer`), never a success the caller would merge (which would drop the entire index)
- **Doc failures are named**: a task that fails to write a doc returns its slug in `DocReport::failed`, not silently counted as `written` — the caller surfaces these to the user so progress never claims completion over an unwritten doc
- **Pinned features are skipped**: `write_feature_docs` filters `!f.pinned` so human-edited docs survive reindex verbatim
- **Atomic writes**: docs are written to `.md.tmp` then renamed, so a killed/timeout task never leaves a truncated doc
- **Paths are validated**: validation canonicalizes and strips `../` escapes; absolute paths inside the repo are normalized to repo-relative
- **Groups are deterministic**: when the model supplies a non-blank group it wins verbatim; otherwise `derive_group` applies a named rule (monorepo containers collapsed, `src/` noise dropped, up to 3 levels kept); features sharing no common directory derive to `None` (ungrouped), not a synthetic group
