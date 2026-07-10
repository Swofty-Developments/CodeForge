# Feature IPC Commands

## Purpose

Exposes Tauri commands for frontend feature queries (`get_features`, `get_feature`, `get_feature_doc`) and mutations (`pin_feature`, `update_feature`, `set_feature_color`) against the in-memory feature index. All mutations append timeline events and persist to `.codeforge/features.json`.

## How it works

- Every command canonicalizes `repo_path` and retrieves the `FeatureIndex` + `TimelineStore` from `AppState.repos`.
- Read commands (`get_features`, `get_feature`, `get_feature_doc`) acquire a read lock and clone the feature or read the living doc from `.codeforge/docs/<slug>.md`.
- Write commands (`pin_feature`, `update_feature`, `set_feature_color`) acquire a write lock, mutate the index, save to disk (`index.save()`), then append a timeline event (`FeaturePinned`, `FeatureEdited`).
- `update_feature` applies a `FeaturePatch` (name, description, tags) and implies a pin; `set_feature_color` delegates to `feature_color::set_color` for validation (`#RGB` or `#RRGGBB`) and also implies a pin.
- Timeline append failures are surfaced to the frontend as named errors (e.g., `"edit persisted but timeline append failed: …"`) — the timeline is the durable audit log, so a lost event is a real failure.
- `index_status` is pure: re-hashes manifest files to compute staleness without holding the index lock or re-indexing.

## Key files

- **`crates/tauri-app/src/commands/features.rs`** — all six Tauri commands; orchestrates index + timeline mutations.
- **`crates/tauri-app/src/runtime/feature_color.rs`** — color validation (`#RGB`/`#RRGGBB`) and the `set_color` mutation, unit-tested against a real `FeatureIndex`.
- **`crates/tauri-app/src/state.rs`** — `AppState` holds the `repos: HashMap<PathBuf, RepoRuntime>` map; `RepoRuntime` bundles the `Arc<RwLock<FeatureIndex>>` + `Arc<TimelineStore>`.
- **`crates/forge-core/src/feature.rs`** — `Feature`, `FeaturePatch` types with `#[serde(rename_all = "camelCase")]` for IPC.

## Invariants & gotchas

- **Timeline append failures are surfaced as errors**, not silent log lines — the timeline is the source of truth for human edits, and a lost event violates the audit contract.
- **All mutations imply a pin** (`pin_feature`, `update_feature`, `set_feature_color`) — pinned features survive re-index verbatim; the UI must make this clear.
- **Color validation is strict** — only `None` (clear) or `#RGB`/`#RRGGBB` hex strings are accepted; anything else is a named error. No silent upsert of unknown slugs (color must be set on an existing feature).
- **`get_feature_doc` returns `Option<String>`** — `None` if the living doc has not been generated yet (`.codeforge/docs/<slug>.md` missing), not an error.
- **`index_status` works whether or not the repo is open** — it reads on-disk state (`.codeforge/features.json` + `index-meta.json`) and never holds the index lock, so the UI can poll it safely.
