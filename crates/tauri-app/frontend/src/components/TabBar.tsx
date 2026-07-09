/* Tab bar / view switcher — Feature · Timeline · Diff review, plus repo name. */

import { For, Show } from "solid-js";
import type { ActiveView } from "../types";
import { appStore } from "../stores/app-store";

const TABS: { view: ActiveView; label: string }[] = [
  { view: "feature", label: "Feature" },
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
        .tab-bar {
          display: flex;
          align-items: center;
          gap: 2px;
          padding: 6px 8px;
          background: var(--bg-muted);
          border-bottom: 1px solid var(--border);
          flex-shrink: 0;
        }
        .tab {
          display: flex; align-items: center; gap: 6px;
          padding: 5px 12px;
          font-size: 12px; font-weight: 500;
          color: var(--text-secondary);
          border-radius: var(--radius-sm);
          transition: background 0.15s ease, color 0.15s ease;
          white-space: nowrap;
        }
        .tab:hover { background: var(--bg-hover); }
        .tab.active {
          background: var(--bg-base);
          color: var(--text);
          border: 1px solid var(--border-strong);
        }
        .tab-context {
          font-family: var(--font-mono);
          font-size: 10px;
          color: var(--primary);
          background: rgba(var(--primary-rgb), 0.08);
          padding: 1px 6px;
          border-radius: var(--radius-pill);
        }
        .tab-bar-spacer { flex: 1; }
        .tab-repo {
          font-size: 11px;
          font-family: var(--font-mono);
          color: var(--text-tertiary);
          padding: 0 var(--space-2);
        }
        .tb-action {
          width: 24px; height: 24px;
          border-radius: var(--radius-sm);
          display: flex; align-items: center; justify-content: center;
          color: var(--text-tertiary);
          transition: background 0.1s, color 0.1s;
        }
        .tb-action:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .tb-action.active { color: var(--primary); background: var(--primary-glow); }
      `}</style>
    </div>
  );
}
