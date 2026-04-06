import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ChatMessage, AgentEventPayload } from "./types";

// Projects
export const getAllProjects = () =>
  invoke<{ id: string; name: string; path: string; color: string | null }[]>("get_all_projects");

export const getThreadsByProject = (projectId: string) =>
  invoke<{ id: string; project_id: string; title: string; color: string | null }[]>(
    "get_threads_by_project",
    { projectId }
  );

export const getMessagesByThread = (threadId: string, limit?: number, offset?: number) =>
  invoke<ChatMessage[]>("get_messages_by_thread", { threadId, limit: limit ?? null, offset: offset ?? null });

export const createProject = (name: string, path: string) =>
  invoke<{ id: string; name: string; path: string }>("create_project", { name, path });

export const renameProject = (id: string, name: string) =>
  invoke("rename_project", { id, name });

export const deleteProject = (id: string, deleteThreads: boolean) =>
  invoke("delete_project", { id, deleteThreads });

// Threads
export const createThread = (projectId: string, title: string, provider: string) =>
  invoke<{ id: string; project_id: string; title: string; color: string | null }>(
    "create_thread",
    { projectId, title, provider }
  );

export const renameThread = (id: string, title: string) =>
  invoke("rename_thread", { id, title });

export const setThreadColor = (id: string, color: string | null) =>
  invoke("set_thread_color", { id, color });

export const deleteThread = (id: string) => invoke("delete_thread", { id });

export const moveThreadToProject = (threadId: string, targetProjectId: string) =>
  invoke("move_thread_to_project", { threadId, targetProjectId });

export const persistUserMessage = (threadId: string, content: string) =>
  invoke<string>("persist_user_message", { threadId, content });

export const deleteMessagesAfter = (threadId: string, messageId: string) =>
  invoke<number>("delete_messages_after", { threadId, messageId });

// Sessions
export const sendMessage = (
  threadId: string,
  text: string,
  provider: string,
  cwd: string,
  model?: string,
  permissionMode?: string
) => invoke("send_message", { threadId, text, provider, cwd, model: model || null, permissionMode: permissionMode || null });

export const interruptSession = (threadId: string) =>
  invoke("interrupt_session", { threadId });

export const stopSession = (threadId: string) =>
  invoke("stop_session", { threadId });

export const respondToApproval = (
  sessionId: string,
  requestId: string,
  approve: boolean
) => invoke("respond_to_approval", { sessionId, requestId, approve });

// Settings
export const getSetting = (key: string) =>
  invoke<string | null>("get_setting", { key });

export const getSettingsBatch = (keys: string[]) =>
  invoke<Record<string, string>>("get_settings_batch", { keys });

export const setSetting = (key: string, value: string) =>
  invoke("set_setting", { key, value });

// Providers
export interface ProviderInfo {
  id: string;
  name: string;
  installed: boolean;
  path: string | null;
  version: string | null;
  install_instructions: string;
  description: string;
  website: string;
}

export const getProviderInfo = () =>
  invoke<ProviderInfo[]>("get_provider_info");

// Worktrees
export interface WorktreeInfo {
  thread_id: string;
  branch: string;
  path: string;
  active: boolean;
  pr_number: number | null;
  /** "active" | "merged" | "closed" | "deleted" | "orphaned" */
  status: string;
  /** Cached from last GitHub poll: "open" | "closed" | "merged" | null */
  pr_state: string | null;
  /** Cached merge commit SHA (set iff pr_state === "merged") */
  pr_merge_commit: string | null;
  /** Cached PR URL for clickable links without another gh call */
  pr_url: string | null;
}

export const createWorktree = (threadId: string, threadTitle: string, projectPath: string, projectId: string) =>
  invoke<WorktreeInfo>("create_worktree", { threadId, threadTitle, projectPath, projectId });

export const getWorktree = (threadId: string) =>
  invoke<WorktreeInfo | null>("get_worktree", { threadId });

export const mergeWorktree = (threadId: string, projectPath: string) =>
  invoke<string>("merge_worktree", { threadId, projectPath });

export const createPrFromWorktree = (threadId: string, projectPath: string, title: string, body: string) =>
  invoke<string>("create_pr_from_worktree", { threadId, projectPath, title, body });

