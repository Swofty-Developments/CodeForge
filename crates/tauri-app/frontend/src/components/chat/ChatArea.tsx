import { For, Show, createEffect, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { Markdown } from "./Markdown";
import { ToolUseCard } from "./ToolUseCard";
import { ThinkingBlock } from "./ThinkingBlock";
import { ThreadSetup } from "./ThreadSetup";
import { McpPanel } from "../sidebar/McpPanel";
import { ThemeSelector } from "../settings/ThemeSelector";
import { SearchOverlay } from "../shared/SearchOverlay";
import type { ContentBlock } from "../../types";

export function ChatArea() {
  const { store, approveRequest, denyRequest, addProject, newThread } = appStore;
  let scrollRef: HTMLDivElement | undefined;

  const messages = () => {
    if (!store.activeTab) return [];
    return store.threadMessages[store.activeTab] || [];
  };

  const isGenerating = () => {
    if (!store.activeTab) return false;
    return store.sessionStatuses[store.activeTab] === "generating";
  };

  const isLoadingMessages = () => {
    if (!store.activeTab) return false;
    return !!store.threadMessagesLoading[store.activeTab];
  };

  const worktree = () => store.worktrees[store.activeTab || ""] || null;

  // Track whether user has manually scrolled away from the bottom
  let userScrolledAway = false;

  function handleScroll() {
    if (!scrollRef) return;
    const distFromBottom = scrollRef.scrollHeight - scrollRef.scrollTop - scrollRef.clientHeight;
    userScrolledAway = distFromBottom > 150;
  }

  // Track content of the last message to trigger scroll during streaming
  const lastMsgContent = () => {
    const msgs = messages();
    if (msgs.length === 0) return 0;
    const last = msgs[msgs.length - 1];
    return last.content?.length || 0;
  };

  // Reset scroll lock when message count changes (new user/assistant message)
  let prevMsgCount = 0;
  createEffect(() => {
    const len = messages().length;
    const _g = isGenerating();
    const _c = lastMsgContent();
    if (len !== prevMsgCount) {
      userScrolledAway = false;
      prevMsgCount = len;
    }
    if (!userScrolledAway) {
      requestAnimationFrame(() => {
        if (scrollRef) {
          scrollRef.scrollTo({ top: scrollRef.scrollHeight, behavior: "smooth" });
        }
      });
    }
  });

  createEffect(() => {
    const tab = store.activeTab;
    if (tab && !store.worktrees[tab]) {
      ipc.getWorktree(tab).then((wt) => {
        if (wt) appStore.setStore("worktrees", tab, wt);
      });
    }
  });

  async function handleCreateWorktree() {
    const tab = store.activeTab;
    if (!tab) return;
    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === tab);
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!thread || !project || project.path === ".") return;
    try {
      const wt = await ipc.createWorktree(tab, thread.title, project.path);
      appStore.setStore("worktrees", tab, wt);
    } catch (e) {
      console.error("Failed to create worktree:", e);
    }
  }

  async function handleMergeWorktree() {
    const tab = store.activeTab;
    if (!tab) return;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!project) return;
    try {
      const msg = await ipc.mergeWorktree(tab, project.path);
      appStore.setStore("worktrees", tab, undefined as any);
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: msg },
      ]);
    } catch (e) {
      // Auto-prompt the AI to help resolve the conflict
      const errorMsg = String(e);
      const prompt = `My merge/push just failed with this error:\n\n\`\`\`\n${errorMsg}\n\`\`\`\n\nPlease help me resolve this. Check the git status, identify any conflicts, and fix them.`;

      try {
        const msgId = await ipc.persistUserMessage(tab, prompt);
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: msgId, thread_id: tab, role: "user" as const, content: prompt },
        ]);

        const wt = store.worktrees[tab];
        const cwd = wt?.active ? wt.path : (project && project.path !== "." ? project.path : ".");

        appStore.setStore("sessionStatuses", tab, "generating");
        await ipc.sendMessage(tab, prompt, store.selectedProvider, cwd, store.selectedModel ?? undefined);
      } catch (sendErr) {
        // If sending fails too, just show the error
        appStore.setStore("threadMessages", tab, (msgs) => [
          ...(msgs || []),
          { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: `Merge failed: ${errorMsg}` },
        ]);
      }
    }
  }

  const hasFolder = () => {
    if (!store.activeTab) return false;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === store.activeTab));
    return project && project.path !== ".";
  };

  const activeProject = () => {
    if (!store.activeTab) return null;
    return store.projects.find((p) => p.threads.some((t) => t.id === store.activeTab)) || null;
  };

  const isGitProject = () => {
    const proj = activeProject();
    return proj ? proj.path !== "." : false;
  };

  function setSuggestion(text: string) {
    appStore.setStore("composerText", text);
  }

  const isVirtualTab = () => store.activeTab?.startsWith("__") || false;

  return (
    <div class="chat-area" ref={scrollRef} onScroll={handleScroll}>
      {/* Virtual tab content */}
      <Show when={store.activeTab === "__mcp__"}>
        <div class="virtual-tab-content">
          <McpPanel />
        </div>
      </Show>
      <Show when={store.activeTab === "__themes__"}>
        <div class="virtual-tab-content">
          <ThemeSelector inline />
        </div>
      </Show>
      <Show when={store.activeTab === "__search__"}>
        <div class="virtual-tab-content">
          <SearchOverlay inline />
        </div>
      </Show>

      {/* Regular chat content */}
      <Show when={!isVirtualTab()}>
      <Show
        when={store.activeTab}
        fallback={
          <Show when={store.projects.filter((p) => p.path !== ".").length > 0} fallback={
            <div class="chat-empty">
              <div class="onboarding-card">
                <div class="onboarding-icon">
                  <svg width="48" height="48" viewBox="0 0 24 24" fill="none" stroke="url(#og1)" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                    <line x1="12" y1="11" x2="12" y2="17" /><line x1="9" y1="14" x2="15" y2="14" />
                    <defs>
                      <linearGradient id="og1" x1="2" y1="3" x2="22" y2="21" gradientUnits="userSpaceOnUse">
                        <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--purple)" />
                      </linearGradient>
                    </defs>
                  </svg>
                </div>
                <h2 class="onboarding-title">Add your first project</h2>
                <p class="onboarding-desc">Point CodeForge to a project folder to get started</p>
                <button class="onboarding-btn" onClick={() => addProject()}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
                    <line x1="12" y1="11" x2="12" y2="17" /><line x1="9" y1="14" x2="15" y2="14" />
                  </svg>
                  Choose a folder
                </button>
                <button class="onboarding-link" onClick={() => newThread()}>
                  Or start without a project
                </button>
              </div>
              <div class="shortcut-hints" style={{ "margin-top": "24px" }}>
                <span class="kbd-hint"><kbd>&#8984;K</kbd> Command palette</span>
                <span class="kbd-hint"><kbd>&#8984;N</kbd> New thread</span>
              </div>
            </div>
          }>
            <div class="chat-empty">
              <div class="hero-mark">
                <svg width="40" height="40" viewBox="0 0 56 56" fill="none">
                  <path d="M12 10L6 28L12 46" stroke="url(#hg1)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                  <path d="M44 10L50 28L44 46" stroke="url(#hg1)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" />
                  <path d="M33 8L23 48" stroke="url(#hg2)" stroke-width="2" stroke-linecap="round" />
                  <defs>
                    <linearGradient id="hg1" x1="6" y1="10" x2="50" y2="46" gradientUnits="userSpaceOnUse">
                      <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--purple)" />
                    </linearGradient>
                    <linearGradient id="hg2" x1="23" y1="48" x2="33" y2="8" gradientUnits="userSpaceOnUse">
                      <stop stop-color="var(--primary)" /><stop offset="1" stop-color="var(--pink)" />
                    </linearGradient>
                  </defs>
                </svg>
              </div>
              <h2 class="hero-title">CodeForge</h2>
              <p class="hero-subtitle">Select a thread from the sidebar, or create a new one</p>
              <div class="hero-quick-actions">
                <For each={store.projects.filter((p) => p.path !== ".").slice(0, 3)}>
                  {(project) => (
                    <button class="hero-quick-btn" onClick={() => newThread(project.id)}>
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                        <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
                      </svg>
                      New Thread in {project.name}
                    </button>
                  )}
                </For>
              </div>
              <div class="shortcut-hints">
                <span class="kbd-hint"><kbd>&#8984;K</kbd> Command palette</span>
                <span class="kbd-hint"><kbd>&#8984;N</kbd> New thread</span>
                <span class="kbd-hint"><kbd>&#8984;O</kbd> Add project</span>
              </div>
            </div>
          </Show>
        }
      >
        {/* Worktree banner — only shown when a worktree is active and messages exist */}
        <Show when={isGitProject() && worktree() && messages().length > 0}>
          <div class="worktree-banner">
            <div class="wt-info">
              <span class="wt-branch">{worktree()!.branch}</span>
              <span class="wt-path">{worktree()!.path}</span>
            </div>
            <button class="wt-merge-btn" onClick={handleMergeWorktree}>
              {(() => {
                const proj = activeProject();
                const tab = store.activeTab;
                if (proj && tab) {
                  const prMap = store.projectPrMap[proj.id];
                  if (prMap?.[tab]) return `Push to PR #${prMap[tab]}`;
                }
                return "Merge back to main";
              })()}
            </button>
          </div>
        </Show>

        {/* Loading skeleton while messages are being fetched */}
        <Show when={isLoadingMessages()}>
          <div class="messages-loading">
            <div class="msg-skeleton">
              <div class="skeleton-line wide" />
              <div class="skeleton-line" />
            </div>
            <div class="msg-skeleton assistant">
              <div class="skeleton-line wide" />
              <div class="skeleton-line medium" />
              <div class="skeleton-line" />
            </div>
          </div>
        </Show>

        <Show
          when={(messages().length > 0 || isGenerating()) && !isLoadingMessages()}
          fallback={
            <Show when={isGitProject() && activeProject()} fallback={
              <div class="chat-empty">
                <p class="new-convo">New conversation</p>
                <p class="provider-hint">
                  Using {store.selectedProvider === "claude_code" ? "Claude Code" : "Codex"}
                </p>
                <div class="suggestion-chips">
                  <button class="suggestion-chip" onClick={() => setSuggestion("Explain this codebase and its architecture")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" /><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" /></svg>
                    Explain codebase
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Help me fix a bug in ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                    Fix a bug
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Write tests for ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="9 11 12 14 22 4" /><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11" /></svg>
                    Write tests
                  </button>
                  <button class="suggestion-chip" onClick={() => setSuggestion("Refactor this code to be cleaner: ")}>
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>
                    Refactor code
                  </button>
                </div>
              </div>
            }>
              <ThreadSetup
                projectId={activeProject()!.id}
                repoPath={activeProject()!.path}
                threadId={store.activeTab!}
              />
            </Show>
          }
        >
          <div class="messages-container">
            <For each={messages()}>
              {(msg, idx) => (
                <MessageBubble
                  msg={msg}
                  isGenerating={isGenerating()}
                  isLast={idx() === messages().length - 1}
                  isLastUser={msg.role === "user" && (() => {
                    const msgs = messages();
                    for (let i = msgs.length - 1; i >= 0; i--) {
                      if (msgs[i].role === "user") return msgs[i].id === msg.id;
                    }
                    return false;
                  })()}
                  threadId={store.activeTab!}
                />
              )}
            </For>

            {/* Context-aware suggestion chips for freshly-setup threads */}
            <Show when={messages().length === 1 && !isGenerating() && isGitProject()}>
              {(() => {
                const proj = activeProject();
                const tab = store.activeTab;
                const isPrThread = proj && tab && store.projectPrMap[proj.id]?.[tab];
                return (
                  <div class="suggestion-chips" style="padding: 8px 0 16px;">
                    <Show when={isPrThread}>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Review this PR and identify any issues, bugs, or improvements")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M13 6h3a2 2 0 012 2v7" /><line x1="6" y1="9" x2="6" y2="21" /></svg>
                        Review this PR
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Summarize all changes in this PR")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M14 2H6a2 2 0 00-2 2v16a2 2 0 002 2h12a2 2 0 002-2V8z" /><polyline points="14 2 14 8 20 8" /><line x1="16" y1="13" x2="8" y2="13" /><line x1="16" y1="17" x2="8" y2="17" /></svg>
                        Summarize changes
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Check this PR for potential bugs, security issues, or performance problems")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                        Check for issues
                      </button>
                    </Show>
                    <Show when={!isPrThread}>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Review my uncommitted changes and suggest improvements")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M1 12s4-8 11-8 11 8 11 8-4 8-11 8-11-8-11-8z" /><circle cx="12" cy="12" r="3" /></svg>
                        Review my changes
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Help me commit my current changes with a good commit message")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="20 6 9 17 4 12" /></svg>
                        Help me commit
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Explain this codebase and its architecture")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M2 3h6a4 4 0 014 4v14a3 3 0 00-3-3H2z" /><path d="M22 3h-6a4 4 0 00-4 4v14a3 3 0 013-3h7z" /></svg>
                        Explain codebase
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Find potential bugs or issues in this codebase")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10" /><line x1="12" y1="8" x2="12" y2="12" /><line x1="12" y1="16" x2="12.01" y2="16" /></svg>
                        Find bugs
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Write tests for ")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="9 11 12 14 22 4" /><path d="M21 12v7a2 2 0 01-2 2H5a2 2 0 01-2-2V5a2 2 0 012-2h11" /></svg>
                        Write tests
                      </button>
                      <button class="suggestion-chip" onClick={() => setSuggestion("Refactor this code to be cleaner: ")}>
                        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><polyline points="16 18 22 12 16 6" /><polyline points="8 6 2 12 8 18" /></svg>
                        Refactor
                      </button>
                    </Show>
                  </div>
                );
              })()}
            </Show>

            <For each={store.pendingApprovals.filter((a) => a.threadId === store.activeTab)}>
              {(approval) => {
                const toolName = () => approval.description.split(":")[0]?.trim() || "tool";

                function handleAutoAccept() {
                  // Set permission mode to bypass and approve ALL pending
                  ipc.setSetting("permission_mode", "bypassPermissions").catch(() => {});
                  appStore.setStore("autoAcceptEnabled", true);
                  appStore.persistState();
                  // Approve all pending approvals for this thread
                  const pending = [...store.pendingApprovals.filter((a) => a.threadId === approval.threadId)];
                  for (const a of pending) {
                    approveRequest(a);
                  }
                }

                return (
                  <div class="approval-card">
                    <div class="approval-header">
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--amber)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                        <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                      </svg>
                      <span class="approval-title">Permission Required</span>
                      <span class="approval-tool">{toolName()}</span>
                    </div>
                    <pre class="approval-desc">{approval.description}</pre>
                    <div class="approval-actions">
                      <button class="deny-btn" onClick={() => denyRequest(approval)}>Deny</button>
                      <button class="approve-btn" onClick={() => approveRequest(approval)}>Approve</button>
                      <button class="bypass-btn" onClick={handleAutoAccept} title="Auto-approve everything from now on">
                        Auto Accept
                      </button>
                    </div>
                  </div>
                );
              }}
            </For>

            <Show when={isGenerating() && messages().length === 0}>
              <div class="typing-indicator">
                <span class="dot" /><span class="dot" /><span class="dot" />
              </div>
            </Show>
          </div>
        </Show>
      </Show>
      </Show>
    </div>
  );
}

