The changes to `headless.rs` added retry logic for transient failures during indexing, but this doesn't affect the staleness detection logic itself — `meta.rs` remains unchanged. The edited files are all frontend/session UI and the headless indexing runner. The staleness detection doc is still accurate.

---
# Index Staleness Detection

## Purpose

Compares the on-disk index version and a content-hash manifest (`.codeforge/index-meta.json`) against the current file tree to decide whether `features.json` is stale, outdated, or fresh — without running Claude or re-indexing.

## How it works

- **Write phase**: after indexing, `meta::write` hashes every file referenced by any feature (entry points ∪ files) and stores `{ version, files: { path → sha256 } }` in `index-meta.json`.
- **Status phase**: `meta::status` reads both `features.json` and `index-meta.json`; if either is absent → `Never`.
- Compares stored `version` to `INDEX_VERSION` (bumped when indexing technique changes) → if older → `Outdated`.
- Re-hashes all manifest files and compares to stored hashes; any mismatch or missing file → `Stale` with the changed paths listed.
- If all hashes match and version is current → `Fresh`.
- Pure function — never executes `claude`, never triggers re-indexing — so the UI can safely check status and prompt the user.

## Key files

- `crates/forge-index/src/meta.rs` — `write` stores the manifest after indexing; `status` computes the staleness verdict by re-hashing; defines `IndexMeta`, `IndexState`, `IndexStatus`.
- `crates/forge-index/src/lib.rs` — exports `index_status` as the public entry point; defines `INDEX_VERSION` (bumped when indexing technique changes).

## Invariants & gotchas

- **Never silent fallback**: `IndexState` is a closed enum — `Never | Fresh | Stale | Outdated` — each with distinct UI behavior. No implicit "assume fresh."
- **Missing file at write time**: a referenced path that doesn't exist when `write` runs is omitted from the manifest (logged) and therefore never tracked for staleness — deliberate rule, not a silent guess.
- **Atomicity**: `write` uses tmp + rename so readers never see a partial file.
- **Version precedence**: `Outdated` verdict (stored version < current) is checked before hash mismatches — even if all hashes match, an old version → `Outdated`.
- **Platform-agnostic keys**: manifest paths are normalized to `/` separators for stable JSON serialization across Windows/Unix.
- **Serialization contract (FZ-2)**: `IndexStatus` serializes as `{ state, changedFiles, indexVersion, currentVersion }` with `state` a lowercase string (`"never"`, `"fresh"`, `"stale"`, `"outdated"`) — frozen contract mirrored in `frontend/src/types.ts`.
