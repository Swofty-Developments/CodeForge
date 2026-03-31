import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import type { Project, Thread, ChatMessage, SessionStatus, AgentEventPayload, Attachment, ContentBlock, ThreadTokenUsage } from "../types";
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
  commandPaletteOpen: boolean;
  searchOpen: boolean;
  usageDashboardOpen: boolean;
  worktrees: Record<string, { thread_id: string; branch: string; path: string; active: boolean } | undefined>;
  splitTab: string | null;
  diffPanelOpen: boolean;
  selectedModel: string | null;
  threadBrowserOpen: Record<string, boolean>;
  threadBrowserUrls: Record<string, string>;
  autoNamingEnabled: boolean;
  namingInProgress: Record<string, boolean>;
  attachments: Attachment[];
  threadTokenUsage: Record<string, ThreadTokenUsage>;
  notificationsEnabled: boolean;
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
    commandPaletteOpen: false,
    searchOpen: false,
    usageDashboardOpen: false,
    worktrees: {},
    splitTab: null,
    diffPanelOpen: false,
    selectedModel: null,
    threadBrowserOpen: {},
    threadBrowserUrls: {},
    autoNamingEnabled: true,
    namingInProgress: {},
    attachments: [],
    threadTokenUsage: {},
    notificationsEnabled: true,
  });

  async function loadData() {
    try {
      const rawProjects = await ipc.getAllProjects();
      const projects: Project[] = [];

      for (const p of rawProjects) {
        const rawThreads = await ipc.getThreadsByProject(p.id);
        const savedColor = await ipc.getSetting(`project_color:${p.id}`);
        projects.push({
          ...p,
          color: savedColor || null,
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

  function setSplitTab(id: string | null) {
    setStore("splitTab", id);
    if (id) loadThreadMessages(id);
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
    const atts = [...store.attachments];
    if ((!text && atts.length === 0) || !store.activeTab) return;

    const threadId = store.activeTab;
    setStore("composerText", "");
    setStore("attachments", []);

    // Build full message: user text + attachment context blocks
    let fullText = text;
    for (const att of atts) {
      const lang = att.language || (att.type === "extraction" ? "html" : "");
      fullText += `\n\n--- Attached: ${att.name} ---\n\`\`\`${lang}\n${att.content}\n\`\`\``;
    }

    try {
      // Finalize any in-progress assistant message before appending the user message
      // (handles mid-response steering where user sends while agent is generating)
      setStore("threadMessages", threadId, (msgs) => {
        if (!msgs) return msgs;
        const last = msgs[msgs.length - 1];
        if (last && last.role === "assistant" && !last.id.startsWith("done-")) {
          return [...msgs.slice(0, -1), { ...last, id: `done-${crypto.randomUUID()}` }];
        }
        return msgs;
      });

      const msgId = await ipc.persistUserMessage(threadId, fullText);
      const userMsg: ChatMessage = { id: msgId, thread_id: threadId, role: "user", content: fullText };
      setStore("threadMessages", threadId, (msgs) => [...(msgs || []), userMsg]);

      const project = store.projects.find((p) => p.threads.some((t) => t.id === threadId));
      const wt = store.worktrees[threadId];
      const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

      setStore("sessionStatuses", threadId, "generating");
      await ipc.sendMessage(threadId, text, store.selectedProvider, cwd, store.selectedModel ?? undefined);
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

  const turnStartTimes: Record<string, number> = {};

  /** Send a desktop notification if permitted. */
  function sendNotification(title: string, body: string) {
    if (!store.notificationsEnabled) return;
    if (typeof Notification === "undefined") return;
    if (Notification.permission !== "granted") return;
    try {
      new Notification(title, { body });
    } catch {
      // Silently ignore if notification fails
    }
  }

  /** Look up the thread title by id. */
  function getThreadTitle(threadId: string): string {
    for (const p of store.projects) {
      const t = p.threads.find((t) => t.id === threadId);
      if (t) return t.title;
    }
    return "Thread";
  }

  /** Request notification permission if not already granted. */
  async function requestNotificationPermission() {
    if (typeof Notification === "undefined") return;
    if (Notification.permission === "default") {
      await Notification.requestPermission();
    }
  }

  /** Get or create the in-progress assistant message for a thread. */
  function getOrCreateAssistantMsg(threadId: string): ChatMessage {
    const msgs = store.threadMessages[threadId];
    if (msgs && msgs.length > 0) {
      const last = msgs[msgs.length - 1];
      if (last.role === "assistant" && !last.id.startsWith("done-")) {
        return last;
      }
    }
    return {
      id: crypto.randomUUID(),
      thread_id: threadId,
      role: "assistant",
      content: "",
      blocks: [],
    };
  }

  /** Update the in-progress assistant message, appending it if new. */
  function updateAssistantMsg(threadId: string, updater: (msg: ChatMessage) => ChatMessage) {
    setStore("threadMessages", threadId, (msgs) => {
      if (!msgs) msgs = [];
      const last = msgs[msgs.length - 1];
      if (last && last.role === "assistant" && !last.id.startsWith("done-")) {
        return [...msgs.slice(0, -1), updater(last)];
      }
      // No active assistant msg — create one
      const fresh: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "assistant",
        content: "",
        blocks: [],
      };
      return [...msgs, updater(fresh)];
    });
  }

  /** Append text to the last text block, or create a new one. */
  function appendTextBlock(blocks: ContentBlock[], text: string): ContentBlock[] {
    const result = [...blocks];
    const last = result[result.length - 1];
    if (last && last.type === "text") {
      result[result.length - 1] = { ...last, content: last.content + text };
    } else {
      result.push({ type: "text", content: text });
    }
    return result;
  }

  /** Flatten blocks to a plain text string for persistence. */
  function flattenBlocks(blocks: ContentBlock[]): string {
    return blocks
      .filter((b) => b.type === "text")
      .map((b) => b.content)
      .join("");
  }

  function handleAgentEvent(payload: AgentEventPayload) {
    const { thread_id, event_type } = payload;

    switch (event_type) {
      case "content_delta": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = appendTextBlock(msg.blocks || [], payload.text || "");
          return { ...msg, content: msg.content + (payload.text || ""), blocks };
        });
        break;
      }
      case "thinking_delta": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = [...(msg.blocks || [])];
          const last = blocks[blocks.length - 1];
          if (last && last.type === "thinking") {
            blocks[blocks.length - 1] = { ...last, content: last.content + (payload.text || "") };
          } else {
            blocks.push({ type: "thinking", content: payload.text || "" });
          }
          return { ...msg, blocks };
        });
        break;
      }
      case "tool_use_start": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = [...(msg.blocks || [])];
          blocks.push({
            type: "tool_use",
            content: "",
            tool_id: payload.tool_id,
            tool_name: payload.tool_name,
            tool_input: "",
            tool_status: "generating",
          });
          return { ...msg, blocks };
        });
        break;
      }
      case "tool_input_delta": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = [...(msg.blocks || [])];
          // Find the tool block by id
          for (let i = blocks.length - 1; i >= 0; i--) {
            if (blocks[i].type === "tool_use" && blocks[i].tool_id === payload.tool_id) {
              blocks[i] = { ...blocks[i], tool_input: (blocks[i].tool_input || "") + (payload.input_json || "") };
              break;
            }
          }
          return { ...msg, blocks };
        });
        break;
      }
      case "tool_use_end": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = [...(msg.blocks || [])];
          for (let i = blocks.length - 1; i >= 0; i--) {
            if (blocks[i].type === "tool_use" && blocks[i].tool_id === payload.tool_id) {
              blocks[i] = { ...blocks[i], tool_status: "running" };
              break;
            }
          }
          return { ...msg, blocks };
        });
        break;
      }
      case "tool_result": {
        updateAssistantMsg(thread_id, (msg) => {
          const blocks = [...(msg.blocks || [])];
          for (let i = blocks.length - 1; i >= 0; i--) {
            if (blocks[i].type === "tool_use" && blocks[i].tool_id === payload.tool_id) {
              // If the result carries an empty tool_name, inherit from the block
              const resolvedName = payload.tool_name || blocks[i].tool_name;
              blocks[i] = {
                ...blocks[i],
                tool_name: resolvedName || blocks[i].tool_name,
                tool_output: payload.tool_output,
                tool_status: payload.is_error ? "error" : "completed",
                tool_error: payload.is_error,
              };
              break;
            }
          }
          return { ...msg, blocks };
        });
        break;
      }
      case "turn_started":
        turnStartTimes[thread_id] = Date.now();
        setStore("sessionStatuses", thread_id, "generating");
        break;
      case "turn_completed": {
        setStore("sessionStatuses", thread_id, "ready");
        setStore("threadMessages", thread_id, (msgs) => {
          if (!msgs) return msgs;
          const last = msgs[msgs.length - 1];
          if (last && last.role === "assistant" && !last.id.startsWith("done-")) {
            // Mark any still-running tools as completed
            const blocks = (last.blocks || []).map((b) =>
              b.type === "tool_use" && (b.tool_status === "running" || b.tool_status === "generating")
                ? { ...b, tool_status: "completed" as const }
                : b
            );
            return [...msgs.slice(0, -1), { ...last, id: `done-${crypto.randomUUID()}`, blocks, content: last.content || flattenBlocks(blocks) }];
          }
          return msgs;
        });
        // Notify for background threads (not the active tab)
        if (thread_id !== store.activeTab) {
          const title = getThreadTitle(thread_id);
          const msgs = store.threadMessages[thread_id];
          const lastMsg = msgs?.[msgs.length - 1];
          const preview = lastMsg?.content?.slice(0, 120) || "Response complete";
          sendNotification(`${title} - Complete`, preview);
        }
        maybeAutoNameThread(thread_id);
        break;
      }
      case "turn_aborted":
        setStore("sessionStatuses", thread_id, "ready");
        setStore("threadMessages", thread_id, (msgs) => {
          if (!msgs) return [{ id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Aborted: ${payload.reason}` }];
          // Finalize any in-progress assistant message and mark running tools as error
          const updated = msgs.map((m) => {
            if (m.role === "assistant" && !m.id.startsWith("done-")) {
              const blocks = (m.blocks || []).map((b) =>
                b.type === "tool_use" && (b.tool_status === "running" || b.tool_status === "generating")
                  ? { ...b, tool_status: "error" as const }
                  : b
              );
              return { ...m, id: `done-${crypto.randomUUID()}`, blocks };
            }
            return m;
          });
          return [...updated, { id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Aborted: ${payload.reason}` }];
        });
        break;
      case "usage_report": {
        const durationMs = turnStartTimes[thread_id] ? Date.now() - turnStartTimes[thread_id] : undefined;
        setStore("threadMessages", thread_id, (msgs) => {
          if (!msgs) return msgs;
          const last = msgs[msgs.length - 1];
          if (last && last.role === "assistant") {
            return [...msgs.slice(0, -1), {
              ...last,
              meta: {
                model: payload.model,
                inputTokens: payload.input_tokens,
                outputTokens: payload.output_tokens,
                costUsd: payload.cost_usd,
                durationMs,
              },
            }];
          }
          return msgs;
        });

        // Accumulate token usage for context window tracking
        const inputTkns = payload.input_tokens ?? 0;
        const outputTkns = payload.output_tokens ?? 0;
        const cacheReadTkns = payload.cache_read_tokens ?? 0;
        const cacheWriteTkns = payload.cache_write_tokens ?? 0;
        const prev = store.threadTokenUsage[thread_id];
        setStore("threadTokenUsage", thread_id, {
          inputTokens: (prev?.inputTokens ?? 0) + inputTkns,
          outputTokens: (prev?.outputTokens ?? 0) + outputTkns,
          cacheReadTokens: (prev?.cacheReadTokens ?? 0) + cacheReadTkns,
          cacheWriteTokens: (prev?.cacheWriteTokens ?? 0) + cacheWriteTkns,
          totalTokens: (prev?.totalTokens ?? 0) + inputTkns + outputTkns,
          model: payload.model ?? prev?.model,
        });
        break;
      }
      case "session_ready":
        setStore("sessionStatuses", thread_id, "ready");
        break;
      case "session_error": {
        setStore("sessionStatuses", thread_id, "error");
        setStore("threadMessages", thread_id, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Error: ${payload.message}` },
        ]);
        // Always notify on errors
        const errTitle = getThreadTitle(thread_id);
        sendNotification(`${errTitle} - Error`, payload.message || "Session error");
        break;
      }
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

  async function maybeAutoNameThread(threadId: string) {
    if (!store.autoNamingEnabled) return;
    if (store.namingInProgress[threadId]) return;

    const msgs = store.threadMessages[threadId];
    if (!msgs || msgs.length < 3) return;

    // Check if thread still has a default name like "Thread N"
    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === threadId);
    if (!thread || !thread.title.match(/^Thread \d+$/)) return;

    setStore("namingInProgress", threadId, true);

    try {
      // Build a summary of the first few messages
      const summary = msgs
        .slice(0, 6)
        .map((m) => `${m.role}: ${m.content.slice(0, 200)}`)
        .join("\n");

      const name = await ipc.autoNameThread(threadId, summary, store.selectedProvider);
      if (name) {
        setStore("projects", (projects) =>
          projects.map((p) => ({
            ...p,
            threads: p.threads.map((t) => t.id === threadId ? { ...t, title: name } : t),
          }))
        );
      }
    } catch (e) {
      console.error("Auto-naming failed:", e);
    } finally {
      setStore("namingInProgress", threadId, false);
    }
  }

  async function editAndResend(threadId: string, messageId: string, newText: string) {
    // Remove messages after (and including) the edited message from DB
    // The messageId is the user message to edit — delete everything after it
    try {
      await ipc.deleteMessagesAfter(threadId, messageId);
    } catch (e) {
      console.error("Failed to delete messages after:", e);
    }

    // Remove from local store: keep messages up to and including the target message index,
    // then remove the target message itself (we'll re-send it)
    setStore("threadMessages", threadId, (msgs) => {
      if (!msgs) return msgs;
      const idx = msgs.findIndex((m) => m.id === messageId);
      if (idx === -1) return msgs;
      return msgs.slice(0, idx);
    });

    // Put text into composer and let the user send it (or auto-send)
    setStore("composerText", newText);
  }

  async function retryLastMessage(threadId: string) {
    const msgs = store.threadMessages[threadId];
    if (!msgs || msgs.length === 0) return;

    // Find the last user message
    let lastUserIdx = -1;
    for (let i = msgs.length - 1; i >= 0; i--) {
      if (msgs[i].role === "user") {
        lastUserIdx = i;
        break;
      }
    }
    if (lastUserIdx === -1) return;

    const userMsg = msgs[lastUserIdx];
    const userText = userMsg.content;

    // Delete messages after the user message from DB
    try {
      await ipc.deleteMessagesAfter(threadId, userMsg.id);
    } catch (e) {
      console.error("Failed to delete messages:", e);
    }

    // Keep messages up to and including the user message, remove the rest
    setStore("threadMessages", threadId, (m) => m ? m.slice(0, lastUserIdx + 1) : m);

    // Re-send using the same flow as sendUserMessage but with known text
    const project = store.projects.find((p) => p.threads.some((t) => t.id === threadId));
    const wt = store.worktrees[threadId];
    const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

    try {
      setStore("sessionStatuses", threadId, "generating");
      await ipc.sendMessage(threadId, userText, store.selectedProvider, cwd, store.selectedModel ?? undefined);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (m) => [...(m || []), errMsg]);
      setStore("sessionStatuses", threadId, "error");
    }
  }

  async function regenerateResponse(threadId: string, assistantMsgId: string) {
    const msgs = store.threadMessages[threadId];
    if (!msgs) return;

    // Find the assistant message
    const assistantIdx = msgs.findIndex((m) => m.id === assistantMsgId);
    if (assistantIdx === -1) return;

    // Find the preceding user message
    let userIdx = -1;
    for (let i = assistantIdx - 1; i >= 0; i--) {
      if (msgs[i].role === "user") {
        userIdx = i;
        break;
      }
    }
    if (userIdx === -1) return;

    const userText = msgs[userIdx].content;

    // Delete assistant message and anything after the user message from DB
    try {
      await ipc.deleteMessagesAfter(threadId, msgs[userIdx].id);
    } catch (e) {
      console.error("Failed to delete messages:", e);
    }

    // Keep messages up to and including the user message
    setStore("threadMessages", threadId, (m) => m ? m.slice(0, userIdx + 1) : m);

    // Re-send
    const project = store.projects.find((p) => p.threads.some((t) => t.id === threadId));
    const wt = store.worktrees[threadId];
    const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

    try {
      setStore("sessionStatuses", threadId, "generating");
      await ipc.sendMessage(threadId, userText, store.selectedProvider, cwd, store.selectedModel ?? undefined);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (m) => [...(m || []), errMsg]);
      setStore("sessionStatuses", threadId, "error");
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
    setSplitTab,
    handleAgentEvent,
    editAndResend,
    retryLastMessage,
    regenerateResponse,
    requestNotificationPermission,
  };
}

export const appStore = createRoot(createAppStore);
