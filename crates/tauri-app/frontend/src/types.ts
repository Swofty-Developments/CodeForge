/**
 * Frontend mirrors of the forge-core IPC types (Rust serde: camelCase structs,
 * snake_case enums) plus the AgentEventPayload from forge-session.
 * KEEP IN SYNC with crates/forge-core/src/*.rs and forge-session/src/payload.rs.
 */

// ── Features ────────────────────────────────────────────────────────────────

export type FileRole = "core" | "support" | "test" | "config" | "unknown";

export interface FeatureFile {
  path: string;
  role: FileRole;
  pinned: boolean;
}

export interface Feature {
  slug: string;
  name: string;
  description: string;
  entryPoints: string[];
  files: FeatureFile[];
  tags: string[];
  pinned: boolean;
  /** User-set graph color (hex like "#74ade8"). Undefined = default per-slug hue. */
  color?: string;
  /** Hierarchy group (slash-delimited path the sidebar nests by). */
  group?: string;
  updatedAt: string; // RFC3339
}

export interface FeaturePatch {
  name?: string;
  description?: string;
  tags?: string[];
}

// ── Timeline ────────────────────────────────────────────────────────────────

export type Actor = "agent" | "human" | "system";

export type EventKind =
  | "session_started"
  | "session_ended"
  | "file_edited"
  | "command_run"
  | "tests_run"
  | "index_started"
  | "index_completed"
  | "feature_pinned"
  | "feature_edited"
  | "doc_updated"
  | "note";

/** Per-kind timeline payloads. The backend writes these exact keys
 *  (forge-daemon/src/hooks.rs, tauri-app command handlers); each `summarize`
 *  branch reads exactly one shape rather than OR-ing guessed key spellings. */
export interface FileEditedPayload {
  tool: string;
  path: string;
}
export interface CommandRunPayload {
  command: string;
  description: string | null;
}
export interface NotePayload {
  text: string;
}
export interface SessionStartedPayload {
  source?: string;
}
export interface IndexStartedPayload {
  force: boolean;
}
/** IndexCompleted is either a success (feature count) or a named failure. */
export type IndexCompletedPayload = { features: number } | { error: string };
export interface FeaturePinnedPayload {
  pinned: boolean;
}
export interface FeatureEditedPayload {
  name: string;
  tags: string[];
}
/** Living-doc auto-refresh outcome — both states named, never conflated. */
export interface DocUpdatedPayload {
  slug: string;
  outcome: "updated" | "failed";
  detail: string;
}

interface TimelineEventBase {
  id: number;
  ts: string; // RFC3339
  sessionId: string | null;
  actor: Actor;
  featureSlugs: string[];
}

/** Discriminated on `kind` so a payload narrows to one concrete shape. */
export type TimelineEvent =
  | (TimelineEventBase & { kind: "file_edited"; payload: FileEditedPayload })
  | (TimelineEventBase & { kind: "command_run"; payload: CommandRunPayload })
  | (TimelineEventBase & { kind: "note"; payload: NotePayload })
  | (TimelineEventBase & { kind: "session_started"; payload: SessionStartedPayload })
  | (TimelineEventBase & { kind: "session_ended"; payload: Record<string, never> })
  | (TimelineEventBase & { kind: "index_started"; payload: IndexStartedPayload })
  | (TimelineEventBase & { kind: "index_completed"; payload: IndexCompletedPayload })
  | (TimelineEventBase & { kind: "feature_pinned"; payload: FeaturePinnedPayload })
  | (TimelineEventBase & { kind: "feature_edited"; payload: FeatureEditedPayload })
  | (TimelineEventBase & { kind: "doc_updated"; payload: DocUpdatedPayload })
  | (TimelineEventBase & { kind: "tests_run"; payload: unknown });

export interface TimelineFilter {
  featureSlug?: string;
  actor?: Actor;
  kinds?: EventKind[];
  since?: string; // RFC3339
  /** Backward-paging cursor: only events with id strictly below this. */
  beforeId?: number;
  limit?: number;
}

// ── Repo / indexing / sessions ──────────────────────────────────────────────

