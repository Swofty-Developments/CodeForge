/* Init-repo prompt: opening a plain folder is NOT a dead end. Shown when
 * open_repo rejects with the machine-matchable "not_a_git_repo:" prefix; offers
 * `git init -b main`, then adopts the context exactly like an open success.
 * Built on the global .overlay system. */

import { Show, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";

/** Last path segment — a readable folder label without the full absolute path. */
function baseName(path: string): string {
  const parts = path.replace(/\/+$/, "").split("/");
  const last = parts[parts.length - 1];
  return last.length > 0 ? last : path;
}

export function InitRepoModal() {
  const [busy, setBusy] = createSignal(false);

  async function initialize(): Promise<void> {
    if (busy()) return;
    setBusy(true);
    try {
      await appStore.confirmInitRepo();
    } finally {
      setBusy(false);
    }
  }

  function cancel(): void {
    if (!busy()) appStore.dismissInitRepoPrompt();
  }

  return (
    <Show when={appStore.initRepoPrompt()} keyed>
      {(path) => (
        <div class="overlay" onClick={cancel}>
          <div class="overlay-panel initrepo-panel" onClick={(e) => e.stopPropagation()}>
            <div class="initrepo-head">
              <span class="initrepo-icon">
                <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="6" cy="6" r="2.4" />
                  <circle cx="6" cy="18" r="2.4" />
                  <circle cx="18" cy="9" r="2.4" />
                  <path d="M6 8.4v7.2M15.7 10 8 16" />
                </svg>
              </span>
              <div class="initrepo-title">Not a git repository</div>
            </div>

            <p class="initrepo-body">
              <span class="initrepo-name">{baseName(path)}</span> isn't a git repository. CodeForge
              uses git for worktrees, branches and the timeline. Initialize it?
            </p>

            <div class="initrepo-path">{path}</div>

            <div class="initrepo-actions">
              <div class="initrepo-spacer" />
              <button class="initrepo-cancel" disabled={busy()} onClick={cancel}>
                Cancel
              </button>
              <button class="initrepo-confirm" disabled={busy()} onClick={() => void initialize()}>
                {busy() ? "Initializing…" : "Initialize repository"}
              </button>
            </div>
          </div>

          <style>{`
            .initrepo-panel { display: flex; flex-direction: column; gap: 14px; }
            .initrepo-head { display: flex; align-items: center; gap: 10px; }
            .initrepo-icon {
              display: flex; align-items: center; justify-content: center;
              width: 30px; height: 30px; border-radius: var(--radius-md);
              color: var(--primary);
              background: rgba(var(--primary-rgb), 0.1);
              border: 1px solid rgba(var(--primary-rgb), 0.25);
              flex-shrink: 0;
            }
            .initrepo-title { font-size: 14px; font-weight: 600; color: var(--text); }
            .initrepo-body { font-size: 12.5px; line-height: 1.55; color: var(--text-secondary); }
            .initrepo-name { font-family: var(--font-mono); font-size: 11.5px; color: var(--text); }
            .initrepo-path {
              font-family: var(--font-mono); font-size: 11px; line-height: 1.6;
              color: var(--editor-fg);
              background: var(--bg-muted);
              border: 1px solid var(--border-variant);
              border-radius: var(--radius-sm);
              padding: 6px 8px;
              white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
              direction: rtl; text-align: left;
            }
            .initrepo-actions { display: flex; align-items: center; gap: 8px; }
            .initrepo-spacer { flex: 1; }
            .initrepo-cancel, .initrepo-confirm {
              font-size: 12px; font-weight: 600; padding: 6px 14px; border-radius: var(--radius-sm);
            }
            .initrepo-cancel { color: var(--text-secondary); border: 1px solid var(--border); }
            .initrepo-cancel:hover:not(:disabled) { background: var(--bg-accent); color: var(--text); }
            .initrepo-confirm { color: #16202e; background: var(--primary); }
            .initrepo-confirm:hover:not(:disabled) { filter: brightness(1.08); }
            .initrepo-cancel:disabled, .initrepo-confirm:disabled { opacity: 0.6; cursor: default; }
          `}</style>
        </div>
      )}
    </Show>
  );
}
