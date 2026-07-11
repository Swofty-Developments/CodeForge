# Feature Detail View

## Purpose

Single-pane view that shows a selected feature's metadata (name, description, tags), entry points (paths openable in editor), files grouped by role with shared-feature markers, a living doc (auto-generated Markdown rendered inline), recent activity timeline, and an "Ask Claude" shortcut that prefills the composer.

## How it works

- Double-click the title or click the description to enter inline edit mode; `Enter`/`Escape` commits/cancels name changes, `Escape` cancels description.
- Entry points and file rows are clickable buttons invoking `openInEditor()` via Tauri's shell plugin to open the path in the OS default editor.
- Files are grouped by `FileRole` (core → support → test → config → unknown) with tint-coded chips; shared files show "also in N" purple chip counting overlap with other features.
- Living doc is rendered from `.codeforge/docs/<slug>.md` via `marked.parse()` — absence shows an honest empty state with a "Generate docs" button that triggers reindexing.
- Recent activity displays the first 15 timeline events (newest first) with kind-specific SVG glyphs, relative timestamps, and fade-slide-down animation on entry.
- **Living-doc hot reload**: when a `doc_updated` event lands for the **open** feature with `outcome: "updated"`, FeatureDetail pulls the fresh `.codeforge/docs/<slug>.md` and re-renders in place (stale-response guard blocks if the user switched features mid-fetch).
- "Ask Claude" button prefills the composer with `Explain the <name> feature and its current state`.

## Key files

- **FeatureDetail.tsx** — main view orchestrating header, collapsible sections, edit states, the Ask Claude hand-off, and doc hot-reload
- **CollapsibleSection.tsx** — shared 0fr→1fr grid-rows collapse with rotating chevron and optional count/trailing content
- **FilesSection.tsx** — role-grouped file rows with tint chips and shared-feature overlap detection
- **RecentActivity.tsx** — timeline event list with kind glyphs, summaries, and relative timestamps
- **open-path.ts** — Tauri shell plugin wrapper to open files in OS editor

## Invariants & gotchas

- **Living doc is never synthesized** — `store.selectedFeatureDoc` is the canonical `.codeforge/docs/<slug>.md` or `undefined`; never fall back to rendering the description as a doc.
- **"unknown" role is distinct** — not a fallback category; explicitly rendered with its own tint and never collapsed into "support".
- **File overlap counts exclude the current feature** — `alsoIn()` map only counts OTHER features; a file in 3 total features shows "also in 2".
- **Edit commits on blur** — name/description inputs auto-focus/select on mount and save when focus is lost; `Enter` commits name changes but not description (multi-line).
- **RECENT_LIMIT is a hard cap** — UI always shows ≤15 events; backend may send more but the view slices `[0, 15)` before rendering.
- **Doc updates are additive to the timeline** — the `doc_updated` event stays visible (with the outcome) even as the doc re-renders; the live event is the audit trail.
- **Stale-response guard** — `handleTimelineEvent` only updates `selectedFeatureDoc` if `store.selectedFeature === slug` at response time; prevents race when the user rapidly clicks between features.
- **Tauri invoke expects `{path, with: null}`** — `openInEditor()` manually constructs the shell plugin payload; changing the signature breaks file opening.
