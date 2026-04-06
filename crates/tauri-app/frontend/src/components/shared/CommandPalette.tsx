import { createSignal, createMemo, For, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

interface PaletteAction {
  id: string;
  label: string;
  category: "Thread" | "Action" | "Setting";
  shortcut?: string;
  onSelect: () => void;
}

export function CommandPalette() {
  const { store, setStore, newThread, addProject, selectThread } = appStore;
  const [query, setQuery] = createSignal("");
  const [selectedIndex, setSelectedIndex] = createSignal(0);
  const [openPrs, setOpenPrs] = createSignal<ipc.OpenPr[]>([]);
  let inputRef: HTMLInputElement | undefined;

  // Load open PRs for GitHub projects (best-effort, non-blocking)
  onMount(() => {
    (async () => {
      for (const project of store.projects) {
        if (project.path !== "." && store.projectGitStatus[project.id] === "github") {
          try {
            const prs = await ipc.listOpenPrs(project.path);
            setOpenPrs(prs);
          } catch { /* ignore — gh might not be installed */ }
          break;
        }
      }
    })();
  });

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
      { id: "add-project", label: "Add Project", category: "Action", onSelect: () => { close(); addProject(); } },
      { id: "new-thread", label: "New Thread", category: "Action", shortcut: "\u2318T", onSelect: () => { newThread(); close(); } },
      { id: "settings", label: "Settings", category: "Setting", shortcut: "\u2318,", onSelect: () => { setStore("settingsOpen", true); close(); } },
      { id: "search", label: "Search", category: "Action", shortcut: "\u2318\u21e7F", onSelect: () => { setStore("searchOpen", true); close(); } },
      { id: "usage", label: "Usage Dashboard", category: "Action", shortcut: "\u2318\u21e7U", onSelect: () => { setStore("usageDashboardOpen", true); close(); } },
      { id: "diff", label: "Toggle Diff Viewer", category: "Action", shortcut: "\u2318\u21e7D", onSelect: () => {
        const tab = store.activeTab;
        if (tab) setStore("threadDiffOpen", tab, !store.threadDiffOpen[tab]);
        close();
      } },
      { id: "keyboard-help", label: "Keyboard Shortcuts", category: "Action", shortcut: "\u2318?", onSelect: () => { setStore("keyboardHelpOpen", true); close(); } },
      { id: "provider-claude", label: "Switch to Claude Code", category: "Setting", onSelect: () => { setStore("selectedProvider", "claude_code"); appStore.persistState(); close(); } },
      { id: "provider-codex", label: "Switch to Codex", category: "Setting", onSelect: () => { setStore("selectedProvider", "codex"); appStore.persistState(); close(); } },
      { id: "browser", label: "Toggle Browser Inspector", category: "Action", shortcut: "\u2318\u21e7B", onSelect: () => {
        const tab = store.activeTab;
        if (tab) setStore("threadBrowserOpen", tab, !store.threadBrowserOpen[tab]);
        close();
      } },
    );

    // Add open PRs as "Work on PR #N" actions
    for (const pr of openPrs()) {
      items.push({
        id: `pr-${pr.number}`,
        label: `Work on PR #${pr.number}: ${pr.title}`,
        category: "Action" as const,
        onSelect: async () => {
          close();
          // Check if already linked to a thread
          try {
            const existingThread = await ipc.findThreadForPr(pr.number);
            if (existingThread) {
              selectThread(existingThread);
              return;
            }
          } catch { /* continue to create */ }

          // Find the GitHub project
          const project = store.projects.find(
            (p) => p.path !== "." && store.projectGitStatus[p.id] === "github"
          );
          if (!project) return;

          // Create a new thread titled after the PR
          const threadId = await newThread(project.id);
          if (!threadId) return;

          // Rename the thread to the PR title
          try {
            await ipc.renameThread(threadId, `PR #${pr.number}: ${pr.title}`);
            setStore("projects", (projects) =>
              projects.map((p) => ({
                ...p,
                threads: p.threads.map((t) =>
                  t.id === threadId ? { ...t, title: `PR #${pr.number}: ${pr.title}` } : t
                ),
              }))
            );

            // Checkout the PR branch into a worktree
            const wt = await ipc.checkoutPrIntoWorktree(threadId, pr.number, project.path);
            setStore("worktrees", threadId, wt);
            setStore("projectPrMap", project.id, threadId, pr.number);
          } catch (e) {
            console.error("Failed to setup PR thread:", e);
          }
        },
      });
    }

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
                <span class="cmd-palette-item-right">
                  {item.shortcut && <kbd class="cmd-palette-shortcut">{item.shortcut}</kbd>}
                  <span class={`cmd-palette-item-category cat-${item.category.toLowerCase()}`}>
                    {item.category}
                  </span>
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
          padding-top: 22vh;
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
          gap: var(--space-3);
          padding: var(--space-4);
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
          padding: var(--space-1);
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
          padding: var(--space-2) var(--space-3);
          border-radius: var(--radius-sm);
          cursor: pointer;
          transition: background 0.08s;
        }
        .cmd-palette-item.selected { background: var(--bg-accent); }
        .cmd-palette-item-label {
          color: var(--text-secondary);
          font-size: 13px;
        }
        .cmd-palette-item.selected .cmd-palette-item-label { color: var(--text); }
        .cmd-palette-item-right {
          display: flex;
          align-items: center;
          gap: var(--space-2);
          flex-shrink: 0;
        }
        .cmd-palette-shortcut {
          font-size: 11px;
          font-family: var(--font-body);
          color: var(--text-tertiary);
          background: var(--bg-base);
          border: 1px solid var(--border);
          border-radius: 4px;
          padding: 1px 6px;
          white-space: nowrap;
        }
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
          padding: var(--space-6);
          text-align: center;
          color: var(--text-tertiary);
          font-size: 13px;
        }
      `}</style>
    </div>
  );
}