export interface PrStatus {
  pr_number: number;
  ci_status: string;         // "success" | "failure" | "pending" | "none" | "unknown"
  review_status: string;     // "approved" | "changes_requested" | "commented" | "none"
  comment_count: number;
  /** Fully-resolved lifecycle state — the backend is the single source of truth. */
  lifecycle: import("./types").LifecycleState;
  /** Worktree status before this reconcile tick, so the frontend can emit
   *  transition events only on change. One of: active/merged/closed/deleted/orphaned. */
  previous_status: string;
  /** Count of review comments new since the last persisted high-water mark. */
  new_comment_count: number;
  /** One-shot: this tick detected a revert (previously merged, now unreachable). */
  revert_detected: boolean;
  /** One-shot: this tick detected a reopen (merged/closed → open). */
  reopen_detected: boolean;
  /** One-shot: the PR no longer exists on GitHub. Linkage has been cleared. */
  pr_missing: boolean;
}

export const getPrStatus = (threadId: string, projectPath: string) =>
  invoke<PrStatus | null>("get_pr_status", { threadId, projectPath });

export interface PrComment {
  author: string;
  state: string;
  body: string;
}

export const getPrReviewComments = (threadId: string, projectPath: string) =>
  invoke<PrComment[]>("get_pr_review_comments", { threadId, projectPath });

export interface OpenPr {
  number: number;
  title: string;
  branch: string;
  author: string;
  url: string;
}

export const listOpenPrs = (projectPath: string) =>
  invoke<OpenPr[]>("list_open_prs", { projectPath });

export const findThreadForPr = (prNumber: number) =>
  invoke<string | null>("find_thread_for_pr", { prNumber });

export const checkoutPrIntoWorktree = (threadId: string, prNumber: number, projectPath: string, projectId: string) =>
  invoke<WorktreeInfo>("checkout_pr_into_worktree", { threadId, prNumber, projectPath, projectId });

/**
 * Link an existing PR to a thread. Persists the linkage to the worktrees table
 * so it survives restarts and is visible to the poller. Handles three cases:
 *   - thread has no worktree: checks out the PR branch into a new worktree
 *   - thread has active worktree w/o PR: stamps the PR number on the existing row
 *   - thread has active worktree w/ different PR: returns an error
 */
export const linkPrToThread = (threadId: string, prNumber: number, projectPath: string, projectId: string) =>
  invoke<WorktreeInfo>("link_pr_to_thread", { threadId, prNumber, projectPath, projectId });

// Health monitoring
export interface WorktreeHealth {
  thread_id: string;
  status: string; // "healthy" | "missing" | "orphaned" | "detached_head"
  branch: string;
  path: string;
}

export const validateWorktrees = (projectPath: string, projectId: string) =>
  invoke<WorktreeHealth[]>("validate_worktrees", { projectPath, projectId });

export const repairWorktree = (threadId: string, projectPath: string, action: string) =>
  invoke<WorktreeInfo>("repair_worktree", { threadId, projectPath, action });

export const cleanupWorktrees = (projectPath: string) =>
  invoke<number>("cleanup_worktrees", { projectPath });

// Conflict resolution
export const getConflictFiles = (cwd: string) =>
  invoke<string[]>("get_conflict_files", { cwd });

export interface ConflictFile {
  path: string;
  ours: string;
  theirs: string;
  base: string;
}

export const getConflictMarkers = (cwd: string, filePath: string) =>
  invoke<ConflictFile>("get_conflict_markers", { cwd, filePath });

export const resolveConflict = (cwd: string, filePath: string, resolution: string) =>
  invoke<void>("resolve_conflict", { cwd, filePath, resolution });

export const finalizeMerge = (cwd: string) =>
  invoke<string>("finalize_merge", { cwd });

export const abortMerge = (cwd: string) =>
  invoke<void>("abort_merge", { cwd });

// Per-turn diff
export const getTurnDiff = (cwd: string, baseCommit: string) =>
  invoke<any[]>("get_turn_diff", { cwd, baseCommit });

export const getTurnChangedFiles = (cwd: string, baseCommit: string) =>
  invoke<any[]>("get_turn_changed_files", { cwd, baseCommit });

export const getHeadCommit = (cwd: string) =>
  invoke<string>("get_head_commit", { cwd });

// Blame
export interface BlameLine {
  line_number: number;
  commit_hash: string;
  author: string;
  date: string;
  content: string;
}

