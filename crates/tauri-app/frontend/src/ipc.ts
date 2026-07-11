/**
 * Typed wrapper for EVERY Tauri invoke + listen helper for each event channel.
 * Components never call invoke() directly. Tauri auto-converts camelCase JS args
 * to snake_case Rust params (repoPath -> repo_path).
 *
 * Registered commands (crates/tauri-app/src/main.rs — frozen contract):
 *   open_repo, init_repo, close_repo, reindex_repo, daemon_status, index_status,
 *   get_features, get_feature, pin_feature, update_feature,
 *   get_timeline, get_diff_by_feature,
 *   start_session, send_session_input, approve_session, stop_session, list_sessions,
 *   open_terminal, write_terminal, resize_terminal, close_terminal, list_terminals
 *
 * Event channels:
 *   "agent-event"     AgentEventPayload (all session streaming, demux by sessionId)
 *   "timeline:event"  { repoPath, event: TimelineEvent } (FZ-5: per-context)
 *   "index:progress"  IndexProgress
 *   "index:status"    { repoPath, status: IndexStatus } (FZ-2, emitted on open_repo)
 *   "repo:changed"    RepoState
 *   "terminal:data"   { id, data } — data is BASE64-encoded PTY output (FZ-3)
 *   "terminal:exit"   { id, code } — child process ended
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentEventPayload,
  BranchInfo,
  ClaudeCliStatus,
  DaemonStatus,
  DiffByFeature,
  Feature,
  FeaturePatch,
  IndexProgress,
  IndexStatus,
  MergeResult,
  PermissionMode,
  RepoState,
  SessionInfo,
  StartSessionOpts,
  TerminalInfo,
  TimelineEvent,
  TimelineFilter,
  Worktree,
} from "./types";

// ── Repo ────────────────────────────────────────────────────────────────────

export function openRepo(path: string): Promise<RepoState> {
  return invoke("open_repo", { path });
}

/** `git init -b main` in a plain folder, then open it exactly like open_repo.
 *  Offered when open_repo rejects with the "not_a_git_repo:" prefix. */
export function initRepo(path: string): Promise<RepoState> {
  return invoke("init_repo", { path });
}

export function closeRepo(path: string): Promise<void> {
  return invoke("close_repo", { path });
}

export function reindexRepo(repoPath: string, force: boolean): Promise<void> {
  return invoke("reindex_repo", { repoPath, force });
}

export function daemonStatus(repoPath: string): Promise<DaemonStatus> {
  return invoke("daemon_status", { repoPath });
}

/** FZ-2: whether the repo's on-disk feature index is fresh / stale / outdated. */
export function indexStatus(repoPath: string): Promise<IndexStatus> {
  return invoke("index_status", { repoPath });
}

// ── Features ────────────────────────────────────────────────────────────────

export function getFeatures(repoPath: string): Promise<Feature[]> {
  return invoke("get_features", { repoPath });
}

export function getFeature(repoPath: string, slug: string): Promise<Feature> {
  return invoke("get_feature", { repoPath, slug });
}

export function getFeatureDoc(repoPath: string, slug: string): Promise<string | null> {
  return invoke("get_feature_doc", { repoPath, slug });
}

export function pinFeature(repoPath: string, slug: string, pinned: boolean): Promise<void> {
  return invoke("pin_feature", { repoPath, slug, pinned });
}

export function updateFeature(repoPath: string, slug: string, patch: FeaturePatch): Promise<Feature> {
  return invoke("update_feature", { repoPath, slug, patch });
}

/** null/undefined color clears; implies pin like update_feature. */
export function setFeatureColor(repoPath: string, slug: string, color?: string): Promise<Feature> {
  return invoke("set_feature_color", { repoPath, slug, color: color ?? null });
}

// ── Worktrees (W3 frozen commands) ──────────────────────────────────────────

export function listWorktrees(repoPath: string): Promise<Worktree[]> {
  return invoke("list_worktrees", { repoPath });
}

export function createWorktree(repoPath: string, name: string, baseRef?: string): Promise<Worktree> {
  return invoke("create_worktree", { repoPath, name, baseRef: baseRef ?? null });
}

export function removeWorktree(repoPath: string, worktreePath: string, force: boolean): Promise<void> {
  return invoke("remove_worktree", { repoPath, worktreePath, force });
}

export function mergeWorktree(baseRepoPath: string, worktreePath: string): Promise<MergeResult> {
  return invoke("merge_worktree", { baseRepoPath, worktreePath });
}

// ── Branches (C2 frozen commands) ───────────────────────────────────────────

/** Local + remote-tracking refs of the repo family (base derived internally). */
export function listBranches(repoPath: string): Promise<BranchInfo[]> {
  return invoke("list_branches", { repoPath });
}

/** git fetch --all --prune. */
export function fetchRemotes(repoPath: string): Promise<void> {
  return invoke("fetch_remotes", { repoPath });
}