export interface RepoState {
  path: string;
  name: string;
  featuresCount: number;
  indexedAt: string | null;
  daemonPort: number | null;
  branch: string | null;
  /** Basename of the repo family's BASE checkout — shared by the base and all
   *  its worktrees; drives the tab project badges when several projects are open. */
  project: string | null;
}

export interface IndexProgress {
  stage: string; // "decompose" | "docs" | "classify"
  detail: string;
  done: number;
  total: number;
}

// ── Index staleness / versioning (FZ-2) ─────────────────────────────────────

/** Whether a repo's on-disk feature index is current. Four named states, never a
 *  guessed default: `never` = no index yet; `fresh` = hashes + version match;
 *  `stale` = some indexed files' content changed outside CodeForge (e.g. a git
 *  pull); `outdated` = stored indexVersion < the current index format. */
export interface IndexStatus {
  state: "never" | "fresh" | "stale" | "outdated";
  changedFiles: string[];
  indexVersion: number;
  currentVersion: number;
}

// ── Embedded terminals (FZ-3) ───────────────────────────────────────────────

/** A live PTY as reported by list_terminals. */
export interface TerminalInfo {
  id: string;
  cwd: string;
  title: string;
}

/** A terminal tab in the bottom panel. `cwd` is bound at open time to the active
 *  worktree; `exited` flips true when the child process ends (terminal:exit). */
export interface TerminalTab {
  id: string;
  title: string;
  cwd: string;
  exited: boolean;
}

// ── Worktrees (W2 frozen forge-core mirror) ─────────────────────────────────

/** A git worktree of a repo. The base checkout reports itself with isBase=true. */
export interface Worktree {
  path: string;
  name: string;
  branch: string | null;
  isBase: boolean;
  ahead: number;
  behind: number;
  dirty: boolean;
}

/** A local or remote-tracking branch of the repo family (C1, forge-git mirror). */
export interface BranchInfo {
  /** Short name, e.g. "feat/x" (never prefixed with the remote). */
  name: string;
  /** null = local branch; otherwise the remote name, e.g. "origin". */
  remote: string | null;
  /** Checked out at the base HEAD. */
  isHead: boolean;
  /** Absolute worktree path where this branch is checked out, else null. */
  checkedOutAt: string | null;
}

/** Outcome of merging a worktree branch back into the base branch. */
export interface MergeResult {
  merged: boolean;
  conflicts: string[];
  aborted: boolean;
  message: string;
  sourceBranch: string;
  targetBranch: string;
}

/** One open repo context = its own RepoRuntime (daemon / index / timeline),
 *  keyed by state.path. The base checkout is context #0. */
export interface RepoContext {
  state: RepoState;
  isBase: boolean;
}

export type SessionStatus = "starting" | "ready" | "generating" | "error" | "stopped";

/** The four SDK permission modes (W5). "Auto" = bypassPermissions auto-runs everything. */
export type PermissionMode = "default" | "acceptEdits" | "plan" | "bypassPermissions";

export interface SessionInfo {
  id: string;
  threadId: string | null;
  title: string;
  model: string | null;
  status: SessionStatus;
}

export interface PastSession {
  id: string;
  threadId: string;
  claudeSessionId: string | null;
  title: string;
  model: string | null;
  createdAt: string;
  updatedAt: string;
}

export interface StartSessionOpts {
  repoPath: string;
  model?: string;
  permissionMode?: PermissionMode;
  resumeSessionId?: string;
}

/** Wire shape returned by the `daemon_status` IPC command. */
export interface DaemonStatus {
  running: boolean;
  port: number | null;
}

/** Wire shape returned by the `check_claude_cli` IPC command. */
export interface ClaudeCliStatus {
  installed: boolean;
  version: string | null;
  authenticated: boolean;
}

/** Store-side daemon model — three explicit, separately-painted states. An
 *  errored probe is `unknown`, never collapsed into a definitive `offline`.
 *  `null` = not probed yet (no repo open). */
export type DaemonState =
  | { kind: "running"; port: number | null }
  | { kind: "offline" }
  | { kind: "unknown"; error: string };

// ── Diff review ─────────────────────────────────────────────────────────────

export type DiffOrigin = "+" | "-" | " ";

export interface DiffLine {
  origin: DiffOrigin;
  content: string;
  oldNo: number | null;
  newNo: number | null;
}

