import { For, Show, createEffect } from "solid-js";
import { appStore } from "../../stores/app-store";

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

  // Auto-scroll on new messages
  createEffect(() => {
    const _ = messages().length;
    setTimeout(() => {
      if (scrollRef) scrollRef.scrollTop = scrollRef.scrollHeight;
    }, 10);
  });

  return (
    <div class="chat-area" ref={scrollRef}>
      <Show
        when={store.activeTab}
        fallback={
          <div class="chat-empty">
            <h2>CodeForge</h2>
            <p>Select or create a thread to start</p>
          </div>
        }
      >
        <Show
          when={messages().length > 0 || isGenerating()}
          fallback={
            <div class="chat-empty">
              <p class="new-convo">New conversation</p>
              <p class="provider-hint">
                Using {store.selectedProvider === "claude_code" ? "Claude Code" : "Codex"}
              </p>
            </div>
          }
        >
          <div class="messages-container">
            <For each={messages()}>
              {(msg) => (
                <div class={`message message-${msg.role}`}>
                  <div class="message-bubble">
                    {msg.content}
                  </div>
                </div>
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

            <Show when={isGenerating() && (messages().length === 0 || messages()[messages().length - 1]?.role !== "assistant")}>
              <div class="message message-assistant">
                <div class="message-bubble typing">
                  <span class="dot" /><span class="dot" /><span class="dot" />
                </div>
              </div>
            </Show>
          </div>
        </Show>
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
      padding: 16px;
      display: flex;
      flex-direction: column;
    }
    .chat-empty {
      flex: 1;
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      color: var(--text-secondary);
      gap: 4px;
    }
    .chat-empty h2 { font-size: 24px; font-weight: 300; color: var(--text-tertiary); }
    .chat-empty p { font-size: 14px; }
    .new-convo { font-size: 18px !important; color: var(--text-secondary); }
    .provider-hint { font-size: 12px !important; color: var(--text-tertiary); }
    .messages-container {
      max-width: 768px;
      width: 100%;
      margin: 0 auto;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }
    .message { display: flex; }
    .message-user { justify-content: flex-end; }
    .message-assistant { justify-content: flex-start; }
    .message-system { justify-content: center; }
    .message-bubble {
      padding: 10px 14px;
      border-radius: var(--radius-lg);
      font-size: 14px;
      line-height: 1.5;
      max-width: 560px;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .message-user .message-bubble {
      background: var(--bg-user-bubble);
      border: 1px solid var(--border);
    }
    .message-assistant .message-bubble {
      color: var(--text);
    }
    .message-system .message-bubble {
      background: var(--bg-muted);
      border: 1px solid var(--border);
      font-size: 12px;
      color: var(--text-secondary);
      border-radius: var(--radius-pill);
      padding: 4px 12px;
    }
    .approval-card {
      background: var(--bg-card);
      border: 1px solid var(--amber);
      border-radius: var(--radius-md);
      padding: 12px 16px;
      max-width: 560px;
    }
    .approval-desc {
      font-size: 13px;
      color: var(--text-secondary);
      margin-bottom: 10px;
      white-space: pre-wrap;
    }
    .approval-actions {
      display: flex;
      gap: 8px;
    }
    .approve-btn, .deny-btn {
      padding: 6px 14px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      font-weight: 500;
      transition: filter 0.12s;
    }
    .approve-btn {
      background: var(--green);
      color: #fff;
    }
    .deny-btn {
      background: var(--bg-muted);
      border: 1px solid var(--border);
      color: var(--text-secondary);
    }
    .approve-btn:hover, .deny-btn:hover { filter: brightness(1.15); }
    .typing {
      display: flex;
      gap: 4px;
      padding: 12px 16px;
    }
    .typing .dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      background: var(--text-tertiary);
      animation: blink 1.4s infinite both;
    }
    .typing .dot:nth-child(2) { animation-delay: 0.2s; }
    .typing .dot:nth-child(3) { animation-delay: 0.4s; }
    @keyframes blink {
      0%, 80%, 100% { opacity: 0.3; }
      40% { opacity: 1; }
    }
  `;
  document.head.appendChild(style);
}
