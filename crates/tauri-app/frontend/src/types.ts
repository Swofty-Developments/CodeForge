export interface Project {
  id: string;
  name: string;
  path: string;
  color: string | null;
  collapsed: boolean;
  threads: Thread[];
}

export interface Thread {
  id: string;
  project_id: string;
  title: string;
  color: string | null;
}

export interface MessageMeta {
  model?: string;
  inputTokens?: number;
  outputTokens?: number;
  durationMs?: number;
  costUsd?: number;
}

/**
 * Kind of a system message — used to style the pill in the chat scroll.
 *
 *   - `info`   (default, grey)   neutral notifications: pushes, links, context
 *   - `warn`   (amber)           recoverable issues: aborted, diverged, reverted
 *   - `error`  (red)             failures: push failed, generation errored
 *   - `review` (blue)            incoming PR review comments (with author chip)
 *
 * System messages in the chat scroll represent **events** (things that happened
 * at a point in time). Persistent **state** lives in `LifecycleBanner`, not here.
 */
export type SystemMessageKind = "info" | "warn" | "error" | "review";

export interface ChatMessage {
  id: string;
  thread_id: string;
  role: "user" | "assistant" | "system";
  content: string;
  blocks?: ContentBlock[];
  meta?: MessageMeta;
  /** Present iff `role === "system"`. Defaults to "info" when absent. */
  system_kind?: SystemMessageKind;
}

export interface AgentEventPayload {
  session_id: string;
  thread_id: string;
  event_type: string;
  text?: string;
  turn_id?: string;
  reason?: string;
  message?: string;
  request_id?: string;
  description?: string;
  input_tokens?: number;
  output_tokens?: number;
  cost_usd?: number;
  model?: string;
  tool_id?: string;
  tool_name?: string;
  input_json?: string;
  tool_output?: string;
  is_error?: boolean;
  cache_read_tokens?: number;
  cache_write_tokens?: number;
}

export interface ThreadTokenUsage {
  inputTokens: number;
  outputTokens: number;
  cacheReadTokens: number;
  cacheWriteTokens: number;
  totalTokens: number;
  model?: string;
}

export type ToolStatus = "generating" | "running" | "completed" | "error";

export interface ContentBlock {
  type: "text" | "tool_use" | "thinking";
  content: string;
  tool_id?: string;
  tool_name?: string;
  tool_input?: string;
  tool_output?: string;
  tool_status?: ToolStatus;
  tool_error?: boolean;
}

export interface Attachment {
  id: string;
  type: "file" | "extraction" | "image";
  name: string;
  content: string;
  language?: string;
}

/**
 * The state of the **agent process** for a thread. This is orthogonal to the
 * PR/worktree lifecycle — it tracks whether the AI is currently running, idle,
 * interrupting, etc. It never contains values like "merged" or "closed";
 * those belong to `LifecycleState`.
 */
export type RunState = "idle" | "starting" | "ready" | "generating" | "interrupting" | "error";

/** @deprecated Use `RunState`. Kept as an alias during the migration. */
export type SessionStatus = RunState;

/** Snapshot of a pull request used by `LifecycleState` variants. */
export interface PrSnapshot {
  number: number;
  url: string;
  state: string; // "open" | "closed" | "merged"
}

/**
 * The **lifecycle state** of a thread — its relationship to its worktree and
 * any linked pull request. Orthogonal to `RunState`. The backend's
 * `get_pr_status` reconciler computes this; the frontend stores it verbatim
 * in `store.lifecycleStates` and surfaces it via the `LifecycleBanner`.
 *
 * `kind: "working"` is the default (worktree active, no PR, or no worktree yet).
 * Variants tagged as "locked" below disable the Composer.
 */
export type LifecycleState =
  | { kind: "working" }
  | {
      kind: "pr_open";
      pr: PrSnapshot;
      ci: string;             // "success" | "failure" | "pending" | "none" | "unknown"
      review: string;         // "approved" | "changes_requested" | "commented" | "none"
      unread_comments: number;
    }
  | {
      kind: "pr_open_diverged";
      pr: PrSnapshot;
      ahead: number;
      behind: number;
    }
  | { kind: "pr_closed"; pr: PrSnapshot }                       // locked
  | { kind: "pr_merged"; pr: PrSnapshot; merge_commit: string } // locked
  | { kind: "pr_reverted"; pr: PrSnapshot }                     // editable — was merged, then reverted
  | { kind: "worktree_missing"; branch: string; path: string }
  | { kind: "worktree_orphaned"; branch: string; path: string };

/** True if the lifecycle state should disable the composer/input. */
export function isLifecycleLocked(state: LifecycleState | undefined): boolean {
  if (!state) return false;
  return state.kind === "pr_merged" || state.kind === "pr_closed";
}

export const THREAD_COLORS = [
  { hex: "#e65961", label: "Red" },
  { hex: "#e6b84d", label: "Amber" },
  { hex: "#59c78c", label: "Green" },
  { hex: "#66b8e0", label: "Sky" },
  { hex: "#6680f2", label: "Blue" },
  { hex: "#c084fc", label: "Purple" },
  { hex: "#f472b6", label: "Pink" },
  { hex: "#fb923c", label: "Orange" },
];