function renderBlock(block: ContentBlock, isLastAndStreaming: boolean) {
  switch (block.type) {
    case "text":
      return block.content.trim() ? <Markdown content={block.content} /> : null;
    case "tool_use":
      return <ToolUseCard block={block} />;
    case "thinking":
      return <ThinkingBlock content={block.content} streaming={isLastAndStreaming && block.type === "thinking"} />;
    default:
      return null;
  }
}

function MessageBubble(props: { msg: any; isGenerating: boolean; isLast: boolean; isLastUser: boolean; threadId: string }) {
  const [copied, setCopied] = createSignal(false);
  const [detailsHidden, setDetailsHidden] = createSignal(false);
  const isAssistant = () => props.msg.role === "assistant";
  const meta = () => props.msg.meta;
  const hasBlocks = () => props.msg.blocks && props.msg.blocks.length > 0;
  const isStreaming = () => isAssistant() && !props.msg.id.startsWith("done-");
  const isDone = () => isAssistant() && props.msg.id.startsWith("done-");

  // Check if there are non-text blocks (tool use or thinking) worth toggling
  const hasDetails = () => {
    if (!props.msg.blocks) return false;
    return (props.msg.blocks as ContentBlock[]).some((b) => b.type !== "text");
  };

  // Get only text blocks for collapsed view
  const textBlocks = () => {
    if (!props.msg.blocks) return [];
    return (props.msg.blocks as ContentBlock[]).filter((b) => b.type === "text" && b.content.trim());
  };

  // Count details for the toggle label
  const detailCounts = () => {
    if (!props.msg.blocks) return { tools: 0, thinking: 0 };
    const blocks = props.msg.blocks as ContentBlock[];
    return {
      tools: blocks.filter((b) => b.type === "tool_use").length,
      thinking: blocks.filter((b) => b.type === "thinking").length,
    };
  };

  async function copyContent() {
    await navigator.clipboard.writeText(props.msg.content);
    setCopied(true);
    setTimeout(() => setCopied(false), 1500);
  }

  function formatDuration(ms: number) {
    if (ms < 1000) return `${ms}ms`;
    return `${(ms / 1000).toFixed(1)}s`;
  }

  function formatTokens(n: number) {
    if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
    return String(n);
  }

  function formatModel(model: string): string {
    // Shorten long model IDs to friendly names
    if (model.includes("opus")) return "Opus";
    if (model.includes("sonnet")) return "Sonnet";
    if (model.includes("haiku")) return "Haiku";
    return model;
  }

  return (
    <div class={`msg msg-${props.msg.role}`}>
      <Show when={props.msg.role === "system"}>
        <div class="msg-system-pill">{props.msg.content}</div>
      </Show>
      <Show when={props.msg.role === "user"}>
        <div class="msg-user-bubble">{props.msg.content}</div>
        <div class="msg-user-actions">
          <button
            class="msg-action-btn"
            title="Edit & resend"
            onClick={() => appStore.editAndResend(props.threadId, props.msg.id, props.msg.content)}
          >
            <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M11 4H4a2 2 0 00-2 2v14a2 2 0 002 2h14a2 2 0 002-2v-7" />
              <path d="M18.5 2.5a2.121 2.121 0 013 3L12 15l-4 1 1-4 9.5-9.5z" />
            </svg>
          </button>
          <Show when={props.isLastUser && !props.isGenerating}>
            <button
              class="msg-action-btn"
              title="Retry"
              onClick={() => appStore.retryLastMessage(props.threadId)}
            >
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <polyline points="23 4 23 10 17 10" />
                <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
              </svg>
            </button>
          </Show>
        </div>
      </Show>
      <Show when={isAssistant()}>
        <div class="msg-assistant">
          {/* Toggle for hiding details on completed messages */}
          <Show when={isDone() && hasDetails()}>
            <button class="msg-details-toggle" onClick={() => setDetailsHidden(!detailsHidden())}>
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <Show when={detailsHidden()} fallback={<><polyline points="6 9 12 15 18 9" /></>}>
                  <polyline points="9 18 15 12 9 6" />
                </Show>
              </svg>
              <Show when={detailsHidden()} fallback="Hide details">
                {(() => {
                  const c = detailCounts();
                  const parts: string[] = [];
                  if (c.tools > 0) parts.push(`${c.tools} tool${c.tools > 1 ? "s" : ""}`);
                  if (c.thinking > 0) parts.push("thinking");
                  return `Show ${parts.join(" + ")}`;
                })()}
              </Show>
            </button>
          </Show>

          {/* Message content */}
          <div class="msg-body">
            <Show when={hasBlocks()} fallback={<Markdown content={props.msg.content} />}>
              <Show when={detailsHidden()}>
                {/* Collapsed: only text blocks */}
                <For each={textBlocks()}>
                  {(block) => <Markdown content={block.content} />}
                </For>
                <Show when={textBlocks().length === 0}>
                  <span class="msg-no-text">Response contained only tool use</span>
                </Show>
              </Show>
              <Show when={!detailsHidden()}>
                {/* Full: all blocks */}
                <For each={props.msg.blocks as ContentBlock[]}>
                  {(block, idx) => renderBlock(block, isStreaming() && idx() === (props.msg.blocks as ContentBlock[]).length - 1)}
                </For>
              </Show>
            </Show>
          </div>

          {/* Footer: meta + copy */}
          <div class="msg-footer">
            <Show when={meta()}>
              <div class="msg-meta">
                <Show when={meta()!.model && meta()!.model !== "unknown"}>
                  <span class="msg-meta-tag">{formatModel(meta()!.model!)}</span>
                </Show>
                <Show when={meta()!.inputTokens != null || meta()!.outputTokens != null}>
                  <span class="msg-meta-tag">{formatTokens(meta()!.inputTokens || 0)} in / {formatTokens(meta()!.outputTokens || 0)} out</span>
                </Show>
                <Show when={meta()!.durationMs}>
                  <span class="msg-meta-tag">{formatDuration(meta()!.durationMs!)}</span>
                </Show>
                <Show when={meta()!.costUsd != null && meta()!.costUsd! > 0}>
                  <span class="msg-meta-tag">${meta()!.costUsd!.toFixed(4)}</span>
                </Show>
              </div>
            </Show>
            <Show when={props.msg.content}>
              <button class="msg-copy" onClick={copyContent} title={copied() ? "Copied!" : "Copy"}>
                <Show when={copied()} fallback={
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                  </svg>
                }>
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                    <polyline points="20 6 9 17 4 12"/>
                  </svg>
                </Show>
              </button>
            </Show>
            <Show when={isDone() && !props.isGenerating}>
              <button
                class="msg-action-btn"
                title="Regenerate response"
                onClick={() => appStore.regenerateResponse(props.threadId, props.msg.id)}
              >
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="23 4 23 10 17 10" />
                  <path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
                </svg>
              </button>
            </Show>
          </div>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("chat-styles")) {
  const style = document.createElement("style");
  style.id = "chat-styles";
  style.textContent = `
    .chat-area {
      flex: 1;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
    }

    /* ── Virtual tab content ── */
    .virtual-tab-content {
      flex: 1;
      overflow-y: auto;
      padding: 16px;
      animation: fade-in 0.15s ease both;
    }
    .virtual-tab-placeholder {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      font-size: 14px;
    }

    /* ── Empty states ── */
    .chat-empty {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: var(--text-secondary);
      gap: 8px;
      animation: fade-in 0.2s ease both;
    }
    .hero-mark {
      margin-bottom: 8px;
      opacity: 0.5;
    }
    .hero-title {
      font-size: 28px;
      font-weight: 700;
      letter-spacing: -0.8px;
      background: linear-gradient(135deg, var(--text) 30%, var(--primary) 70%, var(--purple) 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    .hero-subtitle {
      font-size: 13px;
      color: var(--text-tertiary);
      margin-bottom: 16px;
    }
    .shortcut-hints {
      display: flex;
      gap: 16px;
    }
    .kbd-hint {
      display: flex;
      align-items: center;
      gap: 6px;
      font-size: 11px;
      color: var(--text-tertiary);
    }
    .kbd-hint kbd {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 2px 6px;
      background: var(--bg-accent);
      border: 1px solid var(--border-strong);
      border-radius: 4px;
      font-size: 10px;
      font-family: var(--font-body);
      color: var(--text-secondary);
      line-height: 1.4;
    }

    /* ── Onboarding card (no projects) ── */
    .onboarding-card {
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 8px;
      padding: 40px 48px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: 16px;
      box-shadow: 0 8px 32px rgba(0,0,0,0.3);
      animation: cmdSlideIn 0.25s cubic-bezier(0.16, 1, 0.3, 1);
    }
    @keyframes cmdSlideIn {
      from { opacity: 0; transform: translateY(-6px) scale(0.98); }
      to { opacity: 1; transform: translateY(0) scale(1); }
    }
    .onboarding-icon {
      margin-bottom: 4px;
      opacity: 0.9;
    }
    .onboarding-title {
      font-size: 22px;
      font-weight: 700;
      letter-spacing: -0.5px;
      background: linear-gradient(135deg, var(--text) 30%, var(--primary) 70%, var(--purple) 100%);
      -webkit-background-clip: text;
      -webkit-text-fill-color: transparent;
      background-clip: text;
    }
    .onboarding-desc {
      font-size: 13px;
      color: var(--text-tertiary);
      margin-bottom: 8px;
    }
    .onboarding-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      gap: 8px;
      padding: 10px 28px;
      font-size: 14px;
      font-weight: 600;
      color: #fff;
      background: var(--primary);
      border-radius: var(--radius-md);
      transition: filter 0.15s, transform 0.1s;
      margin-top: 4px;
    }
    .onboarding-btn:hover { filter: brightness(1.15); }
    .onboarding-btn:active { transform: scale(0.97); }
    .onboarding-link {
      font-size: 12px;
      color: var(--text-tertiary);
      background: none;
      border: none;
      cursor: pointer;
      padding: 6px 12px;
      transition: color 0.15s;
      text-decoration: underline;
      text-underline-offset: 2px;
    }
    .onboarding-link:hover { color: var(--text-secondary); }

    /* ── Hero quick actions (projects exist, no tab selected) ── */
    .hero-quick-actions {
      display: flex;
      flex-direction: column;
      gap: 6px;
      margin-bottom: 8px;
      width: 100%;
      max-width: 280px;
    }
    .hero-quick-btn {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 8px 14px;
      font-size: 13px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      transition: all 0.15s ease;
      cursor: pointer;
      width: 100%;
      text-align: left;
    }
    .hero-quick-btn svg { color: var(--primary); flex-shrink: 0; }
    .hero-quick-btn:hover {
      background: rgba(107, 124, 255, 0.08);
      border-color: var(--primary);
      color: var(--text);
    }

    .chat-empty .new-convo {
      font-size: 18px;
      font-weight: 600;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .chat-empty .provider-hint {
      font-size: 12px;
      color: var(--text-tertiary);
      margin-bottom: 8px;
    }
    .suggestion-chips {
      display: flex;
      flex-wrap: wrap;
      gap: 8px;
      margin-top: 8px;
      justify-content: center;
      max-width: 440px;
    }
    .suggestion-chip {
      display: flex;
      align-items: center;
      gap: 7px;
      padding: 8px 14px;
      font-size: 12px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-pill);
      cursor: pointer;
      transition: all 0.15s ease;
    }
    .suggestion-chip svg { color: var(--text-tertiary); flex-shrink: 0; transition: color 0.15s; }
    .suggestion-chip:hover {
      background: var(--bg-accent);
      border-color: var(--border-strong);
      color: var(--text);
      transform: translateY(-1px);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.25);
    }
    .suggestion-chip:hover svg { color: var(--primary); }

    /* ── Messages ── */
    .messages-container {
      max-width: 768px;
      width: 100%;
      margin: 0 auto;
      padding: 16px 16px 24px;
      display: flex;
      flex-direction: column;
      gap: 20px;
      content-visibility: auto;
      contain-intrinsic-size: auto 500px;
      animation: fade-in 0.15s ease both;
    }

    /* ── Loading skeleton ── */
    .messages-loading {
      max-width: 768px;
      width: 100%;
      margin: 0 auto;
      padding: 24px 16px;
      display: flex;
      flex-direction: column;
      gap: 20px;
    }
    .msg-skeleton {
      display: flex;
      flex-direction: column;
      gap: 8px;
      max-width: 65%;
      margin-left: auto;
    }
    .msg-skeleton.assistant {
      margin-left: 0;
      margin-right: auto;
      max-width: 80%;
    }
    .skeleton-line {
      height: 12px;
      background: var(--bg-accent);
      border-radius: 6px;
      width: 40%;
      animation: skeleton-pulse 1.2s ease-in-out infinite;
    }
    .skeleton-line.wide { width: 90%; }
    .skeleton-line.medium { width: 65%; }
    @keyframes skeleton-pulse {
      0%, 100% { opacity: 0.4; }
      50% { opacity: 0.8; }
    }

    .msg {
      width: 100%;
      animation: fade-slide-up 0.2s ease both;
    }

    /* ── User message ── */
    .msg-user-bubble {
      max-width: 85%;
      margin-left: auto;
      padding: 10px 14px;
      border-radius: var(--radius-lg);
      font-size: 14px;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-word;
      background: rgba(107, 124, 255, 0.08);
      border: 1px solid rgba(107, 124, 255, 0.12);
      color: var(--text);
    }

    /* ── System message ── */
    .msg-system-pill {
      text-align: center;
      background: var(--bg-muted);
      border: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-secondary);
      border-radius: var(--radius-pill);
      padding: 4px 12px;
      display: inline-block;
      margin: 0 auto;
    }
    .msg-system { text-align: center; }

    /* ── Assistant message ── */
    .msg-assistant {
      width: 100%;
      display: flex;
      flex-direction: column;
      gap: 0;
    }

    /* Details toggle */
    .msg-details-toggle {
      display: inline-flex;
      align-items: center;
      gap: 5px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-tertiary);
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      margin-bottom: 6px;
      transition: color 0.12s, background 0.12s;
      align-self: flex-start;
    }
    .msg-details-toggle:hover {
      color: var(--text-secondary);
      background: var(--bg-hover);
    }
    .msg-details-toggle svg { flex-shrink: 0; }

    /* Message body — full width, clean typography */
    .msg-body {
      font-size: 14px;
      line-height: 1.6;
      color: var(--text);
    }
    .msg-no-text {
      font-size: 12px;
      color: var(--text-tertiary);
      font-style: italic;
    }

    /* Footer — right-aligned meta */
    .msg-footer {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 10px;
      margin-top: 6px;
      min-height: 20px;
    }
    .msg-meta {
      display: flex;
      align-items: center;
      gap: 0;
    }
    .msg-meta-tag {
      font-size: 10px;
      font-weight: 500;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      white-space: nowrap;
      opacity: 0.7;
    }
    .msg-meta-tag + .msg-meta-tag::before {
      content: "·";
      margin: 0 6px;
      opacity: 0.4;
    }
    .msg-copy {
      color: var(--text-tertiary);
      opacity: 0.5;
      padding: 3px;
      border-radius: var(--radius-sm);
      transition: all 0.12s;
      display: flex;
      align-items: center;
    }
    .msg-copy:hover { opacity: 1; background: var(--bg-accent); color: var(--text-secondary); }

    /* ── Message action buttons (edit, retry, regenerate) ── */
    .msg-user-actions {
      display: flex;
      justify-content: flex-end;
      gap: 4px;
      margin-top: 4px;
      opacity: 0;
      transition: opacity 0.15s ease;
    }
    .msg-user-bubble:hover + .msg-user-actions,
    .msg-user-actions:hover {
      opacity: 1;
    }
    .msg:hover .msg-user-actions {
      opacity: 1;
    }
    .msg:hover .msg-action-btn.footer-action {
      opacity: 1;
    }
    .msg-action-btn {
      display: flex;
      align-items: center;
      padding: 3px;
      border-radius: var(--radius-sm);
      color: var(--text-tertiary);
      opacity: 0;
      transition: opacity 0.12s ease, background 0.12s ease, color 0.12s ease;
      cursor: pointer;
    }
    .msg:hover .msg-action-btn {
      opacity: 0.5;
    }
    .msg:hover .msg-action-btn:hover {
      opacity: 1;
      background: var(--bg-accent);
      color: var(--text-secondary);
    }

    .typing-indicator {
      display: flex;
      gap: 5px;
      padding: 12px 0;
    }
    .typing-indicator .dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--text-tertiary);
      animation: bounce 1.4s infinite both;
    }
    .typing-indicator .dot:nth-child(2) { animation-delay: 0.16s; }
    .typing-indicator .dot:nth-child(3) { animation-delay: 0.32s; }

    @keyframes bounce {
      0%, 80%, 100% { transform: translateY(0); opacity: 0.4; }
      40% { transform: translateY(-5px); opacity: 1; }
    }
    /* ── Approvals ── */
    @keyframes approval-pulse {
      0%, 100% { border-color: rgba(240, 184, 64, 0.4); box-shadow: 0 0 0 0 rgba(240, 184, 64, 0); }
      50% { border-color: rgba(240, 184, 64, 0.7); box-shadow: 0 0 12px -2px rgba(240, 184, 64, 0.15); }
    }
    .approval-card {
      background: rgba(240, 184, 64, 0.04);
      border: 1px solid rgba(240, 184, 64, 0.4);
      border-radius: var(--radius-md);
      padding: 14px 16px;
      animation: approval-pulse 2s ease-in-out infinite;
    }
    .approval-header {
      display: flex;
      align-items: center;
      gap: 8px;
      margin-bottom: 8px;
    }
    .approval-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--amber);
    }
    .approval-desc {
      font-size: 12px;
      font-family: var(--font-mono);
      color: var(--text-secondary);
      margin-bottom: 12px;
      white-space: pre-wrap;
      overflow-wrap: break-word;
      background: var(--bg-base);
      padding: 8px 10px;
      border-radius: var(--radius-sm);
      max-height: 200px;
      overflow-y: auto;
      margin: 0 0 12px;
    }
    .approval-tool {
      font-size: 11px;
      font-family: var(--font-mono);
      color: var(--text-tertiary);
      margin-left: auto;
      padding: 1px 6px;
      background: var(--bg-accent);
      border-radius: var(--radius-pill);
    }
    .approval-actions { display: flex; gap: 6px; justify-content: flex-end; flex-wrap: wrap; }
    .approve-btn, .deny-btn, .bypass-btn {
      padding: 6px 12px;
      border-radius: var(--radius-sm);
      font-size: 11px;
      font-weight: 600;
      transition: all 0.12s;
    }
    .approve-btn {
      background: var(--green);
      color: #fff;
    }
    .approve-btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
    .bypass-btn {
      background: rgba(240, 184, 64, 0.1);
      border: 1px solid rgba(240, 184, 64, 0.2);
      color: var(--amber);
    }
    .bypass-btn:hover {
      background: rgba(240, 184, 64, 0.18);
      border-color: rgba(240, 184, 64, 0.35);
    }
    .deny-btn {
      background: var(--bg-muted);
      border: 1px solid var(--border);
      color: var(--text-secondary);
    }
    .deny-btn:hover { border-color: var(--border-strong); color: var(--text); }

    /* ── Worktree banner ── */
    .worktree-banner {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 8px 16px;
      margin: 12px auto 0;
      max-width: 768px;
      width: calc(100% - 32px);
      background: rgba(107, 124, 255, 0.06);
      border: 1px solid rgba(107, 124, 255, 0.15);
      border-radius: var(--radius-md);
      font-size: 12px;
    }
    .worktree-banner.subtle {
      background: var(--bg-muted);
      border-color: var(--border);
    }
    .wt-info { display: flex; flex-direction: column; gap: 2px; }
    .wt-branch { color: var(--primary); font-family: var(--font-mono); font-size: 12px; }
    .wt-path { color: var(--text-tertiary); font-size: 11px; }
    .wt-hint { color: var(--text-tertiary); }
    .wt-merge-btn {
      padding: 5px 12px;
      background: var(--primary);
      color: white;
      border-radius: var(--radius-sm);
      font-size: 12px;
      font-weight: 500;
      transition: filter 0.12s;
    }
    .wt-merge-btn:hover { filter: brightness(1.15); }
    .wt-create-btn {
      padding: 5px 12px;
      background: var(--bg-accent);
      border: 1px solid var(--border);
      color: var(--text-secondary);
      border-radius: var(--radius-sm);
      font-size: 12px;
      transition: background 0.12s;
    }
    .wt-create-btn:hover { background: var(--bg-muted); }
  `;
  document.head.appendChild(style);
}
