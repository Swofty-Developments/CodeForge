/* Merge outcome surface (capability B). A clean merge fires a success toast and
 * clears; a conflicted/aborted merge shows an honest overlay listing the
 * conflicted files with the "base left clean — resolve in a session" message and
 * a one-click hand-off that prefills a session to resolve them. No fabricated
 * success: the panel only ever reports what git actually did. */

import { For, Show, createEffect } from "solid-js";
import { appStore } from "../stores/app-store";

export function MergeResultPanel() {
  const { store } = appStore;

  // Clean merge → transient success toast, then clear the result.
  createEffect(() => {
    const r = store.mergeResult;
    if (r && r.merged) {
      const msg = `Merged ${r.sourceBranch} into ${r.targetBranch}`;
      queueMicrotask(() => {
        appStore.pushSuccess(msg);
        appStore.dismissMergeResult();
      });
    }
  });

  function resolveInSession(): void {
    const r = store.mergeResult;
    if (!r) return;
    const base = store.contexts.find((c) => c.isBase);
    if (base) void appStore.switchContext(base.state.path);
    const files = r.conflicts.length > 0 ? r.conflicts.join(", ") : "the reported files";
    appStore.prefillComposer(
      `Merging ${r.sourceBranch} into ${r.targetBranch} hit conflicts and git aborted the merge, ` +
        `so ${r.targetBranch} is currently clean. Re-run the merge and resolve the conflicts in: ${files}.`,
    );
    appStore.dismissMergeResult();
  }

  return (
    <Show when={store.mergeResult && !store.mergeResult.merged}>
      {(_) => {
        const r = () => store.mergeResult!;
        return (
          <div class="overlay" onClick={() => appStore.dismissMergeResult()}>
            <div class="overlay-panel mr-panel" onClick={(e) => e.stopPropagation()}>
              <div class="mr-head">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M12 9v4M12 17h.01M10.3 3.9 1.8 18a2 2 0 0 0 1.7 3h17a2 2 0 0 0 1.7-3L13.7 3.9a2 2 0 0 0-3.4 0z" />
                </svg>
                <span class="mr-title">Merge conflict</span>
              </div>

              <p class="mr-summary">
                Merging <code>{r().sourceBranch}</code> into <code>{r().targetBranch}</code> hit conflicts.
                The merge was <strong>aborted</strong> and <code>{r().targetBranch}</code> was left clean —
                nothing was committed.
              </p>

              <Show when={r().conflicts.length > 0}>
                <div class="mr-files-label">Conflicted files</div>
                <div class="mr-files">
                  <For each={r().conflicts}>{(f) => <div class="mr-file">{f}</div>}</For>
                </div>
              </Show>

              <Show when={r().message}>
                <pre class="mr-git">{r().message}</pre>
              </Show>

              <div class="mr-actions">
                <button class="mr-dismiss" onClick={() => appStore.dismissMergeResult()}>Dismiss</button>
                <button class="mr-resolve" onClick={resolveInSession}>
                  <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                    <path d="M21 15a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h13a2 2 0 0 1 2 2z" />
                  </svg>
                  Resolve in a session
                </button>
              </div>
            </div>

            <style>{`
              .mr-panel { max-width: 460px; }
              .mr-head { display: flex; align-items: center; gap: 8px; margin-bottom: 10px; }
              .mr-head svg { color: var(--red); flex-shrink: 0; }
              .mr-title { font-size: 14px; font-weight: 700; color: var(--text); }
              .mr-summary { font-size: 12.5px; line-height: 1.6; color: var(--text-secondary); }
              .mr-summary code {
                font-family: var(--font-mono); font-size: 11px;
                background: var(--bg-muted); border: 1px solid var(--border);
                padding: 0 5px; border-radius: var(--radius-sm); color: var(--text);
              }
              .mr-summary strong { color: var(--amber); font-weight: 600; }
              .mr-files-label {
                font-size: 9px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em;
                color: var(--text-tertiary); margin: 14px 0 6px;
              }
              .mr-files {
                display: flex; flex-direction: column; gap: 2px;
                max-height: 160px; overflow-y: auto;
              }
              .mr-file {
                font-family: var(--font-mono); font-size: 11.5px; color: var(--red);
                background: rgba(var(--red-rgb), 0.06); border: 1px solid rgba(var(--red-rgb), 0.18);
                padding: 4px 8px; border-radius: var(--radius-sm);
                user-select: text; -webkit-user-select: text;
              }
              .mr-git {
                margin-top: 12px;
                font-family: var(--font-mono); font-size: 10.5px; line-height: 1.5;
                color: var(--text-tertiary);
                background: var(--bg-base); border: 1px solid var(--border);
                border-radius: var(--radius-sm); padding: 8px 10px;
                max-height: 120px; overflow: auto; white-space: pre-wrap; word-break: break-word;
                user-select: text; -webkit-user-select: text;
              }
              .mr-actions { display: flex; justify-content: flex-end; gap: 8px; margin-top: 16px; }
              .mr-dismiss, .mr-resolve {
                display: inline-flex; align-items: center; gap: 7px;
                font-size: 12.5px; font-weight: 600; padding: 7px 14px; border-radius: var(--radius-md);
              }
              .mr-dismiss { color: var(--text-secondary); border: 1px solid var(--border); }
              .mr-dismiss:hover { background: var(--bg-accent); color: var(--text); }
              .mr-resolve { color: #16202e; background: var(--primary); }
              .mr-resolve:hover { filter: brightness(1.08); }
            `}</style>
          </div>
        );
      }}
    </Show>
  );
}
