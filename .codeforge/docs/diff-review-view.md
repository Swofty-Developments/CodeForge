---
# Diff Review View

## Purpose

Displays the working tree's changes grouped by feature (not file). Each feature becomes a collapsible accordion with per-file hunks, syntax-highlighted diffs, and +/− counts; files that match no feature land in a muted "unmapped" bucket always rendered last.

## How it works

- `DiffReview` fetches `store.diff.groups` (from `diff_by_feature` backend), sorts via `orderedGroups` (mapped first, unmapped last), drops empty groups, and auto-opens the first.
- Each `DiffFeatureGroup` accordion shows feature name, a `shared` badge (if multiple features claim it), aggregate +/− across files, and renders `FileDiffRow` children lazily (grid `0fr → 1fr`; never-opened groups never mount their diffs).
- `FileDiffRow` has a status-letter chip (M/A/D/R/U), directory/basename split path, and per-file +/−; the hunk body mounts once expanded and delegates to `DiffHunks`.
- `DiffHunks` flattens `file.hunks[]` into render rows (hunk headers + lines with old/new line numbers), applies highlight.js per extension, and marks binary files or backend-truncated diffs with explicit typed flags (`FileDiff.binary`, `FileDiff.truncated`).
- Shared files (assigned to multiple features) appear under each feature group; `totalCounts` deduplicates by path when summing repository-wide +/−.

## Key files

- **DiffReview.tsx** — top-level view: sticky header with refresh and totals, ordered feature groups, empty state when tree is clean.
- **DiffFeatureGroup.tsx** — one feature accordion: name, shared badge, aggregate counts, collapsible file list with mount-latching.
- **FileDiffRow.tsx** — one file row: status chip, split path, +/− counts, collapsible hunk body.
- **DiffHunks.tsx** — hunk renderer: muted headers, dual-gutter line numbers, +/− tinted rows, per-line highlight.js, binary/truncation markers.
- **diff-utils.ts** — pure helpers: `orderedGroups`, `totalCounts` (deduped), `statusMeta`, `langForPath`, `buildRows` (never re-truncates), `injectDiffStyles`.

## Invariants & gotchas

- **Deduplication boundary**: shared files are replicated across groups in the data structure; `totalCounts` deduplicates by path, but per-group `groupCounts` sums every file it holds (no dedup within a group — paths are unique per group by backend contract).
- **Unmapped detection**: `isUnmapped(group)` checks `group.unmapped` flag (backend contract), never sniffs the slug string; `orderedGroups` always places unmapped groups last.
- **No frontend re-truncation**: `FileDiff.truncated` is the single backend decision; `buildRows` renders every line sent and never imposes a second cap — the backend already capped at its line limit.
- **Lazy mount latching**: both `DiffFeatureGroup` and `FileDiffRow` use `mounted` signals that latch true once opened; collapsing then re-opening skips the re-mount to preserve scroll and avoid re-highlighting.
- **Binary/unknown status fallback**: `FileDiff.binary` and unknown status strings resolve to explicit typed markers (`dh-binary`, `dfr-status--unknown`), never silently painted as modified or omitted.
- **Style injection is one-time**: `injectDiffStyles()` checks `#diff-styles` and bails if present; rows render many times but styles inject once per session.
