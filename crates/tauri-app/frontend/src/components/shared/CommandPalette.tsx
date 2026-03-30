import { createSignal, createMemo, For, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";

interface PaletteAction {
  id: string;
  label: string;
  category: "Thread" | "Action" | "Setting";
  onSelect: () => void;
}

export function CommandPalette() {
  const { store, setStore, newThread, selectThread } = appStore;
  const [query, setQuery] = createSignal("");
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  let inputRef: HTMLInputElement | undefined;

  function close() {
    setStore("commandPaletteOpen", false);
  }

  const actions = createMemo<PaletteAction[]>(() => {
    const items: PaletteAction[] = [];

    for (const project of store.projects) {
      for (const thread of project.threads) {
        items.push({
          id: `thread-${thread.id}`,
          label: thread.title,
          category: "Thread",
          onSelect: () => { selectThread(thread.id); close(); },
        });
      }
    }

    for (const project of store.projects) {
      items.push({
        id: `project-${project.id}`,
        label: project.name,
        category: "Action",
        onSelect: () => {
          setStore("projects", (p: any) => p.id === project.id, "collapsed", (c: boolean) => !c);
          close();
        },
      });
    }

    items.push(
      { id: "new-thread", label: "New Thread", category: "Action", onSelect: () => { newThread(); close(); } },
      { id: "settings", label: "Settings", category: "Setting", onSelect: () => { setStore("settingsOpen", true); close(); } },
      { id: "provider-claude", label: "Switch to Claude Code", category: "Setting", onSelect: () => { setStore("selectedProvider", "claude_code"); close(); } },
      { id: "provider-codex", label: "Switch to Codex", category: "Setting", onSelect: () => { setStore("selectedProvider", "codex"); close(); } },
      { id: "browser", label: "Toggle Browser Inspector", category: "Action", onSelect: () => {
        const tab = store.activeTab;
        if (tab) setStore("threadBrowserOpen", tab, !store.threadBrowserOpen[tab]);
        close();
      } },
    );

    return items;
  });

  const filtered = createMemo(() => {
    const q = query().toLowerCase();
    if (!q) return actions();
    return actions().filter((a) => a.label.toLowerCase().includes(q));
  });

  function handleKeyDown(e: KeyboardEvent) {
    const items = filtered();
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((i) => (i + 1) % Math.max(items.length, 1));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((i) => (i - 1 + Math.max(items.length, 1)) % Math.max(items.length, 1));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = items[selectedIndex()];
      if (item) item.onSelect();
    } else if (e.key === "Escape") {
      e.preventDefault();
      close();
    }
  }

  onMount(() => inputRef?.focus());

  createMemo(() => { filtered(); setSelectedIndex(0); });

  return (
    <div class="cmd-palette-overlay" onClick={close}>
      <div class="cmd-palette" onClick={(e) => e.stopPropagation()}>
        <div class="cmd-palette-input-wrap">
          <svg class="cmd-palette-search-icon" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
          </svg>
          <input
            ref={inputRef}
            class="cmd-palette-input"
            type="text"
            placeholder="Type a command..."
            value={query()}
            onInput={(e) => setQuery(e.currentTarget.value)}
            onKeyDown={handleKeyDown}
          />
          <kbd class="cmd-palette-kbd">ESC</kbd>
        </div>
        <div class="cmd-palette-list">
          <For each={filtered()}>
            {(item, i) => (
              <div
                class={`cmd-palette-item ${i() === selectedIndex() ? "selected" : ""}`}
                onMouseEnter={() => setSelectedIndex(i())}
                onClick={() => item.onSelect()}
              >
                <span class="cmd-palette-item-label">{item.label}</span>
                <span class={`cmd-palette-item-category cat-${item.category.toLowerCase()}`}>
                  {item.category}
                </span>
              </div>
            )}
          </For>
          {filtered().length === 0 && (
            <div class="cmd-palette-empty">No results found</div>
          )}
        </div>
      </div>

      <style>{`
        .cmd-palette-overlay {
          position: fixed;
          inset: 0;
          z-index: 9999;
          background: rgba(0, 0, 0, 0.6);
          display: flex;
          align-items: flex-start;
          justify-content: center;
          padding-top: 18vh;
          backdrop-filter: blur(8px);
          -webkit-backdrop-filter: blur(8px);
          animation: cmdFadeIn 0.1s ease-out;
        }
        @keyframes cmdFadeIn {
          from { opacity: 0; }
          to { opacity: 1; }
        }
        @keyframes cmdSlideIn {
          from { opacity: 0; transform: translateY(-6px) scale(0.98); }
          to { opacity: 1; transform: translateY(0) scale(1); }
        }
        .cmd-palette {
          width: 520px;
          max-height: 420px;
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: 12px;
          box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255,255,255,0.03);
          display: flex;
          flex-direction: column;
          overflow: hidden;
          animation: cmdSlideIn 0.15s cubic-bezier(0.16, 1, 0.3, 1);
        }
        .cmd-palette-input-wrap {
          display: flex;
          align-items: center;
          gap: 10px;
          padding: 14px 16px;
          border-bottom: 1px solid var(--border);
        }
        .cmd-palette-search-icon {
          color: var(--text-tertiary);
          flex-shrink: 0;
        }
        .cmd-palette-input {
          flex: 1;
          background: transparent;
          border: none;
          outline: none;
          color: var(--text);
          font-size: 15px;
          font-family: var(--font-body);
          caret-color: var(--primary);
        }
        .cmd-palette-input::placeholder { color: var(--text-tertiary); }
        .cmd-palette-kbd {
          font-size: 10px;
          color: var(--text-tertiary);
          background: var(--bg-accent);
          border: 1px solid var(--border);
          border-radius: 4px;
          padding: 2px 6px;
          font-family: var(--font-body);
          flex-shrink: 0;
        }
        .cmd-palette-list {
          overflow-y: auto;
          padding: 6px;
          max-height: 350px;
        }
        .cmd-palette-list::-webkit-scrollbar { width: 4px; }
        .cmd-palette-list::-webkit-scrollbar-track { background: transparent; }
        .cmd-palette-list::-webkit-scrollbar-thumb {
          background: rgba(255, 255, 255, 0.06);
          border-radius: 4px;
        }
        .cmd-palette-item {
          display: flex;
          align-items: center;
          justify-content: space-between;
          padding: 10px 12px;
          border-radius: 8px;
          cursor: pointer;
          transition: background 0.08s;
        }
        .cmd-palette-item.selected { background: var(--bg-accent); }
        .cmd-palette-item-label {
          color: var(--text-secondary);
          font-size: 13px;
        }
        .cmd-palette-item.selected .cmd-palette-item-label { color: var(--text); }
        .cmd-palette-item-category {
          font-size: 10px;
          padding: 2px 8px;
          border-radius: 4px;
          font-weight: 600;
          letter-spacing: 0.02em;
          flex-shrink: 0;
        }
        .cat-thread {
          color: var(--primary);
          background: rgba(107, 124, 255, 0.1);
        }
        .cat-action {
          color: var(--green);
          background: rgba(76, 214, 148, 0.1);
        }
        .cat-setting {
          color: var(--amber);
          background: rgba(240, 184, 64, 0.1);
        }
        .cmd-palette-empty {
          padding: 24px;
          text-align: center;
          color: var(--text-tertiary);
          font-size: 13px;
        }
      `}</style>
    </div>
  );
}
