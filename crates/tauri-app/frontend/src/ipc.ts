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
  cwd: string
) => invoke("send_message", { threadId, text, provider, cwd });

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

// Events
export const listenAgentEvent = (callback: (payload: AgentEventPayload) => void) =>
  listen<AgentEventPayload>("agent-event", (e) => callback(e.payload));
