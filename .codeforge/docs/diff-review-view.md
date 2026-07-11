# Diff Review View

## Purpose

Feature-grouped diff viewer that organizes staged, unstaged, and untracked changes by the features they belong to, replacing the traditional file-by-file diff view with a feature-centric accordion UI.

## How it works

- **Feature grouping**: backend groups changed files by their mapped features via `which_features(path)`. Files in multiple features appear under each group with a `shared:true` badge. Unmapped files land in a synthetic `unmapped:true` group, visually muted and sorted last.
- **Three-level accordion**: feature groups expand to file rows, file rows expand to hunk views. Each level uses grid `0fr→1fr` transitions and latched mounting—unopened accordions never render their children, so large diffs don't block paint.
- **Diff rendering**: hunks are flattened into header + line rows, syntax-highlighted per extension via `highlight.js`. Old/new line numbers, `+`/`−`/` ` prefixes, and tinted backgrounds surface in a two-column gutter.
- **Backend truncation contract**: `FileDiff.truncated` is the single truncation flag; `FileDiff.binary` marks binaries. The frontend renders every line sent without re-capping or silent omissions.
- **Sticky header with refresh**: total +/− counts across unique files (shared files counted once), manual refresh button that calls `appStore.refreshDiff()` to re-fetch from daemon.
- **Default-open heuristic**: first feature group and its first 3 files open on load to show diffs at a glance without interaction.

## Key files

- **DiffReview.tsx** — view shell with sticky header, refresh control, and total counts. Maps feature groups to `<DiffFeatureGroup>` accordions.
- **DiffFeatureGroup.tsx** — one feature accordion: name, shared badge, per-group +/− counts, file count. Grid `0fr→1fr` body transition with latched child mount.
- **FileDiffRow.tsx** — one file inside a group: status chip (M/A/D/R/U), mono path with dir/base split, +/− counts, collapsible hunk body.
- **DiffHunks.tsx** — renders a file's hunks with line numbers, syntax highlighting, and explicit binary/truncated markers. No client-side re-capping.
- **diff-utils.ts** — pure helpers: group ordering (unmapped last), unique-file count logic, status→glyph/tint mapping, extension→hljs language, hunk→row flattening. Truncation is backend-only.

## Invariants & gotchas

- **Shared files are duplicated, not linked**: a file in three features renders three times with independent accordion state. Total counts must dedupe via `Map<path, FileDiff>` to avoid triple-counting.
- **Unmapped group is a boolean flag, not a slug sniff**: `isUnmapped(group)` checks `group.unmapped`, never string matching on `group.name`. The backend owns the unmapped classification (CONTRACT-4).
- **No frontend line limits**: the backend caps at its server line limit and sets `truncated:true`. The frontend must render every line sent—adding a client cap would silently hide the truncation marker below the fold.
- **Latched mounting prevents re-mount thrash**: `mounted()` latches true and never resets. Closing an accordion hides it via grid `1fr→0fr` but leaves the DOM intact so syntax-highlighting work isn't repeated.
- **Status chips exhaustively map five backend states**: modified/added/deleted/renamed/untracked. An unknown status renders `?` with a neutral tint, never silently defaulting to "modified".
