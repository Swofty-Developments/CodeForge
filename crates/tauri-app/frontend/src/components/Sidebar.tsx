/* Sidebar — the feature tree (features, not files). Repo header with daemon
 * status + index age · activity-sorted feature rows with unseen badges, pins and
 * confidence bars · index-status footer with Reindex. Width is driven by the
 * shell's resize handle via store.sidebarWidth. */

import { For, Show, createEffect, createMemo } from "solid-js";
import { appStore } from "../stores/app-store";
import { FeatureRow } from "./sidebar/FeatureRow";
import { GroupNode, type TreeCtx } from "./sidebar/GroupNode";
import { buildFeatureTree, ungroupedNode } from "./sidebar/tree";
import { latestActivity, markSeen, seen, unseenCounts } from "./sidebar/activity";
import { injectTreeStyles } from "./sidebar/tree-styles";

export function Sidebar() {
  injectTreeStyles();
  const { store } = appStore;

  const maxEventId = () => store.timeline.reduce((m, e) => Math.max(m, e.id), 0);

  // Baseline each feature's watermark to the current head so badges count only
  // activity that lands after the repo is open, then clear on selection.
  createEffect(() => {
    const head = maxEventId();
    for (const f of store.features) {
      if (seen[f.slug] === undefined) markSeen(f.slug, head);
    }
  });

  const tree = createMemo(() => buildFeatureTree(store.features, latestActivity(store.timeline)));
  const activity = createMemo(() => unseenCounts(store.timeline, seen));
  const hasFeatures = () => tree().roots.length > 0 || tree().ungrouped.length > 0;

  function onSelect(slug: string): void {
    markSeen(slug, maxEventId());
    appStore.openFeatureDetail(slug);
  }

  const treeCtx: TreeCtx = {
    activity: (slug) => activity().get(slug) ?? 0,
    selected: () => store.selectedFeature,
    onSelect,
    onTogglePin: (slug, pinned) => void appStore.pinFeature(slug, pinned),
  };

  return (
    <div class="sidebar" style={{ width: `${store.sidebarWidth}px` }}>
      <div class="sidebar-header">
        <div class="sb-repo-line">
          <span class="sb-repo-name" title={store.repo?.path ?? undefined}>
            {store.repo ? store.repo.name : "CodeForge"}
          </span>
        </div>
      </div>

      <div class="section-label sb-section-label">Features</div>

      <div class="sidebar-content">
        <Show
          when={store.repo}
          fallback={
            <div class="sb-blank">
              <span class="sb-blank-title">No repository open</span>
              <span class="sb-blank-sub">Open a repo to see its feature tree.</span>
            </div>
          }
        >
          <Show
            when={hasFeatures()}
            fallback={<div class="sb-blank"><span class="sb-blank-sub">No features indexed yet.</span></div>}
          >
            {/* Grouped features nest under group headers; ungrouped features go
                under an "Ungrouped" header only when real groups exist, else they
                render flat (an all-ungrouped repo shouldn't grow a lone header). */}
            <Show
              when={tree().roots.length > 0}
              fallback={
                <For each={tree().ungrouped}>
                  {(feature, i) => (
                    <FeatureRow
                      feature={feature}
                      active={store.selectedFeature === feature.slug}
                      activity={activity().get(feature.slug) ?? 0}
                      index={i()}
                      onSelect={onSelect}
                      onTogglePin={(slug, pinned) => void appStore.pinFeature(slug, pinned)}
                    />
                  )}
                </For>
              }
            >
              <For each={tree().roots}>
                {(node) => <GroupNode node={node} ctx={treeCtx} />}
              </For>
              <Show when={tree().ungrouped.length > 0}>
                <GroupNode node={ungroupedNode(tree().ungrouped)} ctx={treeCtx} />
              </Show>
            </Show>
          </Show>
        </Show>
      </div>

      {/* Footer shows ONLY live indexing progress; all resting status (daemon,
          freshness, reindex) lives in the bottom status bar now. */}
      <Show when={store.indexProgress}>
        {(p) => (
          <div class="sidebar-footer">
            <div class="sb-progress">
              <div class="sb-progress-head">
                <span class="sb-mono sb-progress-stage">{p().stage}</span>
                <span class="sb-mono sb-progress-count">
                  {p().done}/{p().total}
                </span>
              </div>
              <div class="sb-shimmer" />
              <Show when={p().detail}>
                <span class="sb-progress-detail">{p().detail}</span>
              </Show>
            </div>
          </div>
        )}
      </Show>

      <style>{`
        .sidebar {
          display: flex;
          flex-direction: column;
          background: var(--bg-surface);
          border-right: 1px solid var(--border);
          flex-shrink: 0;
          /* Stretch to the workspace row (between title/worktree strips and the
             status bar) — an explicit 100vh overflowed it and ran the feature
             list's scrollbar past the bottom of the window. */
          min-height: 0;
          min-width: 0;
        }
        .sidebar-header {
          display: flex;
          flex-direction: column;
          gap: 3px;
          padding: var(--space-4) var(--space-4) var(--space-3);
        }
        .sb-repo-line { display: flex; align-items: center; gap: var(--space-2); }
        .sb-repo-name {
          font-size: 13px; font-weight: 600; letter-spacing: -0.2px; color: var(--text);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }

        .sb-section-label { padding: 0 var(--space-4) 6px; }

        .sidebar-content { flex: 1; overflow-y: auto; overflow-x: hidden; padding: 0 var(--space-1); }
        .sb-blank {
          display: flex; flex-direction: column; gap: 4px;
          align-items: center; text-align: center;
          margin: auto; padding: var(--space-8) var(--space-4);
          color: var(--text-tertiary);
        }
        .sb-blank-title { font-size: 12px; font-weight: 600; color: var(--text-secondary); }
        .sb-blank-sub { font-size: 11px; line-height: 1.5; }

        .sidebar-footer { padding: var(--space-3); border-top: 1px solid var(--border); }
        .sb-mono { font-family: var(--font-mono); }

        .sb-progress { display: flex; flex-direction: column; gap: 5px; }
        .sb-progress-head { display: flex; align-items: baseline; justify-content: space-between; }
        .sb-progress-stage { font-size: 10px; color: var(--sky); text-transform: capitalize; }
        .sb-progress-count { font-size: 10px; color: var(--text-tertiary); font-variant-numeric: tabular-nums; }
        .sb-shimmer {
          height: 3px; border-radius: 2px; overflow: hidden;
          background: linear-gradient(90deg,
            rgba(var(--sky-rgb), 0.08) 0%,
            rgba(var(--sky-rgb), 0.35) 45%,
            rgba(var(--primary-rgb), 0.35) 55%,
            rgba(var(--sky-rgb), 0.08) 100%);
          background-size: 200% 100%;
          animation: shimmer-flow 1.5s ease-in-out infinite;
        }
        .sb-progress-detail {
          font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary);
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }

        @media (prefers-reduced-motion: reduce) {
          .sb-shimmer {
            animation: none;
            background: rgba(var(--sky-rgb), 0.2);
          }
        }
      `}</style>
    </div>
  );
}