export const getFileBlame = (cwd: string, filePath: string, revision?: string) =>
  invoke<BlameLine[]>("get_file_blame", { cwd, filePath, revision });

// Worktree sync status
export const checkWorktreeSyncStatus = (threadId: string) =>
  invoke<string>("check_worktree_sync_status", { threadId });

// Undo
export const undoToCommit = (threadId: string, commitSha: string) =>
  invoke<string>("undo_to_commit", { threadId, commitSha });

// Thread forking
export const forkThread = (threadId: string, messageId: string, newTitle: string) =>
  invoke<{ id: string; project_id: string; title: string; color: string | null }>("fork_thread", { threadId, messageId, newTitle });

// Search
export interface SearchResult {
  thread_id: string;
  thread_title: string;
  project_name: string;
  message_id: string;
  role: string;
  content_snippet: string;
  match_index: number;
}

export const searchMessages = (query: string) =>
  invoke<SearchResult[]>("search_messages", { query });

// Usage
export interface ThreadCost {
  thread_id: string;
  thread_title: string;
  cost_usd: number;
  total_tokens: number;
}

export interface ModelCost {
  model: string;
  cost_usd: number;
  total_tokens: number;
}

export interface UsageSummary {
  total_input_tokens: number;
  total_output_tokens: number;
  total_cache_read_tokens: number;
  total_cache_write_tokens: number;
  total_cost_usd: number;
  thread_costs: ThreadCost[];
  model_costs: ModelCost[];
}

export interface ThreadUsage {
  thread_id: string;
  input_tokens: number;
  output_tokens: number;
  cache_read_tokens: number;
  cache_write_tokens: number;
  cost_usd: number;
}

export const getUsageSummary = () =>
  invoke<UsageSummary>("get_usage_summary");

export const getThreadUsage = (threadId: string) =>
  invoke<ThreadUsage>("get_thread_usage", { threadId });

// Diff
export interface ChangedFile {
  path: string;
  status: string;
  insertions: number;
  deletions: number;
}

export interface DiffLine {
  line_type: string;
  content: string;
  old_line: number | null;
  new_line: number | null;
}

export interface DiffHunk {
  header: string;
  lines: DiffLine[];
}

export interface FileDiff {
  path: string;
  hunks: DiffHunk[];
}

export const getChangedFiles = (cwd: string) =>
  invoke<ChangedFile[]>("get_changed_files", { cwd });

export const getSessionDiff = (cwd: string) =>
  invoke<FileDiff[]>("get_session_diff", { cwd });

export const getFileDiff = (cwd: string, filePath: string) =>
  invoke<string>("get_file_diff", { cwd, filePath });

export const getFileContent = (cwd: string, filePath: string, version: string) =>
  invoke<string>("get_file_content", { cwd, filePath, version });

// Browser (CDP screencast via Playwright sidecar)
export const browserNavigate = (threadId: string, url: string) =>
  invoke("browser_navigate", { threadId, url });
export const browserClick = (threadId: string, x: number, y: number) =>
  invoke("browser_click", { threadId, x, y });
export const browserScroll = (threadId: string, x: number, y: number, deltaX: number, deltaY: number) =>
  invoke("browser_scroll", { threadId, x, y, deltaX, deltaY });
export const browserMouseMove = (threadId: string, x: number, y: number) =>
  invoke("browser_mouse_move", { threadId, x, y });
export const browserKeyDown = (threadId: string, key: string, text: string) =>
  invoke("browser_key_down", { threadId, key, text });
export const browserKeyUp = (threadId: string, key: string) =>
  invoke("browser_key_up", { threadId, key });
export const browserTypeText = (threadId: string, text: string) =>
  invoke("browser_type_text", { threadId, text });
export const browserBack = (threadId: string) =>
  invoke("browser_back", { threadId });
export const browserForward = (threadId: string) =>
  invoke("browser_forward", { threadId });
export const browserReload = (threadId: string) =>
  invoke("browser_reload", { threadId });
export const browserResize = (threadId: string, width: number, height: number) =>
  invoke("browser_resize", { threadId, width, height });
export const browserStartInspect = (threadId: string) =>
  invoke("browser_start_inspect", { threadId });
export const browserStopInspect = (threadId: string) =>
  invoke("browser_stop_inspect", { threadId });
export const browserExtract = (threadId: string) =>
  invoke("browser_extract", { threadId });
