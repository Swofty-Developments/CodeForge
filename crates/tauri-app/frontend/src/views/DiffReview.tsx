/* Diff review view — pending changes grouped by feature (accordion per group).
 * Placeholder empty state until forge-git is wired. */

import { For, Show } from "solid-js";
import { appStore } from "../stores/app-store";

export function DiffReview() {
  const { store } = appStore;
  const groups = () => store.diff?.groups ?? [];

  return (
    <div class="dr">
      <Show
        when={groups().length > 0}
        fallback={
          <div class="dr-empty">
            <div class="dr-empty-title">No pending changes</div>
            <div class="dr-empty-sub">
              When the working tree changes, diffs show up here grouped by feature —
              not by file.
            </div>
          </div>
        }
      >
        <div class="dr-list">
          <For each={groups()}>
            {(group) => (
              <div class="dr-group">
                <div class="dr-group-header">
                  <span class="dr-group-name">{group.name}</span>
                  <Show when={group.shared}>
                    <span class="dr-shared tint-purple">shared</span>
                  </Show>
                  <span class="dr-group-count">{group.files.length} files</span>
                </div>
                <For each={group.files}>
                  {(file) => (
                    <div class="dr-file">
                      <span class={`dr-badge dr-badge--${file.status}`}>{file.status}</span>
                      <span class="dr-path">{file.path}</span>
                      <span class="dr-stats">
                        <span class="dr-add">+{file.additions}</span>
                        <span class="dr-del">−{file.deletions}</span>
                      </span>
                    </div>
                  )}
                </For>
              </div>
            )}
          </For>
        </div>
      </Show>

      <style>{`
        .dr { flex: 1; display: flex; flex-direction: column; min-height: 0; }
        .dr-empty { margin: auto; text-align: center; max-width: 320px; animation: fade-slide-up 0.22s var(--ease-out) both; }
        .dr-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
        .dr-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }
        .dr-list { max-width: 768px; width: 100%; margin: 0 auto; padding: var(--space-4); }
        .dr-group {
          margin-bottom: var(--space-3);
          border: 1px solid var(--border);
          border-radius: var(--radius-md);
          background: rgba(255, 255, 255, 0.02);
          overflow: hidden;
          animation: fade-slide-up 0.18s var(--ease-out) both;
        }
        .dr-group-header {
          display: flex; align-items: center; gap: var(--space-2);
          padding: 8px 12px;
          border-bottom: 1px solid var(--border);
        }
        .dr-group-name { font-size: 12px; font-weight: 600; color: var(--text); flex: 1; }
        .dr-shared { font-size: 9px; font-family: var(--font-mono); padding: 0 6px; border-radius: var(--radius-pill); }
        .dr-group-count { font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary); }
        .dr-file {
          display: flex; align-items: center; gap: var(--space-2);
          padding: 5px 12px;
          font-family: var(--font-mono); font-size: 11.5px;
        }
        .dr-file:hover { background: var(--bg-hover); }
        .dr-badge { font-size: 9px; padding: 0 6px; border-radius: 3px; text-transform: uppercase; }
        .dr-badge--modified { background: rgba(var(--primary-rgb), 0.15); color: var(--primary); }
        .dr-badge--added { background: rgba(var(--green-rgb), 0.15); color: var(--green); }
        .dr-badge--deleted { background: rgba(var(--red-rgb), 0.15); color: var(--red); }
        .dr-badge--renamed, .dr-badge--untracked { background: rgba(255, 180, 80, 0.15); color: var(--orange); }
        .dr-path { flex: 1; color: var(--text-secondary); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .dr-stats { display: flex; gap: 6px; font-size: 10px; font-variant-numeric: tabular-nums; }
        .dr-add { color: var(--green); }
        .dr-del { color: var(--red); }
      `}</style>
    </div>
  );
}
