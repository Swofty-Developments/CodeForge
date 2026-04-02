import { Show } from "solid-js";
import { appStore } from "../../stores/app-store";

export function Breadcrumb() {
  const { store, setStore } = appStore;

  const activeThread = () => {
    const tab = store.activeTab;
    if (!tab) return null;
    return store.projects.flatMap((p) => p.threads).find((t) => t.id === tab) || null;
  };

  const activeProject = () => {
    const tab = store.activeTab;
    if (!tab) return null;
    return store.projects.find((p) => p.threads.some((t) => t.id === tab)) || null;
  };

  const worktreeBranch = () => {
    const tab = store.activeTab;
    if (!tab) return null;
    const wt = store.worktrees[tab];
    return wt?.active ? wt.branch : null;
  };

  const prNumber = () => {
    const tab = store.activeTab;
    const project = activeProject();
    if (!tab || !project) return null;
    const prMap = store.projectPrMap[project.id];
    return prMap?.[tab] ?? null;
  };

  const isSpecialTab = () => {
    const tab = store.activeTab;
    return tab?.startsWith("__") ?? false;
  };

  function toggleProjectCollapse() {
    const project = activeProject();
    if (!project) return;
    setStore("projects", (projects) =>
      projects.map((p) => p.id === project.id ? { ...p, collapsed: !p.collapsed } : p)
    );
  }

  function openDiff() {
    const tab = store.activeTab;
    if (tab) setStore("threadDiffOpen", tab, true);
  }

  return (
    <Show when={store.activeTab && !isSpecialTab()}>
      <div class="breadcrumb-bar">
        <Show when={activeProject()}>
          {(proj) => (
            <span class="bc-segment bc-clickable" onClick={toggleProjectCollapse}>
              {proj().name}
            </span>
          )}
        </Show>
        <Show when={activeThread()}>
          {(thread) => (
            <>
              <span class="bc-sep">&rsaquo;</span>
              <span class="bc-segment">{thread().title}</span>
            </>
          )}
        </Show>
        <Show when={worktreeBranch()}>
          {(branch) => (
            <>
              <span class="bc-sep">&rsaquo;</span>
              <span class="bc-segment bc-badge bc-clickable" onClick={openDiff}>
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="18" cy="18" r="3" /><circle cx="6" cy="6" r="3" /><path d="M6 21V9a9 9 0 009 9" />
                </svg>
                {branch()}
              </span>
            </>
          )}
        </Show>
        <Show when={prNumber()}>
          {(pr) => (
            <>
              <span class="bc-sep">&rsaquo;</span>
              <span class="bc-segment bc-badge bc-pr">PR #{pr()}</span>
            </>
          )}
        </Show>
      </div>
    </Show>
  );
}

if (!document.getElementById("breadcrumb-styles")) {
  const s = document.createElement("style");
  s.id = "breadcrumb-styles";
  s.textContent = `
    .breadcrumb-bar {
      display: flex;
      align-items: center;
      gap: 4px;
      height: 24px;
      padding: 0 12px;
      background: var(--bg-muted);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
      user-select: none;
      overflow: hidden;
    }
    .bc-segment {
      font-size: 11px;
      color: var(--text-tertiary);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .bc-clickable {
      cursor: pointer;
      border-radius: 3px;
      padding: 1px 4px;
      transition: background 0.1s, color 0.1s;
    }
    .bc-clickable:hover {
      background: var(--bg-accent);
      color: var(--text-secondary);
    }
    .bc-sep {
      font-size: 11px;
      color: var(--text-tertiary);
      opacity: 0.5;
      flex-shrink: 0;
    }
    .bc-badge {
      display: inline-flex;
      align-items: center;
      gap: 3px;
      font-family: var(--font-mono);
      font-size: 10px;
      padding: 1px 6px;
      border-radius: var(--radius-pill, 99px);
      background: rgba(107, 124, 255, 0.08);
      color: var(--primary);
    }
    .bc-pr {
      background: rgba(107, 124, 255, 0.1);
      color: var(--primary);
      font-weight: 600;
    }
  `;
  document.head.appendChild(s);
}
