/**
 * Typed wrapper for EVERY Tauri invoke + listen helper for each event channel.
 * Components never call invoke() directly. Tauri auto-converts camelCase JS args
 * to snake_case Rust params (repoPath -> repo_path).
 *
 * Registered commands (crates/tauri-app/src/main.rs — frozen contract):
 *   open_repo, close_repo, reindex_repo, daemon_status,
 *   get_features, get_feature, pin_feature, update_feature,
 *   get_timeline, get_diff_by_feature,
 *   start_session, send_session_input, approve_session, stop_session, list_sessions
 *
 * Event channels:
 *   "agent-event"     AgentEventPayload (all session streaming, demux by sessionId)
 *   "timeline:event"  TimelineEvent
 *   "index:progress"  IndexProgress
 *   "repo:changed"    RepoState
 */

import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type {
  AgentEventPayload,
  DaemonStatus,
  DiffByFeature,
  Feature,
  FeaturePatch,
  IndexProgress,
  MergeResult,
  PermissionMode,
  RepoState,
  SessionInfo,
  StartSessionOpts,
  TimelineEvent,
  TimelineFilter,
  Worktree,
} from "./types";

// ── Repo ────────────────────────────────────────────────────────────────────

export function openRepo(path: string): Promise<RepoState> {
  return invoke("open_repo", { path });
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

// ── Event listeners ─────────────────────────────────────────────────────────

export function listenAgentEvent(handler: (payload: AgentEventPayload) => void): Promise<UnlistenFn> {
  return listen<AgentEventPayload>("agent-event", (e) => handler(e.payload));
}

export function listenTimelineEvent(handler: (event: TimelineEvent) => void): Promise<UnlistenFn> {
  return listen<TimelineEvent>("timeline:event", (e) => handler(e.payload));
}

export function listenIndexProgress(handler: (progress: IndexProgress) => void): Promise<UnlistenFn> {
  return listen<IndexProgress>("index:progress", (e) => handler(e.payload));
}

export function listenRepoChanged(handler: (repo: RepoState) => void): Promise<UnlistenFn> {
  return listen<RepoState>("repo:changed", (e) => handler(e.payload));
}
