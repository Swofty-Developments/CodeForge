import { createSignal, Show } from "solid-js";
import { appStore } from "../../stores/app-store";

function injectStyles() {
  if (document.getElementById("thread-toolbar-styles")) return;
  const s = document.createElement("style");
  s.id = "thread-toolbar-styles";
  s.textContent = `
    .thread-toolbar {
      display: flex;
      align-items: center;
      justify-content: flex-end;
      gap: 2px;
      height: 32px;
      padding: 0 8px;
      background: var(--bg-surface);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
      user-select: none;
    }
    .tt-btn {
      width: 26px;
      height: 26px;
      border: none;
      background: transparent;
      color: var(--text-tertiary);
      cursor: pointer;
      border-radius: var(--radius-sm, 6px);
      display: flex;
      align-items: center;
      justify-content: center;
      transition: background 0.1s, color 0.1s;
      position: relative;
    }
    .tt-btn:hover {
      background: var(--bg-accent);
      color: var(--text-secondary);
    }
    .tt-btn.active {
      color: var(--primary);
      background: var(--primary-glow);
    }
    .tt-btn.active:hover {
      background: rgba(107, 124, 255, 0.25);
    }
    .tt-toast {
      position: absolute;
      top: -28px;
      left: 50%;
      transform: translateX(-50%);
      background: var(--bg-accent);
      color: var(--green);
      font-size: 10px;
      font-weight: 600;
      font-family: var(--font-body);
      padding: 3px 8px;
      border-radius: var(--radius-sm, 6px);
      border: 1px solid var(--border);
      white-space: nowrap;
      pointer-events: none;
      animation: ttToastIn 0.15s ease-out;
    }
    @keyframes ttToastIn {
      from { opacity: 0; transform: translateX(-50%) translateY(4px); }
      to { opacity: 1; transform: translateX(-50%) translateY(0); }
    }
  `;
  document.head.appendChild(s);
}

// Inject once on module load
injectStyles();

export function ThreadToolbar() {
  const { store, setStore } = appStore;
  const [showCopied, setShowCopied] = createSignal(false);

  const activeTab = () => store.activeTab;
  const browserOpen = () => {
    const tab = activeTab();
    return tab ? !!store.threadBrowserOpen[tab] : false;
  };
  const diffOpen = () => store.diffPanelOpen;

  // Only show diff for git-activated projects (path !== ".")
  const isGitProject = () => {
    const tab = activeTab();
    if (!tab) return false;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    return project ? project.path !== "." : false;
  };

  function toggleBrowser() {
    const tab = activeTab();
    if (tab) {
      setStore("threadBrowserOpen", tab, !store.threadBrowserOpen[tab]);
    }
  }

  function toggleDiff() {
    setStore("diffPanelOpen", !store.diffPanelOpen);
  }

  function exportChat() {
    const tab = activeTab();
    if (!tab) return;
    const msgs = store.threadMessages[tab];
    if (!msgs || msgs.length === 0) return;

    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === tab);
    const title = thread?.title || "Chat";

    const md = `# ${title}\n\n` + msgs.map((m) => {
      const role = m.role === "user" ? "You" : m.role === "assistant" ? "Assistant" : "System";
      return `**${role}:**\n${m.content}\n`;
    }).join("\n---\n\n");

    navigator.clipboard.writeText(md).then(() => {
      setShowCopied(true);
      setTimeout(() => setShowCopied(false), 1500);
    }).catch(() => {
      // Fallback: trigger download
      const blob = new Blob([md], { type: "text/markdown" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${title.replace(/[^a-zA-Z0-9]/g, "_")}.md`;
      a.click();
      URL.revokeObjectURL(url);
    });
  }

  return (
    <div class="thread-toolbar">
      {/* Browser toggle */}
      <button
        class={`tt-btn ${browserOpen() ? "active" : ""}`}
        onClick={toggleBrowser}
        title="Toggle browser pane (Cmd+Shift+B)"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <circle cx="12" cy="12" r="10"/>
          <line x1="2" y1="12" x2="22" y2="12"/>
          <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
        </svg>
      </button>

      {/* Export chat */}
      <button
        class="tt-btn"
        onClick={exportChat}
        title="Export chat as markdown"
        style="position: relative;"
      >
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
          <polyline points="7 10 12 15 17 10"/>
          <line x1="12" y1="15" x2="12" y2="3"/>
        </svg>
        {showCopied() && <span class="tt-toast">Copied!</span>}
      </button>

      {/* Diff view toggle — only for git-activated projects */}
      <Show when={isGitProject()}>
        <button
          class={`tt-btn ${diffOpen() ? "active" : ""}`}
          onClick={toggleDiff}
          title="Toggle diff view (Cmd+Shift+D)"
        >
          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <line x1="6" y1="3" x2="6" y2="15"/>
            <circle cx="18" cy="6" r="3"/>
            <circle cx="6" cy="18" r="3"/>
            <path d="M18 9a9 9 0 0 1-9 9"/>
          </svg>
        </button>
      </Show>
    </div>
  );
}
