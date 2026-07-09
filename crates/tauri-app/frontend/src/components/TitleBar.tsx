/* Title bar — Zed-style IDE chrome. Draggable, macOS traffic-light inset on the
 * left, then the project + branch + feature-count pills. Right side carries the
 * reindex control. Pills are the primary identity affordance top-left. */

import { Show, createMemo } from "solid-js";
import { appStore } from "../stores/app-store";

export function TitleBar() {
  const { store } = appStore;
  const repo = () => store.repo;
  const indexing = createMemo(
    () => store.indexProgress != null && store.indexProgress.stage !== "error",
  );

  return (
    <div class="titlebar" data-tauri-drag-region>
      <div class="tb-traffic" />
      <Show
        when={repo()}
        fallback={<span class="tb-appname">FeatureForge</span>}
      >
        {(r) => (
          <div class="tb-pills">
            <button
              class="tb-pill tb-pill--project"
              title={r().path}
              onClick={() => appStore.setPaletteOpen(true)}
            >
              <svg class="tb-ico" viewBox="0 0 16 16" aria-hidden="true">
                <path
                  d="M1.5 3.5a1 1 0 0 1 1-1h3l1.2 1.4h6.3a1 1 0 0 1 1 1v6.6a1 1 0 0 1-1 1h-11a1 1 0 0 1-1-1z"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="1.1"
                />
              </svg>
              <span class="tb-pill-label">{r().name}</span>
            </button>

            <Show when={r().branch}>
              {(branch) => (
                <div class="tb-pill tb-pill--branch" title={`on branch ${branch()}`}>
                  <svg class="tb-ico" viewBox="0 0 16 16" aria-hidden="true">
                    <path
                      d="M4.5 2.5v7m0 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0-7a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m7 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0 3v1.5a3 3 0 0 1-3 3H4.5"
                      fill="none"
                      stroke="currentColor"
                      stroke-width="1.1"
                      stroke-linecap="round"
                    />
                  </svg>
                  <span class="tb-pill-label tb-mono">{branch()}</span>
                </div>
              )}
            </Show>

            <div class="tb-pill tb-pill--count" title="features in this repo">
              <span class="tb-pill-label tb-mono">{r().featuresCount}</span>
              <span class="tb-pill-sub">
                {r().featuresCount === 1 ? "feature" : "features"}
              </span>
            </div>
          </div>
        )}
      </Show>

      <div class="tb-right">
        <Show when={repo()}>
          <button
            class="tb-reindex"
            classList={{ "tb-reindex--busy": indexing() }}
            disabled={indexing()}
            onClick={() => void appStore.reindex()}
            title="Re-index this repository"
          >
            <span class="tb-reindex-dot" />
            {indexing() ? "indexing…" : "reindex"}
          </button>
        </Show>
      </div>

      <style>{`
        .titlebar {
          height: var(--titlebar-height);
          flex-shrink: 0;
          display: flex;
          align-items: center;
          gap: var(--space-2);
          padding: 0 var(--space-3) 0 0;
          background: var(--bg-chrome);
          border-bottom: 1px solid var(--border);
          -webkit-app-region: drag;
        }
        /* macOS traffic-light inset (window controls overlay this space). */
        .tb-traffic { width: 72px; flex-shrink: 0; height: 100%; }
        .tb-appname {
          font-size: 12px; font-weight: 600; letter-spacing: -0.2px;
          color: var(--text-secondary);
        }
        .tb-pills { display: flex; align-items: center; gap: var(--space-2); min-width: 0; }
        .tb-pill {
          display: inline-flex; align-items: center; gap: 6px;
          height: 22px; padding: 0 9px;
          border-radius: var(--radius-sm);
          background: var(--element-bg, var(--bg-muted));
          border: 1px solid var(--border-variant);
          color: var(--text-secondary);
          font-size: 12px; line-height: 1;
          -webkit-app-region: no-drag;
          max-width: 260px;
        }
        button.tb-pill { cursor: default; }
        button.tb-pill:hover { background: var(--bg-hover); color: var(--text); }
        .tb-pill--project .tb-pill-label { color: var(--text); font-weight: 500; }
        .tb-pill-label {
          overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
        }
        .tb-mono { font-family: var(--font-mono); font-size: 11px; }
        .tb-pill--branch { color: var(--text-muted); }
        .tb-pill--branch .tb-ico { color: var(--text-tertiary); }
        .tb-pill--count { gap: 4px; }
        .tb-pill--count .tb-pill-label { color: var(--primary); font-weight: 600; }
        .tb-pill-sub { color: var(--text-tertiary); font-size: 11px; }
        .tb-ico { width: 13px; height: 13px; flex-shrink: 0; }

        .tb-right { margin-left: auto; display: flex; align-items: center; -webkit-app-region: no-drag; }
        .tb-reindex {
          display: inline-flex; align-items: center; gap: 6px;
          height: 22px; padding: 0 10px;
          border-radius: var(--radius-sm);
          background: transparent;
          border: 1px solid var(--border-variant);
          color: var(--text-muted);
          font-family: var(--font-mono); font-size: 11px;
        }
        .tb-reindex:hover:not(:disabled) { background: var(--bg-hover); color: var(--text); }
        .tb-reindex:disabled { opacity: 0.8; }
        .tb-reindex-dot {
          width: 6px; height: 6px; border-radius: 50%;
          background: var(--text-tertiary);
        }
        .tb-reindex--busy .tb-reindex-dot {
          background: var(--primary);
          animation: dot-pulse 1.4s ease-in-out infinite;
        }
        @media (prefers-reduced-motion: reduce) {
          .tb-reindex--busy .tb-reindex-dot { animation: none; }
        }
      `}</style>
    </div>
  );
}
