# Domain Models

**Purpose**  
`forge-core` is the zero-dependency domain crate — no IO, no business logic. Every struct crosses the Tauri IPC boundary (Rust ⇄ SolidJS), so all types are `#[serde(rename_all = "camelCase")]`, enums serialize as `snake_case` strings, and the shapes are mirrored 1:1 in `frontend/src/types.ts`.

**How it works**  
- **Feature** — the primary unit CodeForge organizes code by. Slug (stable id), name, description, entry points, many-to-many files (one file can belong to N features), tags, pinned flag, optional user-set color (hex), and optional hierarchy group (slash-delimited path for sidebar nesting, e.g. `"backend/forge-index"`). `FeaturePatch` is the partial edit contract for `update_feature`.
- **FileRole** — enum for a file's role within a feature: `Core | Support | Test | Config | Unknown`. `Unknown` is a distinct, visible state for unclassified files (never conflated with `Support`).
- **Timeline** — append-only immutable event stream. `TimelineEvent` = SQLite rowid + timestamp + session id + actor (`Agent | Human | System`) + kind (`FileEdited | CommandRun | TestsRun | IndexCompleted | DocUpdated | Note | …`) + feature slugs (may be empty until classified) + kind-specific JSON payload. `TimelineFilter` queries with AND-ed optional fields; `before_id` is the backward-paging cursor (rowids are append-only, so id order = time order).
- **Session** — `SessionInfo` (id + thread_id + title + model + status enum) and `StartSessionOpts` (repo path + model + permission mode + resume session id). `SessionStatus` = `Starting | Ready | Generating | Error | Stopped`.
- **Diff** — `DiffByFeature` groups changed files by feature for the review view. Files in N features appear under each with `shared: true`. Files matching no feature fall under a synthetic "Unmapped" group (`unmapped: true`). `FileDiff` = path + status string (`"modified" | "added" | "deleted" | "renamed" | "untracked"`) + hunks + adds/dels counts + `binary` flag + `truncated` flag (server-side line limit hit). `DiffLine.origin` is `'+'` (add), `'-'` (remove), `' '` (context).
- **Worktree** — one git worktree (the base checkout is `is_base: true`). Each opens as its own repo context (daemon / index / timeline) and tab. Tracks path, name (branch or directory on detached HEAD), ahead/behind counts, and dirty flag. `MergeResult` = merged flag + conflicts list + aborted flag + message + source/target branch names.
- **RepoState** — returned by `open_repo`. Path, name, features count, indexed_at (None until first cold-start index completes), daemon port (None on failure), branch (None on detached HEAD), project identity (basename of the BASE checkout, shared by base + all worktrees, for tab badges when multiple projects are open).

**Key files**  
`lib.rs` (exports), `feature.rs` (Feature + FeatureFile + FileRole + FeaturePatch), `timeline.rs` (TimelineEvent + Actor + EventKind + TimelineFilter), `session.rs` (SessionInfo + SessionStatus + StartSessionOpts + RepoState + IndexProgress), `diff.rs` (DiffByFeature + FeatureDiffGroup + FileDiff + DiffHunk + DiffLine), `worktree.rs` (Worktree + MergeResult), `error.rs` (Error + Result).

**Invariants & gotchas**  
- Every struct has a roundtrip test verifying camelCase field names (e.g. `entryPoints`, `updatedAt`) and snake_case enum variants (e.g. `"agent"`, `"file_edited"`).
- Optional fields use `#[serde(default, skip_serializing_if = "Option::is_none")]` so they serialize to `{}` when None, not `{"field":null}`.
- The IPC contract is a compile-time lock: Rust-side changes without mirroring in `types.ts` cause runtime deserialization failures. Grep `frontend/src/types.ts` before renaming fields.
- `Unknown` FileRole is never a fallback — it's an explicit signal that the indexer saw the file but didn't classify it. Don't conflate with `Support` when filtering.
- Approval responses for `AskUserQuestion` events MUST carry `answers: HashMap<String, String>` (question text → selected option label) on "allow" decisions, or the Agent SDK resolves `canUseTool` with `updatedInput: { questions, answers: {} }` and the tool returns "The user did not answer the questions." This plumbing was fixed end-to-end: frontend `QuestionCard` → `appStore.approveRequest(…, answers)` → Tauri `approve_session(answers)` → `manager.approve` → `claude.rs respond_to_approval` → sidecar `approval_response{answers}` → SDK.
