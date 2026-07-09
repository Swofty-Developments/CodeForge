/* Sidebar — the feature tree (features, not files). Repo header with daemon
 * status + index age · activity-sorted feature rows with unseen badges, pins and
 * confidence bars · index-status footer with Reindex. Width is driven by the
 * shell's resize handle via store.sidebarWidth. */

import { For, Show, createEffect, createMemo } from "solid-js";
import { appStore } from "../stores/app-store";
import { FeatureRow } from "./sidebar/FeatureRow";
import { latestActivity, markSeen, seen, sortFeatures, unseenCounts } from "./sidebar/activity";
import { createNow, formatAgo } from "./sidebar/time";
import type { DaemonState } from "../types";

/** Hover title per named daemon state; `unknown` surfaces the probe error. */
function daemonTitle(d: DaemonState | null): string {
  if (!d) return "daemon: no repo";
  switch (d.kind) {
    case "running":
      return d.port != null ? `daemon on :${d.port}` : "daemon on";
    case "offline":
      return "daemon offline";
    case "unknown":
      return `daemon status unknown: ${d.error}`;
  }
}

export function Sidebar() {
  const { store } = appStore;
  const now = createNow();

  const maxEventId = () => store.timeline.reduce((m, e) => Math.max(m, e.id), 0);

  // Baseline each feature's watermark to the current head so badges count only
  // activity that lands after the repo is open, then clear on selection.
  createEffect(() => {
    const head = maxEventId();
    for (const f of store.features) {
      if (seen[f.slug] === undefined) markSeen(f.slug, head);
    }
  });

  const ordered = createMemo(() => sortFeatures(store.features, latestActivity(store.timeline)));
  const activity = createMemo(() => unseenCounts(store.timeline, seen));

  function onSelect(slug: string): void {
    markSeen(slug, maxEventId());
    appStore.openFeatureDetail(slug);
  }

  const daemonKind = () => store.daemon?.kind;
  const indexing = () => !!store.indexProgress;

  return (
    <div class="sidebar" style={{ width: `${store.sidebarWidth}px` }}>
      <div class="sidebar-header">
        <div class="sb-repo-line">
          <span class="sb-repo-name" title={store.repo?.path ?? undefined}>
            {store.repo ? store.repo.name : "FeatureForge"}
          </span>
          <Show when={store.repo}>
            <span
              class="status-dot"
              classList={{
                "status-dot--busy": indexing(),
                "status-dot--ready": !indexing() && daemonKind() === "running",
                "status-dot--error": !indexing() && daemonKind() === "offline",
                "status-dot--waiting": !indexing() && daemonKind() === "unknown",
              }}
              title={daemonTitle(store.daemon)}
            />
          </Show>
        </div>
        <Show when={store.repo}>
          <div class="sb-meta">
            <svg width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4">
              <circle cx="12" cy="12" r="9" /><path d="M12 7v5l3 3" />
            </svg>
            <span>{formatAgo(store.repo!.indexedAt, now())}</span>
          </div>
        </Show>
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
            when={ordered().length > 0}
            fallback={<div class="sb-blank"><span class="sb-blank-sub">No features indexed yet.</span></div>}
          >
            <For each={ordered()}>
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
          </Show>
        </Show>
      </div>

      <div class="sidebar-footer">
        <Show
          when={store.indexProgress}
          fallback={
            <div class="sb-foot-row">
              <div class="sb-index-status">
                <span
                  class="status-dot"
                  classList={{ "status-dot--ready": !!store.repo }}
                />
                <span class="sb-mono">
                  {store.repo ? `indexed ${formatAgo(store.repo.indexedAt, now())}` : "no repo"}
                </span>
              </div>
              <button
                class="sb-reindex"
                disabled={!store.repo}
                title="Re-index this repository"
                onClick={() => void appStore.reindex(false)}
              >
                Reindex
              </button>
            </div>
          }
        >
          {(p) => (
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
        .sb-meta {
          display: flex; align-items: center; gap: 4px;
          font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary);
        }
        .sb-meta svg { opacity: 0.7; flex-shrink: 0; }

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
        .sb-foot-row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); }
        .sb-index-status {
          display: flex; align-items: center; gap: var(--space-2);
          min-width: 0; font-size: 11px; color: var(--text-tertiary);
        }
        .sb-mono { font-family: var(--font-mono); }
        .sb-index-status .sb-mono { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

        .sb-reindex {
          flex-shrink: 0;
          font-family: var(--font-mono); font-size: 10px; font-weight: 600;
          letter-spacing: 0.02em;
          color: var(--text-secondary);
          padding: 3px 10px;
          border-radius: var(--radius-sm);
        }
        .sb-reindex:hover { background: rgba(var(--primary-rgb), 0.1); color: var(--primary); }
        .sb-reindex:disabled { opacity: 0.4; cursor: default; background: none; color: var(--text-tertiary); }

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
