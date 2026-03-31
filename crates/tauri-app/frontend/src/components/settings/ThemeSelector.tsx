import { For, onMount, onCleanup, createSignal } from "solid-js";
import { appStore } from "../../stores/app-store";
import { themes, applyTheme } from "../../themes";
import { getSetting } from "../../ipc";

export function ThemeSelector() {
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

  return (
    <div class="overlay" onClick={(e) => { if (e.target === e.currentTarget) close(); }}>
      <div class="overlay-panel theme-selector-panel">
        <div class="theme-selector-header">
          <h2 class="theme-selector-title">Themes</h2>
          <button class="theme-close-btn" onClick={close} title="Close">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>
        <div class="theme-grid">
          <For each={themes}>
            {(theme) => (
              <button
                class="theme-card"
                classList={{ active: activeId() === theme.id }}
                onClick={() => selectTheme(theme.id)}
              >
                <div class="theme-preview">
                  <For each={theme.preview}>
                    {(color) => <div class="theme-swatch" style={{ background: color }} />}
                  </For>
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
      </div>

      <style>{`
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
          gap: 4px;
          height: 28px;
          border-radius: 5px;
          overflow: hidden;
        }
        .theme-swatch {
          flex: 1;
          border-radius: 3px;
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
      `}</style>
    </div>
  );
}