export const browserClose = (threadId: string) =>
  invoke("browser_close", { threadId });

export interface BrowserEventPayload {
  thread_id: string;
  type: string;
  data?: string;
  url?: string;
  html?: string;
  css?: string;
  selector?: string;
  message?: string;
}

export const listenBrowserEvent = (cb: (p: BrowserEventPayload) => void) =>
  listen<BrowserEventPayload>("browser-event", (e) => cb(e.payload));

// Naming
export const autoNameThread = (threadId: string, messagesSummary: string, provider: string) =>
  invoke<string>("auto_name_thread", { threadId, messagesSummary, provider });

// Onboarding
export interface BinaryStatus {
  name: string;
  installed: boolean;
  version: string | null;
  path: string | null;
}

export interface SetupStatus {
  complete: boolean;
  binaries: BinaryStatus[];
  has_any_binary: boolean;
  gh_installed: boolean;
  gh_authenticated: boolean;
  gh_username: string | null;
}

export const checkSetupStatus = () =>
  invoke<SetupStatus>("check_setup_status");

export const completeSetup = () =>
  invoke("complete_setup");

// GitHub
export interface GhAuthStatus {
  logged_in: boolean;
  username: string | null;
  scopes: string[];
}

export interface PullRequest {
  number: number;
  title: string;
  state: string;
  author: string;
  branch: string;
  base: string;
  url: string;
  additions: number;
  deletions: number;
  changed_files: number;
  created_at: string;
  updated_at: string;
  draft: boolean;
  labels: string[];
  review_status: string;
}

export interface Issue {
  number: number;
  title: string;
  state: string;
  author: string;
  body: string;
  url: string;
  labels: string[];
  created_at: string;
  comments_count: number;
}

export const ghAuthStatus = () =>
  invoke<GhAuthStatus>("gh_auth_status");

export const ghLogin = () =>
  invoke<string>("gh_login");

export const listPrs = (repoPath: string, state?: string) =>
  invoke<PullRequest[]>("list_prs", { repoPath, state: state || null });

export const getPrDiff = (repoPath: string, prNumber: number) =>
  invoke<string>("get_pr_diff", { repoPath, prNumber });

export const listIssues = (repoPath: string, state?: string, search?: string) =>
  invoke<Issue[]>("list_issues", { repoPath, state: state || null, search: search || null });

export const getIssueContext = (repoPath: string, issueNumber: number) =>
  invoke<string>("get_issue_context", { repoPath, issueNumber });

export const getRepoInfo = (repoPath: string) =>
  invoke<any>("get_repo_info", { repoPath });

export const isGithubRepo = (path: string) =>
  invoke<boolean>("is_github_repo", { path });

// Git
export interface GitLogEntry {
  hash: string;
  message: string;
  author: string;
  date: string;
}

export interface GitBranch {
  name: string;
  current: boolean;
  remote: boolean;
}

export interface GitStatusEntry {
  path: string;
  status: string;
  staged: boolean;
}

export const gitLog = (cwd: string, limit: number) =>
  invoke<GitLogEntry[]>("git_log", { cwd, limit });

export const gitBranches = (cwd: string) =>
  invoke<GitBranch[]>("git_branches", { cwd });

export const gitCheckout = (cwd: string, branch: string) =>
  invoke<string>("git_checkout", { cwd, branch });

export const gitCreateBranch = (cwd: string, name: string) =>
  invoke<string>("git_create_branch", { cwd, name });

export const gitCommit = (cwd: string, message: string, files: string[]) =>
  invoke<string>("git_commit", { cwd, message, files });

export const gitPush = (cwd: string) =>
  invoke<string>("git_push", { cwd });

export const gitPushForce = (cwd: string) =>
  invoke<string>("git_push_force", { cwd });

export const gitFetch = (cwd: string) =>
  invoke<string>("git_fetch", { cwd });

export const gitPull = (cwd: string) =>
  invoke<string>("git_pull", { cwd });

export const gitDeleteBranch = (cwd: string, name: string, force?: boolean) =>
  invoke<string>("git_delete_branch", { cwd, name, force: force ?? false });

export const gitMergeBranch = (cwd: string, branch: string) =>
  invoke<string>("git_merge_branch", { cwd, branch });

export const gitStash = (cwd: string, message?: string) =>
  invoke<string>("git_stash", { cwd, message: message ?? null });

