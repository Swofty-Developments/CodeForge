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

export const getMessagesByThread = (threadId: string) =>
  invoke<ChatMessage[]>("get_messages_by_thread", { threadId });

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

// Sessions
export const sendMessage = (
  threadId: string,
  text: string,
  provider: string,
  cwd: string,
  model?: string
) => invoke("send_message", { threadId, text, provider, cwd, model: model || null });

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
}

export const createWorktree = (threadId: string, threadTitle: string, projectPath: string) =>
  invoke<WorktreeInfo>("create_worktree", { threadId, threadTitle, projectPath });

export const getWorktree = (threadId: string) =>
  invoke<WorktreeInfo | null>("get_worktree", { threadId });

export const mergeWorktree = (threadId: string, projectPath: string) =>
  invoke<string>("merge_worktree", { threadId, projectPath });

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

// Events
export const listenAgentEvent = (callback: (payload: AgentEventPayload) => void) =>
  listen<AgentEventPayload>("agent-event", (e) => callback(e.payload));
