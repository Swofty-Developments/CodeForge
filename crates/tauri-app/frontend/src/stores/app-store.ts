import { createRoot } from "solid-js";
import { createStore } from "solid-js/store";
import type { Project, Thread, ChatMessage, RunState, LifecycleState, AgentEventPayload, Attachment, ContentBlock, ThreadTokenUsage } from "../types";
import * as ipc from "../ipc";

export interface WindowState {
  openTabs: string[];
  activeTab: string | null;
  sidebarWidth: number;
  selectedProvider: string;
  selectedModel: string | null;
  autoAcceptEnabled: boolean;
}

const WINDOW_STATE_KEY = "window_state";
const PERSIST_DEBOUNCE_MS = 500;

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
  /**
   * Agent run state per thread (idle/generating/error/...). Orthogonal to
   * `lifecycleStates`. Never contains PR-lifecycle values.
   */
  runStates: Record<string, RunState>;
  /**
   * Lifecycle state per thread — the worktree / PR relationship. The
   * backend's `get_pr_status` reconciler is the single source of truth;
   * `pollPrStatuses` stores whatever it returns here verbatim. Also hydrated
   * on cold start from the worktree row's cached fields.
   */
  lifecycleStates: Record<string, LifecycleState>;
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
  themeOpen: boolean;
  worktrees: Record<string, { thread_id: string; branch: string; path: string; active: boolean } | undefined>;
  splitTab: string | null;
  threadDiffOpen: Record<string, boolean>;
  selectedModel: string | null;
  threadBrowserOpen: Record<string, boolean>;
  threadBrowserUrls: Record<string, string>;
  autoNamingEnabled: boolean;
  namingInProgress: Record<string, boolean>;
  attachments: Attachment[];
  threadTokenUsage: Record<string, ThreadTokenUsage>;
  notificationsEnabled: boolean;
  autoAcceptEnabled: boolean;
  threadMessagesLoading: Record<string, boolean>;
  projectGitStatus: Record<string, "none" | "git" | "github">;
  projectPrMap: Record<string, Record<string, number>>;
  activeModel: string | null;
  unreadTabs: Record<string, boolean>;
  availableSlashCommands: string[];
  recentlyClosedTabs: string[];
  keyboardHelpOpen: boolean;
  threadPrStatus: Record<string, import("../ipc").PrStatus>;
  worktreeHealth: Record<string, string>;  // thread_id -> "healthy" | "missing" | "orphaned" | "detached_head"
  turnCheckpoints: Record<string, Record<string, string>>;  // thread_id -> { turn_id: commit_sha }
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
    runStates: {},
    lifecycleStates: {},
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
    themeOpen: false,
    worktrees: {},
    splitTab: null,
    threadDiffOpen: {},
    selectedModel: null,
    threadBrowserOpen: {},
    threadBrowserUrls: {},
    autoNamingEnabled: true,
    namingInProgress: {},
    attachments: [],
    threadTokenUsage: {},
    notificationsEnabled: true,
    autoAcceptEnabled: false,
    threadMessagesLoading: {},
    projectGitStatus: {},
    projectPrMap: {},
    activeModel: null,
    unreadTabs: {},
    availableSlashCommands: [],
    recentlyClosedTabs: [],
    keyboardHelpOpen: false,
    threadPrStatus: {},
    worktreeHealth: {},
    turnCheckpoints: {},
  });

  // ── Window state persistence (debounced) ──────────────────────────
  let persistTimer: ReturnType<typeof setTimeout> | null = null;

  function persistState() {
    if (persistTimer) clearTimeout(persistTimer);
    persistTimer = setTimeout(() => {
      const state: WindowState = {
        openTabs: store.openTabs.filter((t) => !t.startsWith("opt-")),
        activeTab: store.activeTab?.startsWith("opt-") ? null : store.activeTab,
        sidebarWidth: store.sidebarWidth,
        selectedProvider: store.selectedProvider,
        selectedModel: store.selectedModel,
        autoAcceptEnabled: store.autoAcceptEnabled,
      };
      ipc.setSetting(WINDOW_STATE_KEY, JSON.stringify(state)).catch((e) =>
        console.error("Failed to persist window state:", e)
      );
    }, PERSIST_DEBOUNCE_MS);
  }

  async function restoreState() {
    try {
      const raw = await ipc.getSetting(WINDOW_STATE_KEY);
      if (!raw) return;
      const state: Partial<WindowState> = JSON.parse(raw);

      if (state.sidebarWidth != null && state.sidebarWidth >= 150 && state.sidebarWidth <= 600) {
        setStore("sidebarWidth", state.sidebarWidth);
      }
      if (state.selectedProvider) {
        setStore("selectedProvider", state.selectedProvider);
      }
      if (state.selectedModel !== undefined) {
        setStore("selectedModel", state.selectedModel);
      }
      if (state.autoAcceptEnabled != null) {
        setStore("autoAcceptEnabled", state.autoAcceptEnabled);
      }

      // Restore tabs — only keep IDs that correspond to existing threads or virtual tabs
      if (state.openTabs && state.openTabs.length > 0) {
        const allThreadIds = new Set(
          store.projects.flatMap((p) => p.threads.map((t) => t.id))
        );
        const validTabs = state.openTabs.filter(
          (id) => id.startsWith("__") || allThreadIds.has(id)
        );
        if (validTabs.length > 0) {
          setStore("openTabs", validTabs);
          const activeValid = state.activeTab && validTabs.includes(state.activeTab);
          setStore("activeTab", activeValid ? state.activeTab! : validTabs[validTabs.length - 1]);

          // Load messages for all restored tabs in parallel
          const threadTabs = validTabs.filter((id) => !id.startsWith("__"));
          await Promise.all(threadTabs.map((id) => loadThreadMessages(id)));
        }
      }
    } catch (e) {
      console.error("Failed to restore window state:", e);
    }
  }

  async function loadData() {
    try {
      const rawProjects = await ipc.getAllProjects();

      // Show sidebar skeleton immediately with empty threads
      setStore("projects", rawProjects.map((p) => ({
        ...p, color: null, collapsed: false, threads: [],
      })));

      // Fetch all threads + colors in parallel (not sequentially!)
      const [allThreads, colorBatch] = await Promise.all([
        Promise.all(rawProjects.map((p) => ipc.getThreadsByProject(p.id))),
        ipc.getSettingsBatch(rawProjects.map((p) => `project_color:${p.id}`)),
      ]);

      const projects: Project[] = rawProjects.map((p, i) => ({
        ...p,
        color: colorBatch[`project_color:${p.id}`] || null,
        collapsed: false,
        threads: allThreads[i].map((t) => ({ ...t })),
      }));

      setStore("projects", projects);

      // Restore persisted window state after projects are loaded (don't block)
      restoreState().catch((e) => console.error("Failed to restore state:", e));
    } catch (e) {
      console.error("Failed to load data:", e);
    }
  }

  async function loadThreadMessages(threadId: string) {
    if (store.threadMessages[threadId]) return;
    try {
      setStore("threadMessagesLoading", threadId, true);
      // Load last 100 messages initially for fast thread switching
      const msgs = await ipc.getMessagesByThread(threadId, 100);
      setStore("threadMessages", threadId, msgs);
    } catch (e) {
      console.error("Failed to load messages:", e);
    } finally {
      setStore("threadMessagesLoading", threadId, false);
    }
  }

  async function addProject() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, title: "Select a project folder" });
      if (!selected) return;
      const path = selected as string;

      // Check if project already exists for this path
      const existing = store.projects.find((p) => p.path === path);
      if (existing) {
        // Just create a new thread in the existing project
        await newThread(existing.id);
        return;
      }

      const dirName = path.split("/").pop() || path;
      const created = await ipc.createProject(dirName, path);
      const newProject: Project = { ...created, color: null, collapsed: false, threads: [] };
      setStore("projects", (prev) => [...prev, newProject]);
      await newThread(created.id);
    } catch (e) {
      console.error("Failed to add project:", e);
    }
  }

  async function newThread(projectId?: string): Promise<string | null> {
    // Optimistic: create a placeholder thread instantly in the UI
    const optimisticId = `opt-${crypto.randomUUID()}`;
    let targetProjectId = projectId;

    if (!targetProjectId) {
      const uncat = store.projects.find((p) => p.path === ".");
      if (uncat) {
        targetProjectId = uncat.id;
      } else {
        // Create uncategorized project — can't be optimistic here
        try {
          const created = await ipc.createProject("Uncategorized", ".");
          const newProject: Project = { ...created, color: null, collapsed: false, threads: [] };
          setStore("projects", (prev) => [...prev, newProject]);
          targetProjectId = created.id;
        } catch (e) {
          console.error("Failed to create project:", e);
          return null;
        }
      }
    }

    const count = store.projects.reduce((n, p) => n + p.threads.length, 0);
    const optimisticThread = {
      id: optimisticId,
      project_id: targetProjectId!,
      title: `Thread ${count + 1}`,
      color: null,
    };

    // Instantly update UI
    setStore("projects", (projects) =>
      projects.map((p) =>
        p.id === targetProjectId
          ? { ...p, threads: [...p.threads, optimisticThread], collapsed: false }
          : p
      )
    );
    setStore("openTabs", (tabs) => [...tabs, optimisticId]);
    setStore("activeTab", optimisticId);
    setStore("threadMessages", optimisticId, []);

    // Create for real in the background and swap the ID
    try {
      const thread = await ipc.createThread(targetProjectId!, optimisticThread.title, store.selectedProvider);
      // Swap optimistic ID with real ID
      setStore("projects", (projects) =>
        projects.map((p) => ({
          ...p,
          threads: p.threads.map((t) => t.id === optimisticId ? { ...t, id: thread.id } : t),
        }))
      );
      setStore("openTabs", (tabs) => tabs.map((t) => t === optimisticId ? thread.id : t));
      if (store.activeTab === optimisticId) setStore("activeTab", thread.id);
      const msgs = store.threadMessages[optimisticId];
      if (msgs) {
        setStore("threadMessages", thread.id, msgs);
        setStore("threadMessages", optimisticId, undefined as any);
      }
      persistState();
      return thread.id;
    } catch (e) {
      console.error("Failed to create thread:", e);
      return null;
    }
  }

  function setSplitTab(id: string | null) {
    setStore("splitTab", id);
    if (id) loadThreadMessages(id);
  }

  function openVirtualTab(id: string) {
    if (!store.openTabs.includes(id)) {
      setStore("openTabs", (tabs) => [...tabs, id]);
    }
    setStore("activeTab", id);
    persistState();
  }

  function selectThread(id: string) {
    if (!store.openTabs.includes(id)) {
      setStore("openTabs", (tabs) => [...tabs, id]);
    }
    setStore("activeTab", id);
    setStore("unreadTabs", id, false);
    if (!id.startsWith("__")) {
      loadThreadMessages(id);
    }
    persistState();
  }

  function closeTab(id: string) {
    const remaining = store.openTabs.filter((t) => t !== id);
    setStore("openTabs", remaining);
    if (store.activeTab === id) {
      setStore("activeTab", remaining.length > 0 ? remaining[remaining.length - 1] : null);
    }
    persistState();
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
    persistState();
  }

  /** Map slash commands to natural language prompts the agent can act on. */
  function resolveSlashCommand(input: string): string {
    if (!input.startsWith("/")) return input;
    const cmdName = input.split(" ")[0];
    const cmdArgs = input.slice(cmdName.length).trim();
    const mappings: Record<string, string> = {
      "/commit": "Create a git commit with a descriptive message for the current changes",
      "/review-pr": "Review the current pull request",
      "/review": "Review the current pull request or code changes",
      "/compact": "Summarize the conversation so far to save context",
      "/help": "Show what you can help with",
      "/fix": "Fix the issues in the current code",
      "/test": "Run the tests and fix any failures",
      "/lint": "Run the linter and fix any issues",
      "/refactor": "Refactor the current code for better readability and maintainability",
      "/cost": "Show current token usage and cost for this session",
      "/pr-comments": "Review and respond to PR comments",
      "/release-notes": "Generate release notes for recent changes",
      "/security-review": "Perform a security review of the codebase",
      "/simplify": "Review changed code for reuse, quality, and efficiency, then fix any issues",
    };
    const mapped = mappings[cmdName];
    if (mapped) {
      return cmdArgs ? `${mapped}: ${cmdArgs}` : mapped;
    }
    // For unknown commands, convert the skill name to a natural language instruction
    // e.g. "/frontend-design build a landing page" → "Use the frontend-design skill to: build a landing page"
    const skillName = cmdName.slice(1); // remove leading /
    if (cmdArgs) {
      return `Use the ${skillName} skill/approach to: ${cmdArgs}`;
    }
    return `Execute the ${skillName} skill/command`;
  }

  async function sendUserMessage() {
    let text = store.composerText.trim();
    const atts = [...store.attachments];
    if ((!text && atts.length === 0) || !store.activeTab) return;

    // Resolve slash commands to natural language prompts
    text = resolveSlashCommand(text);

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

      // Show message in UI immediately with optimistic ID
      const optimisticMsgId = `msg-${crypto.randomUUID()}`;
      const userMsg: ChatMessage = { id: optimisticMsgId, thread_id: threadId, role: "user", content: fullText };
      setStore("threadMessages", threadId, (msgs) => [...(msgs || []), userMsg]);

      // Persist in background — don't block the send
      ipc.persistUserMessage(threadId, fullText).catch((e) =>
        console.error("Failed to persist message:", e)
      );

      const project = store.projects.find((p) => p.threads.some((t) => t.id === threadId));
      let wt = store.worktrees[threadId];

      // Auto-create worktree for project threads that don't have one yet
      if (project && project.path !== "." && !wt?.active) {
        try {
          // Check git status first — only create worktree if repo exists
          const repoStatus = await ipc.gitRepoStatus(project.path);

          if (repoStatus.status !== "none") {
            // It's a git repo — create worktree
            const thread = project.threads.find((t) => t.id === threadId);
            if (thread) {
              const newWt = await ipc.createWorktree(threadId, thread.title, project.path, project.id);
              setStore("worktrees", threadId, newWt);
              wt = newWt;
            }
            // Update cached git status if it changed
            if (store.projectGitStatus[project.id] !== repoStatus.status) {
              setStore("projectGitStatus", project.id, repoStatus.status as any);
            }
          } else {
            // Not a git repo — update status so the migration banner shows
            setStore("projectGitStatus", project.id, "none");
          }
        } catch (e) {
          console.error("Failed to check repo / create worktree:", e);
        }
      }

      const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

      // Inject worktree context on first message so the AI understands the environment
      let sendText = text;
      const existingMsgs = store.threadMessages[threadId] || [];
      const isFirstUserMsg = existingMsgs.filter((m) => m.role === "user").length <= 1;
      if (isFirstUserMsg && wt?.active && project) {
        const prMap = store.projectPrMap[project.id];
        const prNum = prMap?.[threadId];
        let ctx = `[Context: You are working in a git worktree at \`${wt.path}\` on branch \`${wt.branch}\`. `;
        ctx += `The main project is at \`${project.path}\`. `;
        if (prNum) {
          ctx += `This worktree is linked to PR #${prNum}. `;
          ctx += `Changes you make here will be pushed to that PR's branch when the user clicks "Push to PR". `;
        } else {
          ctx += `Changes here are isolated from the main branch and will be merged back when the user clicks "Merge back to main". `;
        }
        ctx += `All file operations should use the worktree path, not the main project path.]`;
        sendText = ctx + "\n\n" + text;
      }

      setStore("runStates", threadId, "generating");
      await ipc.sendMessage(threadId, sendText, store.selectedProvider, cwd, store.selectedModel ?? undefined);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (msgs) => [...(msgs || []), errMsg]);
      setStore("runStates", threadId, "error");
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
        if (thread_id !== store.activeTab) {
          setStore("unreadTabs", thread_id, true);
        }
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
        setStore("runStates", thread_id, "generating");
        break;
      case "turn_completed": {
        setStore("runStates", thread_id, "ready");
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
        // Mark unread for background threads
        if (thread_id !== store.activeTab) {
          setStore("unreadTabs", thread_id, true);
        }
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
        setStore("runStates", thread_id, "ready");
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
        // Don't reset to "ready" if we're mid-generation —
        // session_ready fires when the sidecar initializes but before the response
        if (store.runStates[thread_id] !== "generating") {
          setStore("runStates", thread_id, "ready");
        }
        // Capture the confirmed model from the SDK
        if (payload.model) {
          setStore("activeModel", payload.model);
        }
        break;
      case "session_error": {
        // Check if this is a slash_commands update (smuggled via session_error)
        if (payload.message?.startsWith("slash_commands:")) {
          const cmdList = payload.message.slice("slash_commands:".length).split(",").filter(Boolean);
          setStore("availableSlashCommands", cmdList);
          break;
        }
        setStore("runStates", thread_id, "error");
        setStore("threadMessages", thread_id, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id, role: "system" as const, content: `Error: ${payload.message}` },
        ]);
        const errTitle = getThreadTitle(thread_id);
        sendNotification(`${errTitle} - Error`, payload.message || "Session error");
        break;
      }
      case "approval_required":
        if (payload.request_id && payload.description) {
          // Auto-accept if enabled
          if (store.autoAcceptEnabled) {
            ipc.respondToApproval(payload.session_id, payload.request_id!, true).catch(() => {});
          } else {
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
      setStore("runStates", threadId, "generating");
      await ipc.sendMessage(threadId, userText, store.selectedProvider, cwd, store.selectedModel ?? undefined);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (m) => [...(m || []), errMsg]);
      setStore("runStates", threadId, "error");
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
      setStore("runStates", threadId, "generating");
      await ipc.sendMessage(threadId, userText, store.selectedProvider, cwd, store.selectedModel ?? undefined);
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (m) => [...(m || []), errMsg]);
      setStore("runStates", threadId, "error");
    }
  }

  async function loadProjectGitStatus(projectId: string, projectPath: string, threads: { id: string }[], force?: boolean) {
    if (!force && store.projectGitStatus[projectId] !== undefined) return;
    if (projectPath === ".") return;

    try {
      const repoStatus = await ipc.gitRepoStatus(projectPath);
      setStore("projectGitStatus", projectId, repoStatus.status as any);
    } catch {
      setStore("projectGitStatus", projectId, "none");
    }

    const status = store.projectGitStatus[projectId];
    if (status && status !== "none" && threads.length > 0) {
      const map: Record<string, number> = {};
      try {
        // Load worktree info (including PR numbers + status) from the worktrees table.
        // The DB is the source of truth — status field tells us active/merged/closed/deleted/orphaned.
        // We derive a full `LifecycleState` from the cached worktree row so the
        // UI is correct on first paint, before the poller runs.
        for (const t of threads) {
          const wt = await ipc.getWorktree(t.id);
          if (wt) {
            setStore("worktrees", t.id, wt);
            if (wt.pr_number) {
              map[t.id] = wt.pr_number;
            }
            const lifecycle = deriveLifecycleFromWorktree(wt);
            setStore("lifecycleStates", t.id, lifecycle);
          }
        }
      } catch {}
      if (Object.keys(map).length > 0) {
        setStore("projectPrMap", projectId, map);
      }
    }
  }

  /**
   * Derive an initial `LifecycleState` from a worktree row's cached fields.
   * Used for cold-start hydration — no GitHub round-trip. The poller will
   * overwrite this with a fresher value on the next tick.
   */
  function deriveLifecycleFromWorktree(wt: import("../ipc").WorktreeInfo): import("../types").LifecycleState {
    const mkSnap = (state: string): import("../types").PrSnapshot => ({
      number: wt.pr_number ?? 0,
      url: wt.pr_url ?? "",
      state,
    });
    switch (wt.status) {
      case "merged":
        return { kind: "pr_merged", pr: mkSnap("merged"), merge_commit: wt.pr_merge_commit ?? "" };
      case "closed":
        return { kind: "pr_closed", pr: mkSnap("closed") };
      case "orphaned":
        return { kind: "worktree_orphaned", branch: wt.branch, path: wt.path };
      case "deleted":
        return { kind: "working" };
      case "active":
      default:
        if (wt.pr_number) {
          const snap = mkSnap(wt.pr_state ?? "open");
          return { kind: "pr_open", pr: snap, ci: "none", review: "none", unread_comments: 0 };
        }
        return { kind: "working" };
    }
  }

  /**
   * Poll GitHub for PR status across all threads. Skips locked lifecycles
   * (merged/closed/orphaned) to avoid wasted API calls. Parallelizes within
   * a project via `Promise.allSettled`. Uses the backend reconciler
   * (`ipc.getPrStatus`) as the single source of truth — everything it
   * returns is stored verbatim on the thread's lifecycle, and transition
   * flags (`reopen_detected`, `revert_detected`, `pr_missing`) trigger
   * one-shot event messages in the chat scroll.
   */
  async function pollPrStatuses() {
    for (const project of store.projects) {
      if (project.path === ".") continue;

      // Source of truth is the worktree record, not the projectPrMap mirror.
      const threadIds: string[] = [];
      for (const thread of project.threads) {
        const wt = store.worktrees[thread.id];
        if (!wt || !wt.pr_number) continue;
        // Skip terminal / unreachable lifecycles
        const lc = store.lifecycleStates[thread.id];
        if (lc?.kind === "pr_merged" || lc?.kind === "pr_closed") continue;
        if (wt.status === "deleted" || wt.status === "orphaned") continue;
        threadIds.push(thread.id);
      }
      if (threadIds.length === 0) continue;

      // Parallelize within the project
      const results = await Promise.allSettled(
        threadIds.map((tid) => ipc.getPrStatus(tid, project.path).then((s) => ({ tid, s })))
      );

      for (const r of results) {
        if (r.status !== "fulfilled") continue;
        const { tid: threadId, s: status } = r.value;
        if (!status) continue;

        setStore("threadPrStatus", threadId, status);
        // Store the lifecycle verbatim — backend is the single source of truth.
        setStore("lifecycleStates", threadId, status.lifecycle);

        // ── One-shot transition events ──
        // The backend persists the new worktree status; we just surface
        // human-readable events in the chat scroll when things change.
        if (status.pr_missing) {
          pushSystemEvent(threadId, "warn",
            `PR #${status.pr_number} no longer exists on GitHub. Link cleared.`);
          // Reset worktree mirror to match the cleared DB state
          setStore("worktrees", threadId, (wt: any) =>
            wt ? { ...wt, pr_number: null, pr_state: null, pr_merge_commit: null, pr_url: null } : wt);
        } else if (status.reopen_detected) {
          pushSystemEvent(threadId, "info",
            `PR #${status.pr_number} was reopened — thread is editable again.`);
          setStore("worktrees", threadId, (wt: any) =>
            wt ? { ...wt, status: "active", active: true, pr_state: "open" } : wt);
        } else if (status.revert_detected) {
          pushSystemEvent(threadId, "warn",
            `PR #${status.pr_number}'s merge commit is no longer reachable on the base branch — it looks reverted. You can continue working in this thread.`);
        } else if (status.previous_status === "active" && status.lifecycle.kind === "pr_merged") {
          pushSystemEvent(threadId, "info",
            `PR #${status.pr_number} was merged on GitHub. This thread is now read-only.`);
          setStore("worktrees", threadId, (wt: any) =>
            wt ? { ...wt, status: "merged", active: false } : wt);
        } else if (status.previous_status === "active" && status.lifecycle.kind === "pr_closed") {
          pushSystemEvent(threadId, "warn",
            `PR #${status.pr_number} was closed without merging. This thread is now read-only.`);
          setStore("worktrees", threadId, (wt: any) =>
            wt ? { ...wt, status: "closed", active: false } : wt);
        }

        // New review comments ≥ 1 since last poll
        if (status.new_comment_count > 0) {
          // Fetch only the new ones and append them as review-kind messages.
          try {
            const all = await ipc.getPrReviewComments(threadId, project.path);
            const toShow = all.slice(-status.new_comment_count);
            for (const c of toShow) {
              if (!c.body?.trim()) continue;
              pushSystemEvent(threadId, "review",
                `**@${c.author}** (${c.state}):\n\n${c.body}`);
            }
            if (store.activeTab !== threadId && toShow.length > 0) {
              setStore("unreadTabs", threadId, true);
            }
          } catch {}
        }
      }
    }
  }

  /** Append a typed system event to the thread's chat scroll. */
  function pushSystemEvent(
    threadId: string,
    kind: "info" | "warn" | "error" | "review",
    content: string,
  ) {
    setStore("threadMessages", threadId, (msgs) => [
      ...(msgs || []),
      {
        id: `sys-${crypto.randomUUID()}`,
        thread_id: threadId,
        role: "system" as const,
        content,
        system_kind: kind,
      } as any,
    ]);
  }

  // PR status polling runs from App.tsx on a 60s interval.

  return {
    store,
    setStore,
    loadData,
    loadThreadMessages,
    newThread,
    addProject,
    selectThread,
    openVirtualTab,
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
    loadProjectGitStatus,
    persistState,
    pollPrStatuses,
  };
}

export const appStore = createRoot(createAppStore);
