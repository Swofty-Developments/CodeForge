/* Re-index prompt (FZ-2). Shows when a context's feature index went `stale`
 * (files changed outside CodeForge — the git-pull case) or `outdated` (indexed
 * by an older format version). Copy names the concrete state; the user chooses to
 * re-index, defer, or mute this repo. Built on the global .overlay system. */

import { For, Show } from "solid-js";
import { appStore } from "../stores/app-store";
import type { IndexStatus } from "../types";

/** Last path segment — a readable repo label without the full absolute path. */
function repoLabel(path: string): string {
  const parts = path.replace(/\/+$/, "").split("/");
  const last = parts[parts.length - 1];
  return last.length > 0 ? last : path;
}

const MAX_LISTED = 6;

export function StaleModal() {
  const { store } = appStore;

  return (
    <Show when={store.staleModal} keyed>
      {(modal) => {
        const status: IndexStatus = modal.status;
        const repo = repoLabel(modal.repoPath);
        const outdated = status.state === "outdated";
        const extra = Math.max(0, status.changedFiles.length - MAX_LISTED);

        return (
          <div class="overlay" onClick={() => appStore.dismissStaleModal()}>
            <div class="overlay-panel stale-panel" onClick={(e) => e.stopPropagation()}>
              <div class="stale-head">
                <span class="stale-icon" classList={{ "stale-icon--outdated": outdated }}>
                  <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round">
                    <path d="M21 12a9 9 0 1 1-3-6.7M21 4v5h-5" />
                  </svg>
                </span>
                <div class="stale-title">
                  {outdated ? "Feature index is outdated" : "Feature index may be out of date"}
                </div>
              </div>

              <p class="stale-body">
                <Show
                  when={outdated}
                  fallback={
                    <>
                      We noticed files in <span class="stale-repo">{repo}</span> changed outside
                      CodeForge — your feature index may be out of date.
                    </>
                  }
                >
                  <>
                    <span class="stale-repo">{repo}</span> was indexed with an older CodeForge
                    version (<span class="stale-ver">v{status.indexVersion}</span> →{" "}
                    <span class="stale-ver">v{status.currentVersion}</span>). Re-index to use the
                    latest analysis.
                  </>
                </Show>
              </p>

              <Show when={!outdated && status.changedFiles.length > 0}>
                <div class="stale-files">
                  <For each={status.changedFiles.slice(0, MAX_LISTED)}>
                    {(f) => <div class="stale-file">{f}</div>}
                  </For>
                  <Show when={extra > 0}>
                    <div class="stale-more">+{extra} more</div>
                  </Show>
                </div>
              </Show>

              <div class="stale-actions">
                <button class="stale-mute" onClick={() => appStore.suppressStale(modal.repoPath)}>
                  Don't ask again
                </button>
                <div class="stale-spacer" />
                <button class="stale-later" onClick={() => appStore.dismissStaleModal()}>
                  Not now
                </button>
                <button class="stale-reindex" onClick={() => void appStore.reindexStale(modal.repoPath)}>
                  Re-index
                </button>
              </div>
            </div>

            <style>{`
              .stale-panel { display: flex; flex-direction: column; gap: 14px; }
              .stale-head { display: flex; align-items: center; gap: 10px; }
              .stale-icon {
                display: flex; align-items: center; justify-content: center;
                width: 30px; height: 30px; border-radius: var(--radius-md);
                color: var(--amber);
                background: rgba(var(--amber-rgb), 0.1);
                border: 1px solid rgba(var(--amber-rgb), 0.25);
                flex-shrink: 0;
              }
              .stale-icon--outdated {
                color: var(--primary);
                background: rgba(var(--primary-rgb), 0.1);
                border-color: rgba(var(--primary-rgb), 0.25);
              }
              .stale-title { font-size: 14px; font-weight: 600; color: var(--text); }
              .stale-body { font-size: 12.5px; line-height: 1.55; color: var(--text-secondary); }
              .stale-repo { font-family: var(--font-mono); font-size: 11.5px; color: var(--text); }
              .stale-ver { font-family: var(--font-mono); font-size: 11.5px; color: var(--primary); }
              .stale-files {
                display: flex; flex-direction: column; gap: 1px;
                max-height: 140px; overflow-y: auto;
                background: var(--bg-muted);
                border: 1px solid var(--border-variant);
                border-radius: var(--radius-sm);
                padding: 6px 8px;
              }
              .stale-file {
                font-family: var(--font-mono); font-size: 11px; line-height: 1.6;
                color: var(--editor-fg);
                white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
                direction: rtl; text-align: left;
              }
              .stale-more { font-family: var(--font-mono); font-size: 10.5px; color: var(--text-tertiary); margin-top: 2px; }
              .stale-actions { display: flex; align-items: center; gap: 8px; }
              .stale-spacer { flex: 1; }
              .stale-mute, .stale-later, .stale-reindex {
                font-size: 12px; font-weight: 600; padding: 6px 14px; border-radius: var(--radius-sm);
              }
              .stale-mute { color: var(--text-tertiary); }
              .stale-mute:hover { color: var(--text-secondary); background: var(--bg-accent); }
              .stale-later { color: var(--text-secondary); border: 1px solid var(--border); }
              .stale-later:hover { background: var(--bg-accent); color: var(--text); }
              .stale-reindex { color: #16202e; background: var(--primary); }
              .stale-reindex:hover { filter: brightness(1.08); }
            `}</style>
          </div>
        );
      }}
    </Show>
  );
}
