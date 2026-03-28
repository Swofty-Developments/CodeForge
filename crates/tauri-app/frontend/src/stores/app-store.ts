import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import type { Project, Thread, ChatMessage, SessionStatus, AgentEventPayload } from "../types";
import * as ipc from "../ipc";

export interface PendingApproval {
  sessionId: string;
  requestId: string;
  description: string;
  threadId: string;
}

export interface AppStore {
  projects: Project[];
  openTabs: string[];
  activeTab: string | null;
  sidebarWidth: number;
  settingsOpen: boolean;
  composerText: string;
  selectedProvider: string;
  threadMessages: Record<string, ChatMessage[]>;
  sessionStatuses: Record<string, SessionStatus>;
  pendingApprovals: PendingApproval[];
  contextMenu: { type: "thread" | "project"; id: string; x: number; y: number } | null;
  renamingThread: { id: string; text: string } | null;
  renamingProject: { id: string; text: string } | null;
  draggingTab: string | null;
  draggingSidebarThread: string | null;
  providerPickerOpen: boolean;
}

function createAppStore() {
  const [store, setStore] = createStore<AppStore>({
    projects: [],
    openTabs: [],
    activeTab: null,
    sidebarWidth: 260,
    settingsOpen: false,
    composerText: "",
    selectedProvider: "claude_code",
    threadMessages: {},
    sessionStatuses: {},
    pendingApprovals: [],
    contextMenu: null,
    renamingThread: null,
    renamingProject: null,
    draggingTab: null,
    draggingSidebarThread: null,
    providerPickerOpen: false,
  });

  async function loadData() {
    try {
      const rawProjects = await ipc.getAllProjects();
      const projects: Project[] = [];

      for (const p of rawProjects) {
        const rawThreads = await ipc.getThreadsByProject(p.id);
        projects.push({
          ...p,
          collapsed: false,
          threads: rawThreads.map((t) => ({ ...t })),
        });
      }

      setStore("projects", projects);
    } catch (e) {
      console.error("Failed to load data:", e);
    }
  }

  async function loadThreadMessages(threadId: string) {
    if (store.threadMessages[threadId]) return;
    try {
      const msgs = await ipc.getMessagesByThread(threadId);
      setStore("threadMessages", threadId, msgs);
    } catch (e) {
      console.error("Failed to load messages:", e);
    }
  }

  async function newThread(projectId?: string) {
    try {
      let targetProjectId = projectId;
      if (!targetProjectId) {
        const uncat = store.projects.find((p) => p.path === ".");
        if (!uncat) {
          const created = await ipc.createProject("Uncategorized", ".");
          const newProject: Project = { ...created, color: null, collapsed: false, threads: [] };
          setStore("projects", (prev) => [...prev, newProject]);
          targetProjectId = created.id;
        } else {
          targetProjectId = uncat.id;
        }
      }

      const count = store.projects.reduce((n, p) => n + p.threads.length, 0);
      const thread = await ipc.createThread(targetProjectId!, `Thread ${count + 1}`, store.selectedProvider);

      // Update the target project's threads and uncollapse it
      setStore("projects", (projects) =>
        projects.map((p) =>
          p.id === targetProjectId
            ? { ...p, threads: [...p.threads, thread], collapsed: false }
            : p
        )
      );
      setStore("openTabs", (tabs) => [...tabs, thread.id]);
      setStore("activeTab", thread.id);
      setStore("threadMessages", thread.id, []);
    } catch (e) {
      console.error("Failed to create thread:", e);
    }
  }

  function selectThread(id: string) {
    if (!store.openTabs.includes(id)) {
      setStore("openTabs", (tabs) => [...tabs, id]);
    }
    setStore("activeTab", id);
    loadThreadMessages(id);
  }

  function closeTab(id: string) {
    const remaining = store.openTabs.filter((t) => t !== id);
    setStore("openTabs", remaining);
    if (store.activeTab === id) {
      setStore("activeTab", remaining.length > 0 ? remaining[remaining.length - 1] : null);
    }
  }

  function reorderTabs(fromId: string, toIdx: number) {
    setStore("openTabs", (tabs) => {
      const arr = [...tabs];
      const fromIdx = arr.indexOf(fromId);
      if (fromIdx === -1 || fromIdx === toIdx) return arr;
      arr.splice(fromIdx, 1);
      arr.splice(toIdx, 0, fromId);
      return arr;
    });
  }

  async function sendUserMessage() {
    const text = store.composerText.trim();
    if (!text || !store.activeTab) return;

    const threadId = store.activeTab;
    setStore("composerText", "");

    try {
      const msgId = await ipc.persistUserMessage(threadId, text);
      const userMsg: ChatMessage = { id: msgId, thread_id: threadId, role: "user", content: text };
      setStore("threadMessages", threadId, (msgs) => [...(msgs || []), userMsg]);

      const project = store.projects.find((p) => p.threads.some((t) => t.id === threadId));
      const cwd = project && project.path !== "." ? project.path : ".";

      setStore("sessionStatuses", threadId, "generating");
      await ipc.sendMessage(threadId, text, store.selectedProvider, cwd);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (msgs) => [...(msgs || []), errMsg]);
      setStore("sessionStatuses", threadId, "error");
    }
  }

  async function approveRequest(approval: PendingApproval) {
    try {
      await ipc.respondToApproval(approval.sessionId, approval.requestId, true);
      setStore("pendingApprovals", (a) => a.filter((x) => x.requestId !== approval.requestId));
    } catch (e) {
      console.error("Failed to approve:", e);
    }
  }

  async function denyRequest(approval: PendingApproval) {
    try {
      await ipc.respondToApproval(approval.sessionId, approval.requestId, false);
      setStore("pendingApprovals", (a) => a.filter((x) => x.requestId !== approval.requestId));
    } catch (e) {
      console.error("Failed to deny:", e);
    }
  }

  function handleAgentEvent(payload: AgentEventPayload) {
    const { thread_id, event_type } = payload;

    switch (event_type) {
      case "content_delta": {
        setStore("threadMessages", thread_id, (msgs) => {
          if (!msgs) return [{ id: crypto.randomUUID(), thread_id, role: "assistant" as const, content: payload.text || "" }];
          const last = msgs[msgs.length - 1];
          if (last && last.role === "assistant" && !last.id.startsWith("done-")) {
            return [...msgs.slice(0, -1), { ...last, content: last.content + (payload.text || "") }];
          }
          return [...msgs, { id: crypto.randomUUID(), thread_id, role: "assistant" as const, content: payload.text || "" }];
        });
        break;
      }
      case "turn_started":
        setStore("sessionStatuses", thread_id, "generating");
        break;
      case "turn_completed":
        setStore("sessionStatuses", thread_id, "ready");
        setStore("threadMessages", thread_id, (msgs) => {
          if (!msgs) return msgs;
          const last = msgs[msgs.length - 1];
          if (last && last.role === "assistant" && !last.id.startsWith("done-")) {
            return [...msgs.slice(0, -1), { ...last, id: `done-${crypto.randomUUID()}` }];
          }
          return msgs;
        });
        break;
      case "turn_aborted":
        setStore("sessionStatuses", thread_id, "ready");
        setStore("threadMessages", thread_id, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Aborted: ${payload.reason}` },
        ]);
        break;
      case "session_ready":
        setStore("sessionStatuses", thread_id, "ready");
        break;
      case "session_error":
        setStore("sessionStatuses", thread_id, "error");
        setStore("threadMessages", thread_id, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Error: ${payload.message}` },
        ]);
        break;
      case "approval_required":
        if (payload.request_id && payload.description) {
          setStore("pendingApprovals", (a) => [
            ...a,
            {
              sessionId: payload.session_id,
              requestId: payload.request_id!,
              description: payload.description!,
              threadId: thread_id,
            },
          ]);
        }
        break;
    }
  }

  return {
    store,
    setStore,
    loadData,
    loadThreadMessages,
    newThread,
    selectThread,
    closeTab,
    reorderTabs,
    sendUserMessage,
    approveRequest,
    denyRequest,
    handleAgentEvent,
  };
}

export const appStore = createRoot(createAppStore);
