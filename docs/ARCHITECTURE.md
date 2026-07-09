# FeatureForge — Architecture Spec (v1)

**One-liner:** An IDE built off of Claude Code. You open a repository and see *features*, not files. A local
daemon keeps a feature index + append-only timeline, fed by Claude Code hooks and consumed by agents via MCP.
Modeled on CodeForge (same author) for app architecture + design language; styled per Zed One Dark.

Repo: `~/DeveloperPersonal/FeatureForge`. All product decisions below are FINAL unless a brief contradicts
a technical assumption (versions, protocol details) — in that case the brief wins.

## Product loop (the demo that must work)

1. Open a repo → FeatureForge installs its integration kit + runs cold-start indexing (headless `claude -p`).
2. Sidebar shows the **feature tree** (not files). Click a feature → docs, key files, recent changes.
3. Ask for work in the embedded **Claude Code pane** (real `claude` process, stream-json).
4. Hooks (PostToolUse/Stop) fire → daemon classifies changes → **timeline** updates live.
5. **Diff review grouped by feature**, not by file.
6. Feature docs/changelog stay current; agents query the index + timeline via **MCP**.

## Workspace layout

```
FeatureForge/
├── Cargo.toml                      # workspace, resolver=2, same dep style as CodeForge
├── crates/
│   ├── forge-core/                 # shared domain types + errors (serde). No IO.
│   ├── forge-timeline/             # append-only event log, rusqlite (WAL)
│   ├── forge-index/                # feature model, pins, classifier, cold-start indexer
│   ├── forge-daemon/               # axum HTTP (hooks receiver + query API) + MCP stdio bin (forge-mcp) + kit installer
│   ├── forge-session/              # Claude Code session lifecycle via Node agent-sidecar (stream-json)
│   ├── forge-git/                  # diff extraction, diff-by-feature grouping
│   └── tauri-app/                  # Tauri 2.10 shell: commands, state, events
│       ├── agent-sidecar/          # Node sidecar wrapping @anthropic-ai/claude-agent-sdk (CodeForge protocol)
│       └── frontend/               # Vite + SolidJS 1.9 + TS, CSS-custom-property tokens
└── docs/ARCHITECTURE.md            # this file
```

Crate deps: core ← {timeline, index, git, session, daemon} ← tauri-app. No cycles. Library crates never
depend on tauri.

## Domain model (forge-core; exact serde types are the cross-agent contract)

```rust
pub struct Feature {
    pub slug: String,               // kebab-case id, stable
    pub name: String,
    pub description: String,        // 1-3 sentences
    pub entry_points: Vec<PathBuf>, // the "start reading here" files
    pub files: Vec<FeatureFile>,    // many-to-many: files participate in multiple features
    pub tags: Vec<String>,
    pub confidence: f32,            // indexer confidence 0..1
    pub pinned: bool,               // human-pinned features survive re-index verbatim
    pub updated_at: DateTime<Utc>,
}
pub struct FeatureFile { pub path: PathBuf, pub role: FileRole, pub pinned: bool }
pub enum FileRole { Core, Support, Test, Config }

pub struct TimelineEvent {
    pub id: i64,                    // rowid, append-only, never mutated
    pub ts: DateTime<Utc>,          // locked at write time
    pub session_id: Option<String>, // claude session uuid if agent-originated
    pub actor: Actor,               // Agent | Human | System
    pub kind: EventKind,
    pub feature_slugs: Vec<String>, // classification result (may be empty until classified)
    pub payload: serde_json::Value, // kind-specific
}
pub enum EventKind { SessionStarted, SessionEnded, FileEdited, CommandRun, TestsRun,
    IndexStarted, IndexCompleted, FeaturePinned, FeatureEdited, Note }
```

Index on disk (per repo): `.featureforge/features.json` (committable, human-readable),
`.featureforge/docs/<slug>.md` (per-feature living doc), `.featureforge/runtime/` (gitignored):
`timeline.db`, `daemon.json` `{port, pid, started_at}`.

## Daemon (in-process tokio task inside the Tauri app, but reachable externally)