/** remote == null: check out the EXISTING local branch into a new worktree
 *  (already-checked-out is a NAMED error). remote set: create a local branch
 *  tracking <remote>/<branch>; an existing local branch of that name is a NAMED
 *  error. The backend runs create_worktree's post-create steps + opens the context. */
export function addWorktreeForBranch(repoPath: string, branch: string, remote?: string): Promise<Worktree> {
  return invoke("add_worktree_for_branch", { repoPath, branch, remote: remote ?? null });
}

// ── Timeline & diff ─────────────────────────────────────────────────────────

export function getTimeline(repoPath: string, filter: TimelineFilter): Promise<TimelineEvent[]> {
  return invoke("get_timeline", { repoPath, filter });
}

export function getDiffByFeature(repoPath: string): Promise<DiffByFeature> {
  return invoke("get_diff_by_feature", { repoPath });
}

// ── Sessions ────────────────────────────────────────────────────────────────

export function startSession(opts: StartSessionOpts): Promise<SessionInfo> {
  return invoke("start_session", { opts });
}

export function sendSessionInput(id: string, text: string): Promise<void> {
  return invoke("send_session_input", { id, text });
}

export function approveSession(id: string, requestId: string, approve: boolean): Promise<void> {
  return invoke("approve_session", { id, requestId, approve });
}

export function stopSession(id: string): Promise<void> {
  return invoke("stop_session", { id });
}

export function setSessionMode(sessionId: string, mode: PermissionMode): Promise<void> {
  return invoke("set_session_mode", { sessionId, mode });
}

export function listSessions(): Promise<SessionInfo[]> {
  return invoke("list_sessions");
}

/** Save a pasted clipboard image to a temp file; returns the path to attach. */
export function savePastedImage(dataBase64: string, mime: string): Promise<string> {
  return invoke("save_pasted_image", { dataBase64, mime });
}

/** Claude CLI health check: locate (PATH + well-known installs) and run --version. */
export function claudeCliStatus(): Promise<ClaudeCliStatus> {
  return invoke("claude_cli_status");
}

// ── Terminals (FZ-3) ─────────────────────────────────────────────────────────

/** Spawn a PTY rooted at `cwd` (the active worktree). Returns its terminal id. */
export function openTerminal(cwd: string, shell?: string): Promise<string> {
  return invoke("open_terminal", { cwd, shell: shell ?? null });
}

/** Forward raw keystrokes (a UTF-8 string from xterm's onData) to the PTY. */
export function writeTerminal(id: string, data: string): Promise<void> {
  return invoke("write_terminal", { id, data });
}

export function resizeTerminal(id: string, cols: number, rows: number): Promise<void> {
  return invoke("resize_terminal", { id, cols, rows });
}

export function closeTerminal(id: string): Promise<void> {
  return invoke("close_terminal", { id });
}

export function listTerminals(): Promise<TerminalInfo[]> {
  return invoke("list_terminals");
}

// ── Event listeners ─────────────────────────────────────────────────────────

export function listenAgentEvent(handler: (payload: AgentEventPayload) => void): Promise<UnlistenFn> {
  return listen<AgentEventPayload>("agent-event", (e) => handler(e.payload));
}

/** FZ-5: the payload now carries the emitting context's repoPath so the frontend
 *  can append only LIVE events belonging to the active context. */
export function listenTimelineEvent(
  handler: (repoPath: string, event: TimelineEvent) => void,
): Promise<UnlistenFn> {
  return listen<{ repoPath: string; event: TimelineEvent }>("timeline:event", (e) =>
    handler(e.payload.repoPath, e.payload.event),
  );
}

export function listenIndexProgress(handler: (progress: IndexProgress) => void): Promise<UnlistenFn> {
  return listen<IndexProgress>("index:progress", (e) => handler(e.payload));
}

/** FZ-2: emitted on open_repo (and on demand) so the UI can prompt a re-index. */
export function listenIndexStatus(
  handler: (repoPath: string, status: IndexStatus) => void,
): Promise<UnlistenFn> {
  return listen<{ repoPath: string; status: IndexStatus }>("index:status", (e) =>
    handler(e.payload.repoPath, e.payload.status),
  );
}

export function listenRepoChanged(handler: (repo: RepoState) => void): Promise<UnlistenFn> {
  return listen<RepoState>("repo:changed", (e) => handler(e.payload));
}

/** terminal:data carries BASE64-encoded PTY output (decode before term.write). */
export function listenTerminalData(
  handler: (id: string, dataBase64: string) => void,
): Promise<UnlistenFn> {
  return listen<{ id: string; data: string }>("terminal:data", (e) =>
    handler(e.payload.id, e.payload.data),
  );
}

export function listenTerminalExit(
  handler: (id: string, code: number | null) => void,
): Promise<UnlistenFn> {
  return listen<{ id: string; code: number | null }>("terminal:exit", (e) =>
    handler(e.payload.id, e.payload.code),
  );
}
