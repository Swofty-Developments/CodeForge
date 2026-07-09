/* Right session pane — embedded Claude Code sessions (collapsible, placeholder
 * until forge-session is wired). */

import { For, Show, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";

export function SessionPane() {
  const { store } = appStore;
  const [input, setInput] = createSignal("");
  const active = () => store.sessions.find((s) => s.info.id === store.activeSessionId) ?? null;

  async function submit(e: Event) {
    e.preventDefault();
    const text = input().trim();
    if (!text) return;
    setInput("");
    if (!active()) await appStore.startSession();
    await appStore.sendSessionInput(text);
  }

  return (
    <div class="session-pane" style={{ width: `${store.sessionPaneWidth}px` }}>
      <div class="sp-header">
        <span class="section-label">Claude Code</span>
        <span
          class="status-dot"
          classList={{
            "status-dot--ready": active()?.runState === "ready",
            "status-dot--busy": active()?.runState === "generating",
            "status-dot--error": active()?.runState === "error",
          }}
        />
      </div>

      <div class="sp-body">
        <Show
          when={active()}
          fallback={
            <div class="sp-empty">
              <div class="sp-empty-title">No session yet</div>
              <div class="sp-empty-sub">
                Ask for work below — a real <span class="sp-mono">claude</span> session
                starts in this repo, with hooks feeding the timeline.
              </div>
            </div>
          }
        >
          {(session) => (
            <For each={session().blocks}>
              {(block) => (
                <Show when={block.type === "text"}>
                  <div class="sp-text">{block.content}</div>
                </Show>
              )}
            </For>
          )}
        </Show>
        <Show when={store.lastError}>
          <div class="sp-error">{store.lastError}</div>
        </Show>
      </div>

      <form class="sp-composer" onSubmit={submit}>
        <textarea
          class="sp-input"
          placeholder={store.repo ? "Ask Claude about this repo…" : "Open a repository first"}
          rows={2}
          disabled={!store.repo}
          value={input()}
          onInput={(e) => setInput(e.currentTarget.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter" && !e.shiftKey) {
              e.preventDefault();
              void submit(e);
            }
          }}
        />
        <button class="sp-send" type="submit" disabled={!store.repo} title="Send">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2">
            <path d="M12 19V5M5 12l7-7 7 7" />
          </svg>
        </button>
      </form>

      <style>{`
        .session-pane {
          display: flex;
          flex-direction: column;
          flex-shrink: 0;
          min-height: 0;
          background: var(--bg-surface);
          border-left: 1px solid var(--border);
        }
        .sp-header {
          display: flex; align-items: center; justify-content: space-between;
          height: 32px;
          padding: 0 var(--space-3);
          border-bottom: 1px solid var(--border);
          flex-shrink: 0;
        }
        .sp-body { flex: 1; overflow-y: auto; padding: var(--space-3); display: flex; flex-direction: column; gap: var(--space-2); }
        .sp-empty { margin: auto; text-align: center; max-width: 260px; animation: fade-slide-up 0.22s var(--ease-out) both; }
        .sp-empty-title { font-size: 13px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
        .sp-empty-sub { font-size: 11.5px; line-height: 1.5; color: var(--text-tertiary); }
        .sp-mono { font-family: var(--font-mono); color: var(--primary); }
        .sp-text {
          font-size: 13px; line-height: 1.6; color: var(--text);
          white-space: pre-wrap; word-break: break-word;
          animation: msg-assistant-in 0.18s var(--ease-out) both;
          user-select: text; -webkit-user-select: text;
        }
        .sp-error {
          font-size: 11px; font-family: var(--font-mono);
          padding: 6px 10px; border-radius: var(--radius-sm);
          background: rgba(var(--red-rgb), 0.08);
          border: 1px solid rgba(var(--red-rgb), 0.2);
          color: var(--red);
          word-break: break-word;
        }
        .sp-composer {
          display: flex; align-items: flex-end; gap: var(--space-2);
          margin: var(--space-3);
          padding: 10px 12px;
          background: var(--bg-card);
          border: 1px solid var(--border);
          border-radius: var(--radius-lg);
          transition: border-color 0.3s, box-shadow 0.3s;
        }
        .sp-composer:focus-within {
          border-color: var(--border-glow);
          box-shadow: 0 0 0 2px var(--primary-glow);
        }
        .sp-input {
          flex: 1;
          background: none; border: none; outline: none; resize: none;
          color: var(--text);
          font-size: 13px; line-height: 1.4;
          font-family: var(--font-body);
        }
        .sp-input::placeholder { color: var(--text-tertiary); }
        .sp-send {
          width: 26px; height: 26px; border-radius: 50%;
          background: var(--primary); color: #fff;
          display: flex; align-items: center; justify-content: center;
          flex-shrink: 0;
          transition: filter 0.15s, transform 0.1s;
        }
        .sp-send:hover { filter: brightness(1.1); transform: scale(1.04); }
        .sp-send:active { transform: scale(0.96); }
        .sp-send:disabled { opacity: 0.4; cursor: default; transform: none; filter: none; }
      `}</style>
    </div>
  );
}
