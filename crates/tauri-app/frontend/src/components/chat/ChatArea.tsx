import { For, Show, createEffect, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { Markdown } from "./Markdown";
import { ToolUseCard } from "./ToolUseCard";
import { ThinkingBlock } from "./ThinkingBlock";
import { PrDashboard } from "../github/PrDashboard";
import type { ContentBlock } from "../../types";

export function ChatArea() {
  const { store, approveRequest, denyRequest } = appStore;
  let scrollRef: HTMLDivElement | undefined;

  const messages = () => {
    if (!store.activeTab) return [];
    return store.threadMessages[store.activeTab] || [];
  };

  const isGenerating = () => {
    if (!store.activeTab) return false;
    return store.sessionStatuses[store.activeTab] === "generating";
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
      appStore.setStore("threadMessages", tab, (msgs) => [
        ...(msgs || []),
        { id: crypto.randomUUID(), thread_id: tab, role: "system" as const, content: `Merge failed: ${e}` },
      ]);
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

  return (
    <div class="chat-area" ref={scrollRef} onScroll={handleScroll}>
      <Show
        when={store.activeTab}
        fallback={
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
            <p class="hero-subtitle">Select or create a thread to start</p>
            <div class="shortcut-hints">
              <span class="kbd-hint"><kbd>&#8984;K</kbd> Command palette</span>
              <span class="kbd-hint"><kbd>&#8984;N</kbd> New thread</span>
            </div>
          </div>
        }
      >
        {/* Worktree banners — only for git-activated projects */}
        <Show when={isGitProject()}>
          <Show when={worktree()}>
            <div class="worktree-banner">
              <div class="wt-info">
                <span class="wt-branch">{worktree()!.branch}</span>
                <span class="wt-path">{worktree()!.path}</span>
              </div>
              <button class="wt-merge-btn" onClick={handleMergeWorktree}>Merge back to main</button>
            </div>
          </Show>

          <Show when={!worktree() && messages().length === 0 && !isGenerating()}>
            <div class="worktree-banner subtle">
              <span class="wt-hint">Working in main branch</span>
              <button class="wt-create-btn" onClick={handleCreateWorktree}>Create worktree</button>
            </div>
          </Show>
        </Show>

        <Show
          when={messages().length > 0 || isGenerating()}
          fallback={
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

              {/* PR Dashboard for git-activated projects */}
              <Show when={isGitProject() && activeProject()}>
                <PrDashboard projectId={activeProject()!.id} repoPath={activeProject()!.path} />
              </Show>
            </div>
          }
        >
          <div class="messages-container">
            <For each={messages()}>
              {(msg, idx) => (
                <MessageBubble
                  msg={msg}
                  isGenerating={isGenerating()}
                  isLast={idx() === messages().length - 1}
                />
              )}
            </For>

            <For each={store.pendingApprovals.filter((a) => a.threadId === store.activeTab)}>
              {(approval) => (
                <div class="approval-card">
                  <div class="approval-header">
                    <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="var(--amber)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z" />
                      <line x1="12" y1="9" x2="12" y2="13" /><line x1="12" y1="17" x2="12.01" y2="17" />
                    </svg>
                    <span class="approval-title">Permission Required</span>
                  </div>
                  <pre class="approval-desc">{approval.description}</pre>
                  <div class="approval-actions">
                    <button class="deny-btn" onClick={() => denyRequest(approval)}>Deny</button>
                    <button class="approve-btn" onClick={() => approveRequest(approval)}>Approve</button>
                  </div>
                </div>
              )}
            </For>

            <Show when={isGenerating() && messages().length === 0}>
              <div class="typing-indicator">
                <span class="dot" /><span class="dot" /><span class="dot" />
              </div>
            </Show>
          </div>
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

function MessageBubble(props: { msg: any; isGenerating: boolean; isLast: boolean }) {
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

    /* ── Empty states ── */
    .chat-empty {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: var(--text-secondary);
      gap: 8px;
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
    .new-convo {
      font-size: 18px !important;
      font-weight: 600;
      color: var(--text) !important;
      letter-spacing: -0.3px;
    }
    .provider-hint {
      font-size: 12px !important;
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
    }

    .msg { width: 100%; }

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
    .approval-actions { display: flex; gap: 8px; justify-content: flex-end; }
    .approve-btn, .deny-btn {
      padding: 7px 16px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      font-weight: 600;
      transition: all 0.12s;
    }
    .approve-btn {
      background: var(--green);
      color: #fff;
    }
    .approve-btn:hover { filter: brightness(1.1); transform: translateY(-1px); }
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
