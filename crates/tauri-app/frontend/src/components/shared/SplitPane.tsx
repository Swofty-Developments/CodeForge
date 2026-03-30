import { For, Show, createEffect, createSignal, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import { Markdown } from "../chat/Markdown";
import * as ipc from "../../ipc";
import type { ChatMessage } from "../../types";

/**
 * Self-contained chat + composer for a single thread.
 * Used inside SplitView to render each pane independently.
 */
export function SplitPane(props: { threadId: string }) {
  const { store, setStore, approveRequest, denyRequest } = appStore;
  let scrollRef: HTMLDivElement | undefined;

  const [text, setText] = createSignal("");

  const messages = () => store.threadMessages[props.threadId] || [];

  const isGenerating = () =>
    store.sessionStatuses[props.threadId] === "generating";

  const threadTitle = () => {
    const thread = store.projects
      .flatMap((p) => p.threads)
      .find((t) => t.id === props.threadId);
    return thread?.title || "Thread";
  };

  // Auto-scroll on new messages
  createEffect(() => {
    const _ = messages().length;
    const _g = isGenerating();
    requestAnimationFrame(() => {
      if (scrollRef) scrollRef.scrollTop = scrollRef.scrollHeight;
    });
  });

  // Ensure messages are loaded
  onMount(() => {
    appStore.loadThreadMessages(props.threadId);
  });

  async function handleSend() {
    const content = text().trim();
    if (!content) return;
    setText("");

    const threadId = props.threadId;

    try {
      const msgId = await ipc.persistUserMessage(threadId, content);
      const userMsg: ChatMessage = {
        id: msgId,
        thread_id: threadId,
        role: "user",
        content,
      };
      setStore("threadMessages", threadId, (msgs) => [
        ...(msgs || []),
        userMsg,
      ]);

      const project = store.projects.find((p) =>
        p.threads.some((t) => t.id === threadId)
      );
      const wt = store.worktrees[threadId];
      const cwd =
        wt?.active
          ? wt.path
          : project && project.path !== "."
            ? project.path
            : ".";

      setStore("sessionStatuses", threadId, "generating");
      await ipc.sendMessage(
        threadId,
        content,
        store.selectedProvider,
        cwd
      );
    } catch (e) {
      const errMsg: ChatMessage = {
        id: crypto.randomUUID(),
        thread_id: threadId,
        role: "system",
        content: `Error: ${e}`,
      };
      setStore("threadMessages", threadId, (msgs) => [
        ...(msgs || []),
        errMsg,
      ]);
      setStore("sessionStatuses", threadId, "error");
    }
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      handleSend();
    }
  }

  const [copied, setCopied] = createSignal<string | null>(null);

  async function copyContent(msgId: string, content: string) {
    await navigator.clipboard.writeText(content);
    setCopied(msgId);
    setTimeout(() => setCopied(null), 1500);
  }

  return (
    <div class="split-pane">
      <div class="split-pane-header">
        <span class="split-pane-title">{threadTitle()}</span>
        <Show when={isGenerating()}>
          <span class="split-pane-status">Working...</span>
        </Show>
      </div>

      <div class="split-pane-messages" ref={scrollRef}>
        <Show
          when={messages().length > 0 || isGenerating()}
          fallback={
            <div class="split-pane-empty">
              <p>New conversation</p>
            </div>
          }
        >
          <div class="split-pane-msg-list">
            <For each={messages()}>
              {(msg) => {
                const isAssistant = () => msg.role === "assistant";
                const isStreaming = () =>
                  isAssistant() &&
                  !msg.id.startsWith("done-") &&
                  isGenerating();

                return (
                  <div class={`sp-message sp-message-${msg.role}`}>
                    <div class="sp-message-content">
                      <div
                        class="sp-message-bubble"
                        classList={{ streaming: isStreaming() }}
                      >
                        <Show when={isAssistant()} fallback={msg.content}>
                          <Markdown content={msg.content} />
                        </Show>
                        <Show when={isStreaming()}>
                          <span class="cursor" />
                        </Show>
                      </div>
                      <Show when={isAssistant() && msg.content}>
                        <button
                          class="sp-copy-btn"
                          onClick={() => copyContent(msg.id, msg.content)}
                        >
                          {copied() === msg.id ? "Copied!" : "Copy"}
                        </button>
                      </Show>
                    </div>
                  </div>
                );
              }}
            </For>

            <For
              each={store.pendingApprovals.filter(
                (a) => a.threadId === props.threadId
              )}
            >
              {(approval) => (
                <div class="approval-card">
                  <div class="approval-desc">{approval.description}</div>
                  <div class="approval-actions">
                    <button
                      class="approve-btn"
                      onClick={() => approveRequest(approval)}
                    >
                      Approve
                    </button>
                    <button
                      class="deny-btn"
                      onClick={() => denyRequest(approval)}
                    >
                      Deny
                    </button>
                  </div>
                </div>
              )}
            </For>

            <Show when={isGenerating() && messages().length === 0}>
              <div class="typing-indicator">
                <span class="dot" />
                <span class="dot" />
                <span class="dot" />
              </div>
            </Show>
          </div>
        </Show>
      </div>

      <div class="split-pane-composer">
        <div class="split-pane-input-row">
          <textarea
            class="split-pane-input"
            placeholder="Message..."
            value={text()}
            onInput={(e) => setText(e.currentTarget.value)}
            onKeyDown={handleKeyDown}
            rows={1}
          />
          <button
            class={`split-pane-send ${isGenerating() ? "stop" : ""}`}
            onClick={handleSend}
            disabled={isGenerating()}
          >
            {isGenerating() ? "\u25A0" : "\u2191"}
          </button>
        </div>
      </div>
    </div>
  );
}