axum on `127.0.0.1:0` (ephemeral) → writes actual port to `.featureforge/runtime/daemon.json` per open repo.
Routes:
- `POST /hooks/event` — receives Claude Code hook payloads (hook JSON piped by installed hook script).
  Fire-and-forget from the hook's perspective. Daemon: parse → append raw TimelineEvent → async classify
  (path→feature via index; unmatched paths queue re-index) → broadcast to frontend via tauri event.
- `GET /api/features`, `GET /api/features/:slug`, `GET /api/timeline?feature=&since=`, `GET /api/health`.
- MCP: `forge-mcp` stdio binary (bin target in forge-daemon). JSON-RPC 2.0 over stdio, hand-rolled
  (initialize, tools/list, tools/call). Reads daemon.json (walks up from cwd) and proxies to HTTP.
  Tools: `list_features`, `get_feature(slug)`, `which_features(path)`, `feature_timeline(slug, limit)`,
  `record_note(text, feature_slugs)`.

**Integration kit** (installed into a repo on open; idempotent; never clobbers user content):
- `.claude/settings.json` — merge in hooks: PostToolUse(Edit|Write|MultiEdit|NotebookEdit → forward to
  daemon via `.featureforge/hooks/forward.sh`), Stop, SessionStart. Script reads port from daemon.json,
  `curl --max-time 2 ... || true` (no-op when app closed).
- `.mcp.json` — merge `featureforge` stdio server → `~/.featureforge/bin/forge-mcp` (binary copied there
  on app start).
- `CLAUDE.md` — append a fenced, marker-delimited section (`<!-- featureforge:start/end -->`) telling the
  agent to consult the feature index via MCP before exploring, and to `record_note` decisions.
- `.featureforge/` scaffold + `.gitignore` for runtime/.

## Indexing (forge-index)

Cold start: spawn `claude -p` (headless, cwd=repo, `--output-format json`, sonnet) with a decomposition
prompt → strict JSON array of features (slug/name/description/entry_points/files/tags/confidence).
Validate + clamp paths to files that exist. Then per-feature doc pass (parallel, capped) writing
`docs/<slug>.md`. Progress streamed to frontend (`index:progress` events). Re-index: preserve pinned
features/files verbatim; merge by slug. Incremental: classifier maps edited paths→features by exact/dir
match; unknown paths accumulate and a cheap headless pass assigns them.

## Sessions (forge-session + agent-sidecar) — per claude-integration brief, follow it exactly

CodeForge's proven design, adopted wholesale: one Node sidecar process per session wrapping
`@anthropic-ai/claude-agent-sdk` `query()`, custom NDJSON over stdin/stdout.
In: `query{prompt,cwd,model,permissionMode,sessionId}`, `approval_response`, `abort`. Out: `ready`,
`session_ready`, `turn_started/completed`, `text_delta`, `thinking_delta`, `tool_use_start/input`,
`tool_result`, `approval_request`, `usage`, `error`. Snapshot-vs-delta dedup by prior text length;
`turn_completed` guaranteed via finally; resume fallback chain (resume → continue → fresh).
Rust: resolve node via login-shell env (`$SHELL -l -i -c 'env -0'`, TERM=dumb, OnceLock cache, merge over
process env), spawn kill_on_drop, 4 tokio tasks (stdin writer / stdout parser / stderr collector /
first-query augmenter). One Tauri event channel `agent-event` with flat payload {event_type + optional
fields}, demuxed frontend-side by thread_id. DB writes from async via spawn_blocking. cwd = repo root so
hooks/MCP/CLAUDE.md apply to embedded sessions.

## Diff review (forge-git)

`git status --porcelain=v2` + `git diff` (HEAD) parsed → hunks per file → group files by feature via index
(files in N features appear under each, marked "shared") → frontend renders per-feature accordion with
syntax-highlighted hunks. Ungrouped files fall under "Unmapped".

## Tauri IPC surface (commands)

