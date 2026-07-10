# Diff Review View

## Purpose

Organizes pending git changes by feature (not by file). Sticky header with refresh control and total +/- counts; one accordion per feature group, with the unmapped bucket muted and always rendered last.

## How it works

- **DiffReview** fetches groups from `appStore.diff` and sorts via `orderedGroups` (mapped groups first, unmapped last).
- **Refresh button** calls `appStore.refreshDiff()` to rebuild the entire diff (backend query).
- **Each group** renders an accordion (DiffFeatureGroup) with its name, shared-flag indicator, aggregate +/- counts, and file count.
- **Each file** (FileDiffRow) shows a status chip (M/A/D/R/U), monospace path with dir muted, +/- counts, and a collapsible hunk body.
- **Hunks** (DiffHunks) render line-numbered old/new gutters with per-line syntax highlighting via hljs. Binary files surface as an explicit marker; truncated diffs append a "truncated at server limit" pill — no client-side re-capping.
- **Latched mounting**: Group and file bodies only mount once expanded — never-opened accordions never render their children.

## Key files

- **DiffReview.tsx** — Top-level view; sticky header, refresh control, groups loop.
- **DiffFeatureGroup.tsx** — Feature accordion (name, shared badge, counts, chevron, collapsible file list).
- **FileDiffRow.tsx** — One file row (status chip, mono path, +/- counts, collapsible hunks).
- **DiffHunks.tsx** — Renders hunks (headers + line rows), syntax-highlighted via hljs, binary/truncated markers.
- **diff-utils.ts** — Pure helpers (group ordering, counts, status mapping, extension→lang, buildRows, escapeHtml).

## Invariants & gotchas

- **Unmapped group is a backend flag** (`group.unmapped`), never a slug sniff. It must render last and muted.
- **Total counts dedupe by path** across shared files (a file can appear in multiple groups). Group counts do not dedupe.
- **Truncation is the backend's decision** (`FileDiff.truncated` set server-side). Frontend never re-caps hunks or lines.
- **Latching relies on `mounted` signal** — never reset it once true, or the hunk/file body re-mounts and flickers.
- **Status mapping is exhaustive** (M/A/D/R/U). Unknown statuses resolve to `?` with `"unknown"` class, never silently painted as modified.
- **One-time style injection** (`injectDiffStyles`) is idempotent; check `#diff-styles` before re-injecting or risk duplicate `<style>` blocks.
- **Syntax highlighting uses `innerHTML`** — never inject unsanitized backend content; `escapeHtml` fallback when hljs fails.
- **First group auto-opens** (first three files expanded) for immediate visual context. All others default closed.
