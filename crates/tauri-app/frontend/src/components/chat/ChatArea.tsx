import { For, Show, createEffect, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import { Markdown } from "./Markdown";

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

  createEffect(() => {
    const _ = messages().length;
    const _g = isGenerating();
    requestAnimationFrame(() => {
      if (scrollRef) {
        scrollRef.scrollTo({ top: scrollRef.scrollHeight, behavior: "smooth" });
      }
    });
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

  function setSuggestion(text: string) {
    appStore.setStore("composerText", text);
  }

  return (
    <div class="chat-area" ref={scrollRef}>
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
        <Show when={worktree()}>
          <div class="worktree-banner">
            <div class="wt-info">
              <span class="wt-branch">{worktree()!.branch}</span>
              <span class="wt-path">{worktree()!.path}</span>
            </div>
            <button class="wt-merge-btn" onClick={handleMergeWorktree}>Merge back to main</button>
          </div>
        </Show>

        <Show when={!worktree() && hasFolder()}>
          <div class="worktree-banner subtle">
            <span class="wt-hint">Working in main branch</span>
            <button class="wt-create-btn" onClick={handleCreateWorktree}>Create worktree</button>
          </div>
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
                  <div class="approval-desc">{approval.description}</div>
                  <div class="approval-actions">
                    <button class="approve-btn" onClick={() => approveRequest(approval)}>Approve</button>
                    <button class="deny-btn" onClick={() => denyRequest(approval)}>Deny</button>
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

function MessageBubble(props: { msg: any; isGenerating: boolean; isLast: boolean }) {
  const [copied, setCopied] = createSignal(false);
  const isAssistant = () => props.msg.role === "assistant";
  const meta = () => props.msg.meta;

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

  return (
    <div class={`message message-${props.msg.role}`}>
      <div class="message-content">
        <div class="message-bubble">
          <Show when={isAssistant()} fallback={props.msg.content}>
            <Markdown content={props.msg.content} />
          </Show>
        </div>
        <div class="message-footer">
          <Show when={isAssistant() && meta()}>
            <div class="message-meta">
              <Show when={meta()!.model}>
                <span class="meta-tag">{meta()!.model}</span>
              </Show>
              <Show when={meta()!.inputTokens != null || meta()!.outputTokens != null}>
                <span class="meta-tag">{formatTokens(meta()!.inputTokens || 0)} in / {formatTokens(meta()!.outputTokens || 0)} out</span>
              </Show>
              <Show when={meta()!.durationMs}>
                <span class="meta-tag">{formatDuration(meta()!.durationMs!)}</span>
              </Show>
            </div>
          </Show>
          <Show when={isAssistant() && props.msg.content}>
            <button class="copy-btn" onClick={copyContent} title={copied() ? "Copied!" : "Copy"}>
              <Show when={copied()} fallback={
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <rect x="9" y="9" width="13" height="13" rx="2"/><path d="M5 15H4a2 2 0 01-2-2V4a2 2 0 012-2h9a2 2 0 012 2v1"/>
                </svg>
              }>
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="20 6 9 17 4 12"/>
                </svg>
              </Show>
            </button>
          </Show>
        </div>
      </div>
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
      gap: 16px;
    }

    .message {
      display: flex;
    }
    .message-user { justify-content: flex-end; }
    .message-assistant { justify-content: flex-start; }
    .message-system { justify-content: center; }

    .message-content {
      display: flex;
      flex-direction: column;
      gap: 4px;
      max-width: 600px;
    }
    .message-user .message-content { align-items: flex-end; }
    .message-assistant .message-content { align-items: flex-start; }
    .message-system .message-content { align-items: center; }

    .message-bubble {
      padding: 10px 14px;
      border-radius: var(--radius-lg);
      font-size: 14px;
      line-height: 1.6;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .message-user .message-bubble {
      background: rgba(107, 124, 255, 0.08);
      border: 1px solid rgba(107, 124, 255, 0.12);
      color: var(--text);
    }
    .message-assistant .message-bubble {
      color: var(--text);
      white-space: normal;
    }
    .message-system .message-bubble {
      background: var(--bg-muted);
      border: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-secondary);
      border-radius: var(--radius-pill);
      padding: 4px 12px;
    }

    .message-footer {
      display: flex;
      align-items: center;
      gap: 8px;
      min-height: 18px;
    }
    .message-meta {
      display: flex;
      align-items: center;
      gap: 6px;
      flex-wrap: wrap;
    }
    .meta-tag {
      font-size: 10px;
      font-weight: 500;
      color: var(--text-tertiary);
      padding: 1px 0;
      font-family: var(--font-mono);
      white-space: nowrap;
    }
    .meta-tag + .meta-tag::before {
      content: "·";
      margin-right: 6px;
      color: var(--text-tertiary);
      opacity: 0.4;
    }
    .copy-btn {
      color: var(--text-tertiary);
      padding: 3px;
      border-radius: var(--radius-sm);
      transition: all 0.12s;
      margin-left: auto;
      display: flex;
      align-items: center;
    }
    .copy-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

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
    .approval-card {
      background: var(--bg-card);
      border: 1px solid var(--amber);
      border-radius: var(--radius-md);
      padding: 12px 16px;
    }
    .approval-desc {
      font-size: 13px;
      color: var(--text-secondary);
      margin-bottom: 10px;
      white-space: pre-wrap;
    }
    .approval-actions { display: flex; gap: 8px; }
    .approve-btn, .deny-btn {
      padding: 6px 14px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      font-weight: 500;
      transition: filter 0.12s;
    }
    .approve-btn { background: var(--green); color: #fff; }
    .deny-btn { background: var(--bg-muted); border: 1px solid var(--border); color: var(--text-secondary); }
    .approve-btn:hover, .deny-btn:hover { filter: brightness(1.15); }

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