`open_repo(path) -> RepoState`, `close_repo`, `get_features`, `get_feature(slug)`,
`reindex_repo(force: bool)`, `pin_feature(slug, pinned)`, `update_feature(slug, patch)`,
`get_timeline(filter) -> Vec<TimelineEvent>`, `get_diff_by_feature`, `start_session(opts) -> SessionInfo`,
`send_session_input(id, text)`, `stop_session(id)`, `list_sessions`, `daemon_status`.
Events: `timeline:event`, `index:progress`, `session:{id}:stream`, `session:{id}:status`, `repo:changed`.

## Frontend (SolidJS 1.9 + Vite, matching CodeForge exactly)

Single global store via `createRoot(createStore)` in `stores/app-store.ts`; typed invoke wrappers in
`ipc.ts`; one global `agent-event` listener demuxing by thread/session id. Vite :5173 strictPort,
beforeDevCommand empty (start vite separately in dev).

Layout (CodeForge shell): `#app` flex row → resizable sidebar (260px, 180-500 clamp, bg-surface,
border-right) = **feature tree** (activity badges, pin icons, index-status footer) · main panel column =
tab bar / breadcrumb 24px / active view / status bar 24px with animated activity line · right side pane =
Claude Code sessions (spring-physics divider, snap points). Views: Feature detail / Timeline / Diff review
/ Welcome. Cmd+K palette (520px, 22vh top, blur(8px) overlay).

**Styling: CodeForge "Obsidian Forge" tokens are the base theme** (it's the user's own design): bg stack
#0f1012→#222228, text #f0eef8/#b8b6c8/#807e92, primary #6b7cff, green #4cd694, red #f25f67, amber #f0b840,
sky #58c4f0, purple #b47aff, translucent-white borders, radii 6/10/14/20; DM Sans body + JetBrains Mono
metadata/code (bundle via @fontsource, NOT Google CDN); signature motion cubic-bezier(0.16,1,0.3,1)
entrances 120-220ms + translate/scale, grid-rows 0fr→1fr collapses, pulse dots, streaming line fade-ups,
prefers-reduced-motion coverage. Universal semantic-tint formula rgba(accent,.04-.15) bg + rgba(accent,
.12-.4) border + solid accent text. Zed discipline layered on: 1px borders separate regions (never
shadows at rest), content column darkest, 5-6px overlay scrollbars, tight component metrics. Ship a
"Zed One Dark" alternative theme JSON from the zed-style brief. Centralize keyframes in global.css
(fix CodeForge's known duplication/hardcoded-palette flaws).

Feature detail view: description, entry points (open-in-editor via `open` deep link), file list with roles
+ shared-feature chips, living doc (rendered md, editable → FeatureEdited event + pin), recent timeline
slice, "Ask Claude about this feature" (prefills session input).

Timeline view: virtualized reverse-chron list, lane color per session, filter chips (feature, actor, kind),
live via `timeline:event`. Immutable — no edit affordances.

## Non-goals for v1 (explicit)

Codex support · parallel worktree sessions · embedded browser · file editing of any kind (read-only code
peek + "open in editor" only) · cloud anything · Windows/Linux polish (macOS first).

## Persistence (per app-architecture brief)

App-level DB `~/.featureforge/featureforge.db`: rusqlite 0.31 `features=["bundled"]`, single connection in
`Arc<std::sync::Mutex<Database>>` managed state, PRAGMA journal_mode=WAL + foreign_keys=ON +
synchronous=NORMAL + busy_timeout=5000, hand-written idempotent migrations (column_exists PRAGMA guard).
Tables: repos, threads, messages, sessions (claude_session_id for resume), settings KV, usage_logs.
Per-repo timeline DB `.featureforge/runtime/timeline.db` (owned by forge-timeline, same pragmas).
Tauri 2.10.x + plugins: dialog, shell, process. Capabilities: core:default, dialog:default, shell:default,
core:event:default. Window 1200x800 min 800x500. All commands `Result<T, String>`; IDs cross IPC as
Strings (typed newtype parse Rust-side).

## Quality bar

Workspace `cargo build` + `cargo clippy -- -D warnings` green; `tsc --noEmit` green; app boots to welcome,
opens a real repo, indexes it, streams a session, shows live timeline + feature-grouped diff. Rust style:
thiserror per crate, no unwraps on IO paths, tracing instead of println.
