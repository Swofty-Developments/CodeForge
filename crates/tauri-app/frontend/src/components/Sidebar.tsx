/* Sidebar — the feature tree (features, not files): pin icons, confidence-dim
 * rows, tag badges, index-status footer. */

import { For, Show } from "solid-js";
import { appStore } from "../stores/app-store";

export function Sidebar() {
  const { store } = appStore;

  return (
    <div class="sidebar" style={{ width: `${store.sidebarWidth}px` }}>
      <div class="sidebar-header">
        <span class="sidebar-title">FeatureForge</span>
        <Show when={store.featuresArePlaceholder}>
          <span class="sidebar-demo-badge">preview</span>
        </Show>
      </div>

      <div class="section-label" style={{ padding: "0 16px 6px" }}>
        Features
      </div>

      <div class="sidebar-content">
        <For each={store.features}>
          {(feature, i) => (
            <button
              class="ft-row"
              classList={{ "ft-row--active": store.selectedFeature === feature.slug }}
              style={{ "animation-delay": `${i() * 30}ms` }}
              onClick={() => appStore.selectFeature(feature.slug)}
            >
              <span
                class="ft-dot"
                style={{ opacity: String(0.35 + feature.confidence * 0.65) }}
              />
              <span class="ft-name">{feature.name}</span>
              <span class="ft-count">{feature.files.length}</span>
              <Show when={feature.pinned}>
                <svg class="ft-pin" width="10" height="10" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M16 3v2l-1 1v5l3 3v2h-5v6l-1 1-1-1v-6H6v-2l3-3V6L8 5V3h8z" />
                </svg>
              </Show>
            </button>
          )}
        </For>
        <Show when={store.features.length === 0}>
          <div class="ft-empty">No features yet — open a repo to index it.</div>
        </Show>
      </div>

      <div class="sidebar-footer">
        <Show
          when={store.indexProgress}
          fallback={
            <div class="ft-index-status">
              <span
                class="status-dot"
                classList={{ "status-dot--ready": !!store.repo && !store.featuresArePlaceholder }}
              />
              <span>
                {store.repo
                  ? `${store.features.length} features indexed`
                  : "No repository open"}
              </span>
            </div>
          }
        >
          {(p) => (
            <div class="ft-index-status">
              <span class="status-dot status-dot--busy" />
              <span>
                Indexing {p().stage} · {p().done}/{p().total}
              </span>
            </div>
          )}
        </Show>
      </div>

      <style>{`
        .sidebar {
          display: flex;
          flex-direction: column;
          background: var(--bg-surface);
          border-right: 1px solid var(--border);
          flex-shrink: 0;
          height: 100vh;
        }
        .sidebar-header {
          display: flex;
          align-items: center;
          gap: var(--space-2);
          padding: var(--space-4) var(--space-4) var(--space-3);
        }
        .sidebar-title { font-size: 14px; font-weight: 700; letter-spacing: -0.3px; color: var(--text); }
        .sidebar-demo-badge {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 0 var(--space-1); border-radius: 3px; line-height: 1.5;
          color: var(--amber); background: rgba(var(--amber-rgb), 0.1);
        }
        .sidebar-content { flex: 1; overflow-y: auto; overflow-x: hidden; padding: 0 var(--space-1); }
        .sidebar-footer { padding: var(--space-3); border-top: 1px solid var(--border); }

        .ft-row {
          display: flex; align-items: center; gap: var(--space-2);
          width: calc(100% - 8px);
          padding: 6px var(--space-3);
          margin: 1px var(--space-1);
          border-radius: var(--radius-sm);
          cursor: pointer;
          text-align: left;
          transition: background 0.15s, box-shadow 0.2s;
          position: relative;
          animation: row-stagger-in 0.2s ease-out both;
        }
        .ft-row:hover { background: var(--bg-hover); }
        .ft-row--active { background: var(--bg-accent); box-shadow: 0 1px 6px rgba(0, 0, 0, 0.15); }
        .ft-row--active::before {
          content: ""; position: absolute; left: 0; top: 6px; bottom: 6px;
          width: 2px; border-radius: 1px; background: var(--primary);
        }
        .ft-dot { width: 6px; height: 6px; border-radius: 50%; background: var(--primary); flex-shrink: 0; }
        .ft-name {
          flex: 1; min-width: 0;
          font-size: 12px; color: var(--text-secondary);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .ft-row--active .ft-name { color: var(--text); font-weight: 500; }
        .ft-count {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          color: var(--text-tertiary);
        }
        .ft-pin { color: var(--amber); flex-shrink: 0; }
        .ft-empty { padding: var(--space-4); font-size: 12px; color: var(--text-tertiary); }
        .ft-index-status {
          display: flex; align-items: center; gap: var(--space-2);
          font-size: 11px; color: var(--text-tertiary); font-family: var(--font-mono);
        }
      `}</style>
    </div>
  );
}
