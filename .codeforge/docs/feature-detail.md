# Feature Detail View

## Purpose

Displays a single feature's metadata (name, description, tags, pinned state), its entry points, files grouped by role, an auto-generated living doc, and recent timeline events. Supports inline editing of name and description and opens files in the user's editor.

## How it works

- Double-click the title or single-click the description to enter inline edit mode; changes commit on blur or Enter, Escape cancels.
- Entry points and files render as clickable buttons that invoke `openInEditor` via the Tauri shell plugin to open paths in the OS default handler.
- Files are grouped by role (`core`, `support`, `test`, `config`, `unknown`) in a fixed order, with a "also in N" chip when a file participates in other features.
- The living doc is parsed Markdown from `.codeforge/docs/<slug>.md`, fetched by `appStore.selectedFeatureDoc`; absent doc shows a "Generate docs" CTA that calls `appStore.reindex()`.
- Recent activity timeline is sliced to the first 15 events, each rendered with a kind-specific icon, accent color, and one-line summary derived from the typed payload.
- "Ask Claude about this feature" prefills the composer with a prompt about the selected feature.

## Key files

- **crates/tauri-app/frontend/src/views/FeatureDetail.tsx** — main view with inline-editable header, sections for entry points / files / doc / activity, and the Ask Claude button.
- **crates/tauri-app/frontend/src/components/feature/CollapsibleSection.tsx** — reusable 0fr→1fr grid-rows collapse with rotating chevron, label, optional count, and trailing slot.
- **crates/tauri-app/frontend/src/components/feature/FilesSection.tsx** — groups files by role with tint chips, shows "also in N" count for shared paths, opens files on click.
- **crates/tauri-app/frontend/src/components/feature/RecentActivity.tsx** — renders timeline events with kind-specific glyph, accent, relative time, and summary.
- **crates/tauri-app/frontend/src/components/feature/event-summary.ts** — maps `EventKind` to accent color, SVG glyph, and one-line summary by reading the discriminated payload.
- **crates/tauri-app/frontend/src/components/feature/open-path.ts** — joins repo-relative paths and invokes the Tauri shell plugin to open them in the OS editor.

## Invariants & gotchas

- The living doc is **never** the description standing in — absence means "not indexed yet", shown honestly by the fallback CTA, not silently replaced by `feature.description`.
- Files with role `"unknown"` are a distinct, visible state (the indexer couldn't classify the file) — never fold them into `"support"`.
- Timeline event summaries read from **typed, kind-discriminated payloads** — each `case` branch reads exactly the field(s) the backend writes for that kind. No key guessing.
- `openInEditor` requires the Tauri shell plugin and the `shell:allow-open` capability on the Rust side; missing either breaks file/entry-point clicks silently.
- "also in N" chip counts features that **also** reference the path, not total references — the current feature is excluded from the count.
