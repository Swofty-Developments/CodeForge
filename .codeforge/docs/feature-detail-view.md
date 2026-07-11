The changes this turn were to backend Tauri IPC command registration (`main.rs`), command module organization (`commands/mod.rs`), and the headless indexer's retry logic (`headless.rs`). None of these affect the Feature Detail View's frontend behavior, UI rendering, or interaction patterns. The doc remains accurate.

---
# Feature Detail View

## Purpose

Main panel showing a single feature's metadata, documentation, files, and timeline. Supports inline editing (name, description) with optimistic updates and autosave to the store.

## How it works

- Renders editable header (double-click name, click description) that commits via `appStore.updateFeature` on blur/Enter, Escape cancels.
- Pin button toggles `appStore.pinFeature` to keep the feature in the sidebar's pinned section.
- Groups files by `FileRole` (core/support/test/config/unknown) with role-tinted chips; shows "also in N" badge when a file appears in multiple features.
- Entry points open via Tauri shell plugin (`plugin:shell|open`) to OS default editor.
- Living doc section parses `store.selectedFeatureDoc` (the `.codeforge/docs/<slug>.md` content) as Markdown; absent doc shows "Generate docs" button that triggers reindex.
- Recent activity feed shows last 15 timeline events, newest first, with kind-specific glyphs and relative timestamps updated every 10s.
- "Ask Claude" button prefills composer with feature-specific prompt via `appStore.prefillComposer`.

## Key files

- `views/FeatureDetail.tsx` — main view orchestration, editable header, collapsible sections.
- `components/feature/FilesSection.tsx` — role-grouped file list with cross-feature overlap badges.
- `components/feature/RecentActivity.tsx` — timeline event list with kind glyphs and relative time.
- `components/feature/CollapsibleSection.tsx` — shared 0fr→1fr grid-rows collapse with rotating chevron.
- `components/feature/event-summary.ts` — per-kind accent, glyph path, one-line summary extraction.
- `components/feature/open-path.ts` — Tauri shell plugin wrapper for opening paths in OS editor.

## Invariants & gotchas

- **Living doc is authoritative**: `.codeforge/docs/<slug>.md` is the single source of truth; `description` field never substitutes for it. Absent doc shows empty state, not description fallback.
- **Inline edits must trim**: `commitName`/`commitDesc` trim before comparing to prevent spurious whitespace-only updates.
- **File roles are distinct**: `unknown` is a visible, tinted role (indexer didn't classify), never folded into `support`.
- **Event payloads are kind-discriminated**: `summarize` reads exactly the fields each kind's backend emits; no key guessing or fallback parsing.
- **Entry points open via shell plugin**: requires `shell:allow-open` capability in Tauri config; direct `invoke("plugin:shell|open")` avoids bundling the full JS wrapper.
- **Optimistic UI with no rollback**: edits commit immediately to `appStore.updateFeature` on blur; Escape cancels editing but doesn't revert a committed change.
- **Pin state persists**: pin toggle calls `appStore.pinFeature`, which writes to the feature index and keeps the feature at the top of the sidebar even when filtering.
