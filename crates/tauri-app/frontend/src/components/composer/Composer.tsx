import { Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { open } from "@tauri-apps/plugin-dialog";
import * as ipc from "../../ipc";

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
      // Move thread to existing group
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
              onInput={(e) => setStore("composerText", e.currentTarget.value)}
              onKeyDown={handleKeyDown}
              rows={1}
            />
            <button
              class={`send-btn ${isGenerating() ? "stop" : ""}`}
              onClick={sendUserMessage}
              disabled={isGenerating()}
            >
              {isGenerating() ? "\u25A0" : "\u2191"}
            </button>
          </div>
          <div class="composer-meta">
            <button
              class="meta-pill"
              onClick={() => setStore("providerPickerOpen", true)}
            >
              <span class="provider-dot" style={{ background: "var(--green)" }} />
              {providerLabel()}
              <span class="chevron">&#x25BC;</span>
            </button>
            <button class="meta-pill subtle" onClick={pickFolder}>
              {folderLabel()}
            </button>
            <div class="spacer" />
            <Show when={statusLabel()}>
              <div class="status-pill">
                <span class="status-dot" style={{ color: statusColor()! }}>&#x25CF;</span>
                <span style={{ color: statusColor()!, "font-size": "10px" }}>
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
      font-family: inherit;
    }
    .send-btn {
      width: 32px;
      height: 32px;
      border-radius: 50%;
      background: var(--primary);
      color: white;
      font-size: 16px;
      display: flex;
      align-items: center;
      justify-content: center;
      flex-shrink: 0;
      transition: background 0.15s;
    }
    .send-btn:hover { filter: brightness(1.1); }
    .send-btn.stop { background: var(--red); font-size: 12px; }
    .send-btn:disabled { opacity: 0.5; cursor: not-allowed; }
    .composer-meta {
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .meta-pill {
      font-size: 11px;
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
      font-size: 8px;
      color: var(--text-tertiary);
      transition: transform 0.15s;
    }
    .meta-pill:hover .chevron { color: var(--text-secondary); }
    .provider-dot {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .meta-pill.subtle {
      background: none;
      color: var(--text-tertiary);
    }
    .spacer { flex: 1; }
    .status-pill {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      background: var(--bg-muted);
    }
    .status-dot { font-size: 6px; }
  `;
  document.head.appendChild(style);
}