export const gitStashPop = (cwd: string) =>
  invoke<string>("git_stash_pop", { cwd });

export const gitCreatePr = (cwd: string, title: string, body: string, branch: string, base: string) =>
  invoke<string>("git_create_pr", { cwd, title, body, branch, base });

export const gitDiffBranches = (cwd: string, branch1: string, branch2: string) =>
  invoke<any>("git_diff_branches", { cwd, branch1, branch2 });

export const gitStatus = (cwd: string) =>
  invoke<GitStatusEntry[]>("git_status", { cwd });

export interface RemoteUpdate {
  branch: string;
  behind: number;
  latest_message: string;
}

export interface RepoStatus {
  status: "none" | "git" | "github";
  branch: string | null;
  has_remote: boolean;
}

export const gitRepoStatus = (cwd: string) =>
  invoke<RepoStatus>("git_repo_status", { cwd });

export const gitInitRepo = (cwd: string) =>
  invoke<string>("git_init_repo", { cwd });

export const gitCheckRemote = (cwd: string, branch?: string, prNumber?: string) =>
  invoke<RemoteUpdate | null>("git_check_remote", { cwd, branch: branch ?? null, prNumber: prNumber ?? null });

export const gitPullBranch = (cwd: string, branch?: string, prNumber?: string) =>
  invoke<string>("git_pull_branch", { cwd, branch: branch ?? null, prNumber: prNumber ?? null });

// MCP
export interface McpServer {
  name: string;
  url_or_command: string;
  transport: string;
  scope: string;
  status: string;
}

export interface SlashCommand {
  name: string;
  description: string;
  source: string;
}

export const mcpListServers = (provider: string) =>
  invoke<McpServer[]>("mcp_list_servers", { provider });

export const mcpAddServer = (
  provider: string,
  name: string,
  urlOrCommand: string,
  transport: string,
  scope: string,
  envVars: string[],
  extraArgs: string[],
) => invoke<string>("mcp_add_server", { provider, name, urlOrCommand, transport, scope, envVars, extraArgs });

export const mcpRemoveServer = (provider: string, name: string, scope: string) =>
  invoke<string>("mcp_remove_server", { provider, name, scope });

export const listSlashCommands = (provider: string) =>
  invoke<SlashCommand[]>("list_slash_commands", { provider });

// Skills / Plugins
export interface SkillInfo {
  name: string;
  source: string;
  enabled: boolean;
  version: string | null;
}

export interface MarketplaceSource {
  name: string;
  source: string;
}

export const listSkills = (provider: string) =>
  invoke<SkillInfo[]>("list_skills", { provider });

export const installSkill = (provider: string, name: string) =>
  invoke<string>("install_skill", { provider, name });

export const uninstallSkill = (provider: string, name: string) =>
  invoke<string>("uninstall_skill", { provider, name });

export const enableSkill = (provider: string, name: string) =>
  invoke<string>("enable_skill", { provider, name });

export const disableSkill = (provider: string, name: string) =>
  invoke<string>("disable_skill", { provider, name });

export const listMarketplaces = (provider: string) =>
  invoke<MarketplaceSource[]>("list_marketplaces", { provider });

export const addMarketplace = (provider: string, source: string) =>
  invoke<string>("add_marketplace", { provider, source });

// Themes
export interface ThemeData {
  id: string;
  name: string;
  description: string;
  preview: string[];
  vars: Record<string, string>;
  is_custom: boolean;
}

export const listThemes = () =>
  invoke<ThemeData[]>("list_themes");

export const importTheme = (jsonContent: string) =>
  invoke<ThemeData>("import_theme", { jsonContent });

export const deleteCustomTheme = (id: string) =>
  invoke("delete_custom_theme", { id });

export const exportTheme = (id: string) =>
  invoke<string>("export_theme", { id });

// Filesystem
export const openInFileManager = (path: string) =>
  invoke("open_in_file_manager", { path });

/**
 * Open an http/https URL in the user's default browser. Use this instead of
 * `window.open` — the Tauri webview silently drops `window.open` for most URLs.
 */
export const openExternalUrl = (url: string) =>
  invoke("open_external_url", { url });

// Events
export const listenAgentEvent = (callback: (payload: AgentEventPayload) => void) =>
  listen<AgentEventPayload>("agent-event", (e) => callback(e.payload));
