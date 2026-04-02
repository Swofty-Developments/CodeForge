import { For } from "solid-js";
import { appStore } from "../../stores/app-store";

interface Shortcut {
  keys: string;
  description: string;
}

interface ShortcutCategory {
  title: string;
  shortcuts: Shortcut[];
}

const categories: ShortcutCategory[] = [
  {
    title: "Navigation",
    shortcuts: [
      { keys: "\u2318K", description: "Command palette" },
      { keys: "\u23181\u20139", description: "Switch to tab by position" },
      { keys: "\u2318W", description: "Close current tab" },
      { keys: "\u2318T", description: "New thread" },
      { keys: "\u2318\u21e7T", description: "Reopen last closed tab" },
    ],
  },
  {
    title: "Chat",
    shortcuts: [
      { keys: "Enter", description: "Send message" },
      { keys: "\u21e7Enter", description: "New line" },
      { keys: "\u2318Enter", description: "Force send (even mid-response)" },
      { keys: "\u2318.", description: "Stop / interrupt generation" },
      { keys: "/", description: "Slash commands" },
    ],
  },
  {
    title: "Panels",
    shortcuts: [
      { keys: "\u2318\u21e7B", description: "Toggle browser inspector" },
      { keys: "\u2318\u21e7D", description: "Toggle diff viewer" },
      { keys: "\u2318\u21e7F", description: "Search" },
      { keys: "\u2318\u21e7U", description: "Usage dashboard" },
      { keys: "\u2318\\", description: "Toggle split view" },
    ],
  },
  {
    title: "General",
    shortcuts: [
      { keys: "\u2318,", description: "Settings" },
      { keys: "\u2318?", description: "This help" },
      { keys: "Esc", description: "Close overlay / dismiss" },
    ],
  },
];

export function KeyboardHelp() {
  const { setStore } = appStore;

  function close() {
    setStore("keyboardHelpOpen", false);
  }

  return (
    <div class="overlay" onClick={close}>
      <div class="kb-help-panel" onClick={(e) => e.stopPropagation()}>
        <div class="kb-help-header">
          <h3 class="kb-help-title">Keyboard Shortcuts</h3>
          <button class="close-btn" onClick={close}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
        </div>

        <div class="kb-help-grid">
          <For each={categories}>
            {(cat) => (
              <div class="kb-help-category">
                <div class="kb-help-category-title">{cat.title}</div>
                <For each={cat.shortcuts}>
                  {(sc) => (
                    <div class="kb-help-row">
                      <kbd class="kb-help-keys">{sc.keys}</kbd>
                      <span class="kb-help-desc">{sc.description}</span>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>
      </div>

      <style>{`
        .kb-help-panel {
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          padding: 24px;
          max-width: 640px;
          width: 90%;
          max-height: 80vh;
          overflow-y: auto;
          box-shadow: 0 24px 80px rgba(0, 0, 0, 0.5), 0 0 1px rgba(255, 255, 255, 0.05);
          animation: overlay-panel-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
        }
        .kb-help-header {
          display: flex;
          align-items: center;
          justify-content: space-between;
          margin-bottom: 20px;
        }
        .kb-help-title {
          font-size: 16px;
          font-weight: 600;
          color: var(--text);
          margin: 0;
        }
        .kb-help-grid {
          display: grid;
          grid-template-columns: 1fr 1fr;
          gap: 20px;
        }
        @media (max-width: 520px) {
          .kb-help-grid {
            grid-template-columns: 1fr;
          }
        }
        .kb-help-category {
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .kb-help-category-title {
          font-size: 10px;
          font-weight: 600;
          text-transform: uppercase;
          letter-spacing: 0.08em;
          color: var(--text-tertiary);
          margin-bottom: 4px;
        }
        .kb-help-row {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 4px 0;
        }
        .kb-help-keys {
          font-size: 11px;
          font-family: var(--font-body);
          color: var(--text-secondary);
          background: var(--bg-accent);
          border: 1px solid var(--border);
          border-radius: 4px;
          padding: 2px 8px;
          min-width: 48px;
          text-align: center;
          flex-shrink: 0;
          white-space: nowrap;
        }
        .kb-help-desc {
          font-size: 13px;
          color: var(--text-secondary);
        }
      `}</style>
    </div>
  );
}
