/* Feature detail view — description, entry points, file list with roles,
 * living doc + recent timeline (placeholders until wired). */

import { For, Show, createMemo } from "solid-js";
import type { FileRole } from "../types";
import { appStore } from "../stores/app-store";

const ROLE_TINT: Record<FileRole, string> = {
  core: "tint-primary",
  support: "tint-sky",
  test: "tint-green",
  config: "tint-amber",
};

export function FeatureDetail() {
  const { store } = appStore;
  const feature = createMemo(() =>
    store.features.find((f) => f.slug === store.selectedFeature) ?? null,
  );

  return (
    <div class="fd">
      <Show
        when={feature()}
        fallback={
          <div class="fd-empty">
            <div class="fd-empty-title">Select a feature</div>
            <div class="fd-empty-sub">Pick a feature from the sidebar to see its docs, key files and recent changes.</div>
          </div>
        }
      >
        {(f) => (
          <div class="fd-inner">
            <div class="fd-header">
              <h2 class="fd-title">{f().name}</h2>
              <Show when={f().pinned}>
                <span class="fd-pill tint-amber">pinned</span>
              </Show>
              <span class="fd-confidence">{Math.round(f().confidence * 100)}%</span>
            </div>
            <p class="fd-description">{f().description}</p>
            <div class="fd-tags">
              <For each={f().tags}>{(tag) => <span class="fd-pill tint-purple">{tag}</span>}</For>
            </div>

            <div class="section-label fd-section">Entry points</div>
            <For each={f().entryPoints}>
              {(path) => <div class="fd-file fd-file--entry">{path}</div>}
            </For>

            <div class="section-label fd-section">Files</div>
            <For each={f().files}>
              {(file) => (
                <div class="fd-file">
                  <span class="fd-file-path">{file.path}</span>
                  <span class={`fd-pill ${ROLE_TINT[file.role]}`}>{file.role}</span>
                </div>
              )}
            </For>

            <div class="section-label fd-section">Living doc</div>
            <div class="fd-doc-placeholder">
              The per-feature doc (.featureforge/docs/{f().slug}.md) renders here once indexing is wired.
            </div>
          </div>
        )}
      </Show>

      <style>{`
        .fd { flex: 1; display: flex; flex-direction: column; min-height: 0; }
        .fd-inner {
          max-width: 768px;
          width: 100%;
          margin: 0 auto;
          padding: var(--space-6) var(--space-4) var(--space-8);
          animation: fade-slide-up 0.22s var(--ease-out) both;
        }
        .fd-empty { margin: auto; text-align: center; max-width: 320px; }
        .fd-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
        .fd-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }
        .fd-header { display: flex; align-items: center; gap: var(--space-2); }
        .fd-title { font-size: 20px; font-weight: 700; letter-spacing: -0.5px; color: var(--text); flex: 1; }
        .fd-confidence { font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary); }
        .fd-description { margin-top: var(--space-2); font-size: 13px; line-height: 1.6; color: var(--text-secondary); }
        .fd-tags { display: flex; gap: var(--space-1); margin-top: var(--space-3); flex-wrap: wrap; }
        .fd-section { margin: var(--space-6) 0 var(--space-2); }
        .fd-pill {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 1px 6px; border-radius: var(--radius-pill);
          text-transform: uppercase; letter-spacing: 0.04em;
        }
        .fd-file {
          display: flex; align-items: center; justify-content: space-between; gap: var(--space-2);
          padding: 6px 10px;
          margin-bottom: 2px;
          border-radius: var(--radius-sm);
          background: rgba(255, 255, 255, 0.02);
          border: 1px solid var(--border);
          font-family: var(--font-mono); font-size: 11.5px;
          color: var(--text-secondary);
        }
        .fd-file--entry { color: var(--primary); }
        .fd-file-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .fd-doc-placeholder {
          padding: var(--space-4);
          border: 1px dashed var(--border-strong);
          border-radius: var(--radius-md);
          font-size: 12px; color: var(--text-tertiary);
        }
      `}</style>
    </div>
  );
}
