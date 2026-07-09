/**
 * Frontend mirrors of the forge-core IPC types (Rust serde: camelCase structs,
 * snake_case enums) plus the AgentEventPayload from forge-session.
 * KEEP IN SYNC with crates/forge-core/src/*.rs and forge-session/src/payload.rs.
 */

// ── Features ────────────────────────────────────────────────────────────────

export type FileRole = "core" | "support" | "test" | "config";

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
  confidence: number;
  pinned: boolean;
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
  | "note";

export interface TimelineEvent {
  id: number;
  ts: string; // RFC3339
  sessionId: string | null;
  actor: Actor;
  kind: EventKind;
  featureSlugs: string[];
  payload: unknown;
}

export interface TimelineFilter {
  featureSlug?: string;
  actor?: Actor;
  kinds?: EventKind[];
  since?: string; // RFC3339
  limit?: number;
}

// ── Repo / indexing / sessions ──────────────────────────────────────────────

export interface RepoState {
  path: string;
  name: string;
  featuresCount: number;
  indexedAt: string | null;
  daemonPort: number | null;
}

export interface IndexProgress {
  stage: string; // "decompose" | "docs" | "classify"
  detail: string;
  done: number;
  total: number;
}

export type SessionStatus = "starting" | "ready" | "generating" | "error" | "stopped";

export interface SessionInfo {
  id: string;
  threadId: string | null;
  title: string;
  model: string | null;
  status: SessionStatus;
}

export interface StartSessionOpts {
  repoPath: string;
  model?: string;
  permissionMode?: string; // "default" | "acceptEdits" | "plan" | "bypassPermissions"
  resumeSessionId?: string;
}

export interface DaemonStatus {
  running: boolean;
  port: number | null;
}

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

export interface FileDiff {
  path: string;
  status: string; // "modified" | "added" | "deleted" | "renamed" | "untracked"
  hunks: DiffHunk[];
  additions: number;
  deletions: number;
}

export interface FeatureDiffGroup {
  slug: string;
  name: string;
  shared: boolean;
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
  | "session_ready"
  | "slash_commands"
  | "session_error"
  | "usage_report"
  | "tool_use_start"
  | "tool_input_delta"
  | "tool_use_end"
  | "tool_result";

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
}

/** A session as rendered in the right-hand session pane. */
export interface SessionUi {
  info: SessionInfo;
  runState: RunState;
  /** Flat mirror of every streamed agent block; block objects are shared with `messages`. */
  blocks: ContentBlock[];
  messages: SessionMessage[];
  pendingApproval: { requestId: string; description: string } | null;
  slashCommands: string[];
  claudeSessionId: string | null;
  usage: SessionUsage;
}

export type ActiveView = "welcome" | "feature" | "timeline" | "diff";
