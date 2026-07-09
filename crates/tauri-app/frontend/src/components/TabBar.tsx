/* Tab bar / view switcher — Feature · Timeline · Diff review, plus repo name. */

import { For, Show } from "solid-js";
import type { ActiveView } from "../types";
import { appStore } from "../stores/app-store";

const TABS: { view: ActiveView; label: string }[] = [
  { view: "feature", label: "Feature" },
  { view: "graph", label: "Graph" },
  { view: "timeline", label: "Timeline" },
  { view: "diff", label: "Diff review" },
];

export function TabBar() {
  const { store } = appStore;

  return (
    <div class="tab-bar">
      <For each={TABS}>
        {(tab) => (
          <button
            class="tab"
            classList={{ active: store.activeView === tab.view }}
            onClick={() => appStore.setActiveView(tab.view)}
          >
            {tab.label}
            <Show when={tab.view === "feature" && store.selectedFeature}>
              <span class="tab-context">{store.selectedFeature}</span>
            </Show>
          </button>
        )}
      </For>
      <div class="tab-bar-spacer" />
      <Show when={store.repo}>
        {(repo) => <span class="tab-repo">{repo().name}</span>}
      </Show>
      <button
        class="tb-action"
        title="Toggle session pane (⌘\)"
        classList={{ active: store.sessionPaneOpen }}
        onClick={() => appStore.toggleSessionPane()}
      >
        <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
          <rect x="3" y="4" width="18" height="16" rx="2" />
          <line x1="14" y1="4" x2="14" y2="20" />
        </svg>
      </button>

      <style>{`
        /* Zed tab strip: chrome sits on the panel surface; the active tab adopts
         * the content background and merges into it (its bottom border is hidden
         * by a -1px overlap over the strip's own border). Hover is instant. */
        .tab-bar {
          display: flex;
          align-items: stretch;
          height: var(--tabbar-height);
          padding: 0 var(--space-2) 0 0;
          background: var(--bg-surface);
          border-bottom: 1px solid var(--border);
          flex-shrink: 0;
        }
        .tab {
          position: relative;
          display: flex; align-items: center; gap: 6px;
          padding: 0 14px;
          font-size: 13px;
          color: var(--text-tertiary);
          border-right: 1px solid var(--border-variant);
          white-space: nowrap;
        }
        .tab:first-child { border-left: 1px solid var(--border-variant); }
        .tab:hover { color: var(--text-secondary); background: var(--bg-hover); }
        .tab.active {
          color: var(--text);
          background: var(--bg-tab-active);
          margin-bottom: -1px;
          border-bottom: 1px solid var(--bg-tab-active);
        }
        .tab-context {
          font-family: var(--font-mono);
          font-size: 10px;
          color: var(--text-accent);
          max-width: 160px; overflow: hidden; text-overflow: ellipsis;
        }
        .tab-bar-spacer { flex: 1; }
        .tab-repo {
          display: flex; align-items: center;
          font-size: 11px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          padding: 0 var(--space-3);
        }
        .tab-bar .tb-action {
          align-self: center;
          width: 24px; height: 24px;
          border-radius: var(--radius-sm);
          display: flex; align-items: center; justify-content: center;
          color: var(--text-tertiary);
        }
        .tab-bar .tb-action:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .tab-bar .tb-action.active { color: var(--primary); background: var(--primary-glow); }
      `}</style>
    </div>
  );
}
