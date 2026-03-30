import { Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { open } from "@tauri-apps/plugin-dialog";
import * as ipc from "../../ipc";
import { ModelSelector } from "./ModelSelector";

export function Composer() {
  const { store, setStore, sendUserMessage } = appStore;

  const isActive = () => store.activeTab !== null;
  const isGenerating = () => {
    if (!store.activeTab) return false;
    return store.sessionStatuses[store.activeTab] === "generating";
  };

  const sessionStatus = () => {
    if (!store.activeTab) return null;
    return store.sessionStatuses[store.activeTab] || null;
  };

  const folderLabel = () => {
    if (!store.activeTab) return "No folder";
    const project = store.projects.find((p) =>
      p.threads.some((t) => t.id === store.activeTab)
    );
    if (!project || project.path === ".") return "No folder";
    return project.path.split("/").pop() || project.path;
  };

  async function pickFolder() {
    const selected = await open({ directory: true, title: "Select project folder" });
    if (!selected || !store.activeTab) return;
    const path = selected as string;

    const existing = store.projects.find((p) => p.path === path);
    if (existing) {
      await ipc.moveThreadToProject(store.activeTab, existing.id);
      moveThreadLocally(store.activeTab, existing.id);
    } else {
      const dirName = path.split("/").pop() || path;
      const created = await ipc.createProject(dirName, path);
      setStore("projects", (prev) => [
        ...prev,
        { ...created, color: null, collapsed: false, threads: [] },
      ]);
      await ipc.moveThreadToProject(store.activeTab!, created.id);
      moveThreadLocally(store.activeTab!, created.id);
    }
  }

  function moveThreadLocally(threadId: string, targetProjectId: string) {
    setStore("projects", (projects) => {
      const thread = projects.flatMap((p) => p.threads).find((t) => t.id === threadId);
      if (!thread) return projects;
      return projects.map((p) => ({
        ...p,
        threads:
          p.id === targetProjectId
            ? [...p.threads.filter((t) => t.id !== threadId), thread]
            : p.threads.filter((t) => t.id !== threadId),
      }));
    });
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      sendUserMessage();
    }
  }

  const providerLabel = () =>
    store.selectedProvider === "claude_code" ? "Claude Code" : "Codex";

  const statusColor = () => {
    const s = sessionStatus();
    if (s === "ready") return "var(--green)";
    if (s === "generating") return "var(--sky)";
    if (s === "starting") return "var(--amber)";
    if (s === "error") return "var(--red)";
    return null;
  };

  const statusLabel = () => {
    const s = sessionStatus();
    if (s === "ready") return "Ready";
    if (s === "generating") return "Working";
    if (s === "starting") return "Connecting";
    if (s === "error") return "Error";
    return null;
  };

  return (
    <Show when={isActive()}>
      <div class="composer-wrapper">
        <div class="composer-card">
          <div class="composer-input-row">
            <textarea
              class="composer-input"
              placeholder="Message..."
              value={store.composerText}
              onInput={(e) => {
                setStore("composerText", e.currentTarget.value);
                const el = e.currentTarget;
                el.style.height = "auto";
                el.style.height = Math.min(el.scrollHeight, 120) + "px";
              }}
              onKeyDown={handleKeyDown}
              rows={1}
            />
            <button
              class="send-btn"
              classList={{ stop: isGenerating() }}
              onClick={sendUserMessage}
              disabled={isGenerating()}
            >
              <Show when={isGenerating()} fallback={
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="12" y1="19" x2="12" y2="5" /><polyline points="5 12 12 5 19 12" />
                </svg>
              }>
                <svg width="12" height="12" viewBox="0 0 24 24" fill="currentColor"><rect x="4" y="4" width="16" height="16" rx="2" /></svg>
              </Show>
            </button>
          </div>
          <div class="composer-meta">
            <button
              class="meta-pill"
              onClick={() => setStore("providerPickerOpen", true)}
            >
              {providerLabel()}
              <span class="provider-dot" style={{ background: "var(--green)" }} />
              <svg class="chevron" width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><polyline points="6 9 12 15 18 9" /></svg>
            </button>
            <ModelSelector />
            <button class="meta-pill subtle" onClick={pickFolder}>
              <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                <path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" />
              </svg>
              {folderLabel()}
            </button>
            <div class="spacer" />
            <Show when={statusLabel()}>
              <div class="status-pill">
                <span class="status-dot" style={{ background: statusColor()! }} />
                <span class="status-text" style={{ color: statusColor()! }}>
                  {statusLabel()}
                </span>
              </div>
            </Show>
          </div>
        </div>
      </div>
    </Show>
  );
}

if (!document.getElementById("composer-styles")) {
  const style = document.createElement("style");
  style.id = "composer-styles";
  style.textContent = `
    .composer-wrapper {
      padding: 8px 20px 16px;
      display: flex;
      justify-content: center;
      flex-shrink: 0;
    }
    .composer-card {
      width: 100%;
      max-width: 768px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-xl);
      padding: 12px 14px;
      display: flex;
      flex-direction: column;
      gap: 8px;
      transition: border-color 0.2s, box-shadow 0.2s;
    }
    .composer-card:focus-within {
      border-color: var(--border-glow);
      box-shadow: 0 0 0 2px var(--primary-glow), 0 4px 16px rgba(0, 0, 0, 0.15);
    }
    .composer-input-row {
      display: flex;
      align-items: flex-end;
      gap: 8px;
    }
    .composer-input {
      flex: 1;
      background: none;
      border: none;
      color: var(--text);
      font-size: 14px;
      resize: none;
      outline: none;
      padding: 4px 0;
      line-height: 1.4;
      min-height: 22px;
      max-height: 120px;
      font-family: var(--font-body);
    }
    .send-btn {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      background: var(--primary);
      color: white;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      transition: background 0.15s, transform 0.1s;
    }
    .send-btn:hover { filter: brightness(1.1); transform: scale(1.04); }
    .send-btn:active { transform: scale(0.96); }
    .send-btn.stop { background: var(--red); }
    .send-btn:disabled { opacity: 0.5; cursor: not-allowed; transform: none; }
    .composer-meta {
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .meta-pill {
      font-size: 11px;
      font-weight: 500;
      color: var(--text-secondary);
      padding: 4px 10px 4px 8px;
      border-radius: var(--radius-pill);
      background: var(--bg-muted);
      border: 1px solid var(--border);
      transition: background 0.15s, border-color 0.15s;
      display: flex;
      align-items: center;
      gap: 5px;
    }
    .meta-pill:hover { background: var(--bg-accent); border-color: var(--border-strong); }
    .meta-pill .chevron {
      color: var(--text-tertiary);
      transition: color 0.15s;
    }
    .meta-pill:hover .chevron { color: var(--text-secondary); }
    .provider-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .meta-pill.subtle {
      background: none;
      border-color: transparent;
      color: var(--text-tertiary);
    }
    .meta-pill.subtle:hover {
      background: var(--bg-hover);
      border-color: var(--border);
    }
    .spacer { flex: 1; }
    .status-pill {
      display: flex;
      align-items: center;
      gap: 5px;
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      background: var(--bg-muted);
    }
    .status-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
    }
    .status-text { font-size: 10px; font-weight: 500; }
  `;
  document.head.appendChild(style);
}