if (!document.getElementById("split-pane-styles")) {
  const style = document.createElement("style");
  style.id = "split-pane-styles";
  style.textContent = `
    .split-pane {
      display: flex;
      flex-direction: column;
      height: 100%;
      min-width: 0;
      overflow: hidden;
    }

    .split-pane-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 8px 14px;
      border-bottom: 1px solid rgba(255,255,255,0.07);
      flex-shrink: 0;
      background: #171719;
    }
    .split-pane-title {
      font-size: 12px;
      font-weight: 500;
      color: rgba(255,255,255,0.7);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .split-pane-status {
      font-size: 11px;
      color: #6680f2;
      flex-shrink: 0;
    }

    .split-pane-messages {
      flex: 1;
      overflow-y: auto;
      display: flex;
      flex-direction: column;
    }
    .split-pane-empty {
      flex: 1;
      display: flex;
      align-items: center;
      justify-content: center;
      color: rgba(255,255,255,0.3);
      font-size: 14px;
    }
    .split-pane-msg-list {
      max-width: 100%;
      padding: 12px 14px 20px;
      display: flex;
      flex-direction: column;
      gap: 12px;
    }

    .sp-message { display: flex; }
    .sp-message-user { justify-content: flex-end; }
    .sp-message-assistant { justify-content: flex-start; }
    .sp-message-system { justify-content: center; }

    .sp-message-content {
      display: flex;
      flex-direction: column;
      gap: 3px;
      max-width: 90%;
    }
    .sp-message-user .sp-message-content { align-items: flex-end; }
    .sp-message-assistant .sp-message-content { align-items: flex-start; }
    .sp-message-system .sp-message-content { align-items: center; }

    .sp-message-bubble {
      padding: 8px 12px;
      border-radius: 10px;
      font-size: 13px;
      line-height: 1.55;
      white-space: pre-wrap;
      word-break: break-word;
    }
    .sp-message-user .sp-message-bubble {
      background: rgba(255,255,255,0.06);
      border: 1px solid rgba(255,255,255,0.07);
    }
    .sp-message-assistant .sp-message-bubble {
      color: rgba(255,255,255,0.88);
      white-space: normal;
    }
    .sp-message-assistant .sp-message-bubble.streaming {
      border-left: 2px solid #6680f2;
      padding-left: 10px;
    }
    .sp-message-system .sp-message-bubble {
      background: rgba(255,255,255,0.04);
      border: 1px solid rgba(255,255,255,0.07);
      font-size: 11px;
      color: rgba(255,255,255,0.5);
      border-radius: 100px;
      padding: 3px 10px;
    }

    .sp-copy-btn {
      font-size: 10px;
      color: rgba(255,255,255,0.3);
      padding: 2px 6px;
      border-radius: 4px;
      transition: all 0.12s;
      opacity: 0;
      background: none;
      border: none;
      cursor: pointer;
    }
    .sp-message:hover .sp-copy-btn { opacity: 1; }
    .sp-copy-btn:hover { background: rgba(255,255,255,0.06); color: rgba(255,255,255,0.6); }

    .split-pane-composer {
      padding: 8px 14px 12px;
      border-top: 1px solid rgba(255,255,255,0.07);
      flex-shrink: 0;
      background: #171719;
    }
    .split-pane-input-row {
      display: flex;
      align-items: flex-end;
      gap: 8px;
      background: #131316;
      border: 1px solid rgba(255,255,255,0.07);
      border-radius: 10px;
      padding: 8px 10px;
    }
    .split-pane-input {
      flex: 1;
      background: none;
      border: none;
      color: rgba(255,255,255,0.88);
      font-size: 13px;
      resize: none;
      outline: none;
      padding: 2px 0;
      line-height: 1.4;
      min-height: 20px;
      max-height: 80px;
      font-family: inherit;
    }
    .split-pane-send {
      width: 28px;
      height: 28px;
      border-radius: 50%;
      background: #6680f2;
      color: white;
      font-size: 14px;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      border: none;
      cursor: pointer;
      transition: background 0.15s;
    }
    .split-pane-send:hover { filter: brightness(1.1); }
    .split-pane-send.stop { background: #e65961; font-size: 10px; }
    .split-pane-send:disabled { opacity: 0.5; cursor: not-allowed; }
  `;
  document.head.appendChild(style);
}
