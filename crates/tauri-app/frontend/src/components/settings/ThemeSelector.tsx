import { For, onMount, onCleanup, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import { themes, applyTheme } from "../../themes";
import { getSetting } from "../../ipc";

export function ThemeSelector(props?: { inline?: boolean }) {
  const { setStore } = appStore;
  const [activeId, setActiveId] = createSignal("obsidian-forge");

  onMount(async () => {
    try {
      const saved = await getSetting("theme");
      if (saved) setActiveId(saved);
    } catch {}
  });

  function close() {
    setStore("themeOpen", false);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  }

  onMount(() => window.addEventListener("keydown", handleKeyDown));
  onCleanup(() => window.removeEventListener("keydown", handleKeyDown));

  function selectTheme(id: string) {
    applyTheme(id);
    setActiveId(id);
  }

  const content = (
    <>
      <div class="theme-selector-header">
        <h2 class="theme-selector-title">Themes</h2>
        {!props?.inline && (
          <button class="theme-close-btn" onClick={close} title="Close">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        )}
      </div>
      <div class="theme-grid">
          <For each={themes}>
            {(theme) => (
              <button
                class="theme-card"
                classList={{ active: activeId() === theme.id }}
                onClick={() => selectTheme(theme.id)}
              >
                <div class="theme-preview" style={{ background: theme.vars["--bg-base"] || theme.preview[0] }}>
                  <div class="theme-preview-sidebar" style={{ background: theme.vars["--bg-surface"] || theme.preview[1] }}>
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--primary"] || theme.preview[2], opacity: "0.7" }} />
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                  </div>
                  <div class="theme-preview-main">
                    <div class="theme-preview-topbar" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                    <div class="theme-preview-msg-user" style={{ background: theme.vars["--primary"] || theme.preview[2] }} />
                    <div class="theme-preview-msg-bot" style={{ background: theme.vars["--text"] || "#fff" }} />
                    <div class="theme-preview-composer" style={{ background: theme.vars["--text"] || "#fff" }} />
                  </div>
                </div>
                <div class="theme-info">
                  <span class="theme-name">{theme.name}</span>
                  <span class="theme-desc">{theme.description}</span>
                </div>
                {activeId() === theme.id && <span class="theme-badge">Active</span>}
              </button>
            )}
          </For>
        </div>
    </>
  );

  if (props?.inline) {
    return <div class="theme-selector-inline">{content}</div>;
  }

  return (
    <div class="overlay" onClick={(e) => { if (e.target === e.currentTarget) close(); }}>
      <div class="overlay-panel theme-selector-panel">{content}</div>
    </div>
  );
}

if (!document.getElementById("theme-selector-styles")) {
  const s = document.createElement("style");
  s.id = "theme-selector-styles";
  s.textContent = `
    .theme-selector-inline {
      max-width: 680px;
      margin: 0 auto;
      padding: 24px;
      width: 100%;
    }
    .theme-selector-panel {
      max-width: 680px;
      width: 94%;
      max-height: 80vh;
      overflow-y: auto;
      padding: 20px 24px 24px;
    }
    .theme-selector-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 16px;
    }
    .theme-selector-title {
      font-size: 16px;
      font-weight: 700;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .theme-close-btn {
      color: var(--text-tertiary);
      padding: 5px;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      transition: background 0.15s, color 0.15s;
    }
    .theme-close-btn:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
    .theme-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 10px;
    }
    @media (min-width: 620px) {
      .theme-grid {
        grid-template-columns: repeat(3, 1fr);
      }
    }
    .theme-card {
      position: relative;
      display: flex;
      flex-direction: column;
      gap: 10px;
      padding: 12px;
      border-radius: var(--radius-md);
      border: 1px solid var(--border);
      background: var(--bg-muted);
      text-align: left;
      cursor: pointer;
      transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
    }
    .theme-card:hover {
      border-color: var(--border-strong);
      background: var(--bg-accent);
    }
    .theme-card.active {
      border-color: var(--primary);
      box-shadow: 0 0 0 1px var(--primary-glow);
    }
    .theme-preview {
      display: flex;
      height: 80px;
      border-radius: 6px;
      overflow: hidden;
      border: 1px solid rgba(128,128,128,0.15);
    }
    .theme-preview-sidebar {
      width: 22%;
      display: flex;
      flex-direction: column;
      gap: 3px;
      padding: 4px 3px;
    }
    .theme-preview-sidebar-line {
      height: 3px;
      border-radius: 1px;
      opacity: 0.5;
    }
    .theme-preview-main {
      flex: 1;
      display: flex;
      flex-direction: column;
      padding: 4px 5px;
      gap: 3px;
    }
    .theme-preview-topbar {
      height: 4px;
      border-radius: 1px;
      opacity: 0.3;
    }
    .theme-preview-msg-user {
      height: 6px;
      border-radius: 3px;
      align-self: flex-end;
      width: 50%;
      opacity: 0.15;
    }
    .theme-preview-msg-bot {
      height: 10px;
      border-radius: 3px;
      width: 70%;
      opacity: 0.08;
    }
    .theme-preview-composer {
      margin-top: auto;
      height: 8px;
      border-radius: 4px;
      opacity: 0.12;
    }
    .theme-info {
      display: flex;
      flex-direction: column;
      gap: 2px;
      min-width: 0;
    }
    .theme-name {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .theme-desc {
      font-size: 11px;
      color: var(--text-tertiary);
      line-height: 1.3;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }
    .theme-badge {
      position: absolute;
      top: 8px;
      right: 8px;
      font-size: 10px;
      font-weight: 600;
      padding: 2px 7px;
      border-radius: var(--radius-pill);
      background: var(--primary);
      color: #fff;
      letter-spacing: 0.02em;
    }
  `;
  document.head.appendChild(s);
}