export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

/** The closed set of git statuses the backend emits. */
export type FileStatus = "modified" | "added" | "deleted" | "renamed" | "untracked";

export interface FileDiff {
  path: string;
  status: FileStatus;
  hunks: DiffHunk[];
  additions: number;
  deletions: number;
  /** Binary file: `hunks` is empty and the review shows "binary, not shown". */
  binary: boolean;
  /** Backend capped the diff at the server line limit (the single, authoritative
   *  truncation). The UI shows an explicit marker; it never re-truncates. */
  truncated: boolean;
}

export interface FeatureDiffGroup {
  slug: string;
  name: string;
  shared: boolean;
  /** True only for the synthetic bucket of files that matched no feature. */
  unmapped: boolean;
  files: FileDiff[];
}

export interface DiffByFeature {
  groups: FeatureDiffGroup[];
}

// ── Agent event stream (the single `agent-event` Tauri channel) ─────────────

export type AgentEventType =
  | "content_delta"
  | "thinking_delta"
  | "turn_started"
  | "turn_completed"
  | "turn_aborted"
  | "approval_required"
  | "ask_user_question"
  | "session_ready"
  | "slash_commands"
  | "session_error"
  | "usage_report"
  | "tool_use_start"
  | "tool_input_delta"
  | "tool_use_end"
  | "tool_result"
  // Added by Agent A (forge-session). Speculative until its contractNotes land:
  // both are assumed to carry human-readable detail in `message` (see reducer).
  | "session_resume_failed"
  | "session_persistence_degraded";

/** Flat union payload; fields present depend on eventType. Demux by sessionId. */
export interface AgentEventPayload {
  sessionId: string;
  threadId: string;
  eventType: AgentEventType;
  text?: string;
  turnId?: string;
  reason?: string;
  message?: string;
  requestId?: string;
  description?: string;
  inputTokens?: number;
  outputTokens?: number;
  costUsd?: number;
  model?: string;
  toolId?: string;
  toolName?: string;
  inputJson?: string;
  toolOutput?: string;
  isError?: boolean;
  cacheReadTokens?: number;
  cacheWriteTokens?: number;
  /** Structured questions array for ask_user_question events. */
  questions?: unknown;
  commands?: string[];
}

// ── UI-side shapes ──────────────────────────────────────────────────────────

export type ToolStatus = "generating" | "running" | "completed" | "error";

export interface ContentBlock {
  type: "text" | "tool_use" | "thinking";
  content: string;
  toolId?: string;
  toolName?: string;
  toolInput?: string;
  toolOutput?: string;
  toolStatus?: ToolStatus;
  toolError?: boolean;
}

export type RunState = "idle" | "starting" | "ready" | "generating" | "interrupting" | "error";

export interface MessageMeta {
  model?: string;
  inputTokens?: number;
  outputTokens?: number;
  costUsd?: number;
}

/** One chat message. Id conventions: `opt-*` optimistic user send, plain uuid
 *  live streaming assistant message, `done-*` finalized. */
export interface SessionMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  blocks: ContentBlock[];
  meta?: MessageMeta;
  /** System messages only: severity that tints the centered pill. */
  level?: "info" | "warn" | "error";
}

/** Token/cost totals accumulated across a session's turns. */
export interface SessionUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  costUsd: number;
}

export interface ErrorToast {
  id: number;
  message: string;
  /** Tints the toast; defaults to error (red). */
  kind?: "error" | "success";
}

/** A session as rendered in the right-hand session pane. `messages` is the
 *  single source of truth; the stream renders straight from it. */
export interface SessionUi {
  info: SessionInfo;
  runState: RunState;
  messages: SessionMessage[];
  pendingApproval: { requestId: string; description: string } | null;
  pendingQuestion: { requestId: string; questions: unknown } | null;
  slashCommands: string[];
  claudeSessionId: string | null;
  usage: SessionUsage;
  /** The repo context (worktree/base path) this session belongs to. */
  contextPath: string;
  /** Live permission mode; switchable via set_session_mode. */
  permissionMode: PermissionMode;
}

export type ActiveView = "welcome" | "feature" | "timeline" | "diff" | "graph";
