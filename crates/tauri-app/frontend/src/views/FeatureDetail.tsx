/* Feature detail view — inline-editable header (name/description), entry points
 * (open in editor), files grouped by role, living doc, recent activity, and the
 * "Ask Claude about this feature" hand-off. Sections use the shared collapse. */

import { For, Show, createMemo, createSignal } from "solid-js";
import { marked } from "marked";
import type { Feature } from "../types";
import { appStore } from "../stores/app-store";
import { CollapsibleSection } from "../components/feature/CollapsibleSection";
import { FilesSection } from "../components/feature/FilesSection";
import { RecentActivity } from "../components/feature/RecentActivity";
import { openInEditor } from "../components/feature/open-path";
import { createNow } from "../components/sidebar/time";

const RECENT_LIMIT = 15;

function confidenceTint(c: number): string {
  if (c >= 0.75) return "tint-green";
  if (c >= 0.5) return "tint-amber";
  return "tint-red";
}

export function FeatureDetail() {
  const { store } = appStore;
  const now = createNow();
  const feature = createMemo(
    () => store.features.find((f) => f.slug === store.selectedFeature) ?? null,
  );

  const [editingName, setEditingName] = createSignal(false);
  const [editingDesc, setEditingDesc] = createSignal(false);

  function commitName(f: Feature, value: string): void {
    setEditingName(false);
    const next = value.trim();
    if (next && next !== f.name) void appStore.updateFeature(f.slug, { name: next });
  }

  function commitDesc(f: Feature, value: string): void {
    setEditingDesc(false);
    const next = value.trim();
    if (next !== f.description) void appStore.updateFeature(f.slug, { description: next });
  }

  function open(path: string): void {
    void openInEditor(store.repo?.path, path).catch((e) => appStore.pushError(String(e)));
  }

  function askClaude(f: Feature): void {
    appStore.prefillComposer(`Explain the ${f.name} feature and its current state`);
  }

  const recent = createMemo(() => store.selectedFeatureTimeline.slice(0, RECENT_LIMIT));
  // The living doc (.featureforge/docs/<slug>.md) is the single source of truth,
  // written during indexing. Absent = "not indexed yet" (an honest empty state,
  // shown by the Show fallback below), never the description standing in for it.
  const livingDoc = createMemo(() => {
    const md = store.selectedFeatureDoc;
    return md ? (marked.parse(md) as string) : "";
  });

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
            {/* ── Header ── */}
            <div class="fd-header">
              <Show
                when={editingName()}
                fallback={
                  <h2 class="fd-title" title="Double-click to rename" onDblClick={() => setEditingName(true)}>
                    {f().name}
                  </h2>
                }
              >
                <input
                  class="fd-title-input"
                  value={f().name}
                  ref={(el) => setTimeout(() => { el.focus(); el.select(); }, 0)}
                  onBlur={(e) => commitName(f(), e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") { e.preventDefault(); commitName(f(), e.currentTarget.value); }
                    else if (e.key === "Escape") { e.preventDefault(); setEditingName(false); }
                  }}
                />
              </Show>

              <button
                class="fd-pin"
                classList={{ "fd-pin--on": f().pinned }}
                title={f().pinned ? "Unpin feature" : "Pin feature"}
                onClick={() => void appStore.pinFeature(f().slug, !f().pinned)}
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="currentColor">
                  <path d="M16 3v2l-1 1v5l3 3v2h-5v6l-1 1-1-1v-6H6v-2l3-3V6L8 5V3h8z" />
                </svg>
              </button>

              <span class={`fd-chip fd-confidence ${confidenceTint(f().confidence)}`}>
                {Math.round(f().confidence * 100)}% conf
              </span>
            </div>

            <Show when={f().tags.length > 0}>
              <div class="fd-tags">
                <For each={f().tags}>{(tag) => <span class="fd-chip tint-purple">{tag}</span>}</For>
              </div>
            </Show>

            {/* ── Description (click to edit) ── */}
            <Show
              when={editingDesc()}
              fallback={
                <p
                  class="fd-description"
                  classList={{ "fd-description--empty": !f().description }}
                  title="Click to edit"
                  onClick={() => setEditingDesc(true)}
                >
                  {f().description || "Add a description…"}
                </p>
              }
            >
              <textarea
                class="fd-desc-input"
                rows={3}
                value={f().description}
                ref={(el) => setTimeout(() => { el.focus(); el.select(); }, 0)}
                onBlur={(e) => commitDesc(f(), e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") { e.preventDefault(); setEditingDesc(false); }
                }}
              />
            </Show>

            {/* ── Ask Claude ── */}
            <button class="fd-ask" onClick={() => askClaude(f())}>
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M21 15a2 2 0 0 1-2 2H8l-4 4V5a2 2 0 0 1 2-2h13a2 2 0 0 1 2 2z" />
              </svg>
              Ask Claude about this feature
            </button>

            {/* ── Entry points ── */}
            <CollapsibleSection label="Entry points" count={f().entryPoints.length}>
              <Show
                when={f().entryPoints.length > 0}
                fallback={<div class="fd-hint">No entry points recorded.</div>}
              >
                <div class="fd-entries">
                  <For each={f().entryPoints}>
                    {(path) => (
                      <button class="fd-entry" onClick={() => open(path)} title={`Open ${path}`}>
                        <span class="fd-entry-path">{path}</span>
                        <svg class="fd-entry-open" width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                          <path d="M14 4h6v6M20 4l-9 9M18 13v6H5V6h6" />
                        </svg>
                      </button>
                    )}
                  </For>
                </div>
              </Show>
            </CollapsibleSection>

            {/* ── Files ── */}
            <CollapsibleSection label="Files" count={f().files.length}>
              <Show when={f().files.length > 0} fallback={<div class="fd-hint">No files mapped.</div>}>
                <FilesSection
                  files={f().files}
                  allFeatures={store.features}
                  currentSlug={f().slug}
                  onOpen={open}
                />
              </Show>
            </CollapsibleSection>

            {/* ── Living doc — the generated .featureforge/docs/<slug>.md ── */}
            <CollapsibleSection label="Living doc">
              <Show
                when={livingDoc()}
                fallback={
                  <div class="fd-doc-empty">
                    <div class="fd-hint">This feature has no living doc yet.</div>
                    <button class="fd-doc-index" onClick={() => void appStore.reindex()}>
                      Generate docs
                    </button>
                  </div>
                }
              >
                <div class="fd-doc md-render" innerHTML={livingDoc()} />
              </Show>
            </CollapsibleSection>

            {/* ── Recent activity ── */}
            <CollapsibleSection label="Recent activity" count={recent().length}>
              <RecentActivity events={recent()} now={now()} />
            </CollapsibleSection>
          </div>
        )}
      </Show>

      <style>{`
        .fd { flex: 1; display: flex; flex-direction: column; min-height: 0; }
        .fd-inner {
          max-width: 768px; width: 100%; margin: 0 auto;
          padding: var(--space-6) var(--space-4) var(--space-10);
          animation: fade-slide-up 0.22s var(--ease-out) both;
        }
        .fd-empty { margin: auto; text-align: center; max-width: 320px; }
        .fd-empty-title { font-size: 15px; font-weight: 600; color: var(--text-secondary); margin-bottom: 6px; }
        .fd-empty-sub { font-size: 12px; line-height: 1.5; color: var(--text-tertiary); }

        .fd-header { display: flex; align-items: center; gap: var(--space-2); }
        .fd-title {
          font-size: 18px; font-weight: 600; letter-spacing: -0.3px; color: var(--text);
          flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;
          cursor: text;
        }
        .fd-title-input {
          flex: 1; min-width: 0;
          font-family: var(--font-body);
          font-size: 18px; font-weight: 600; letter-spacing: -0.3px;
          padding: 2px 6px; margin: -2px 0;
          background: var(--bg-muted);
        }
        .fd-pin {
          display: flex; align-items: center; flex-shrink: 0;
          padding: var(--space-1); border-radius: var(--radius-sm);
          color: var(--text-tertiary);
        }
        .fd-pin:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .fd-pin--on, .fd-pin--on:hover { color: var(--amber); }

        .fd-chip {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 1px 6px; border-radius: var(--radius-pill);
          text-transform: uppercase; letter-spacing: 0.04em; flex-shrink: 0;
        }
        .fd-confidence { font-variant-numeric: tabular-nums; }
        .fd-tags { display: flex; gap: var(--space-1); margin-top: var(--space-3); flex-wrap: wrap; }

        .fd-description {
          margin-top: var(--space-3);
          font-size: 13px; line-height: 1.6; color: var(--text-secondary);
          border-radius: var(--radius-sm); padding: 4px 6px; margin-left: -6px; margin-right: -6px;
          cursor: text;
        }
        .fd-description:hover { background: var(--bg-hover); }
        .fd-description--empty { color: var(--text-tertiary); font-style: italic; }
        .fd-desc-input {
          margin-top: var(--space-3);
          width: 100%;
          font-family: var(--font-body); font-size: 13px; line-height: 1.6;
          padding: 8px 10px; resize: vertical;
        }

        .fd-ask {
          display: inline-flex; align-items: center; gap: 8px;
          margin-top: var(--space-5);
          padding: 8px 16px;
          font-size: 13px; font-weight: 600;
          color: #16202e; background: var(--primary);
          border-radius: var(--radius-md);
        }
        .fd-ask:hover { filter: brightness(1.08); }

        .fd-hint { font-size: 12px; color: var(--text-tertiary); }

        .fd-entries { display: flex; flex-direction: column; gap: 2px; }
        .fd-entry {
          display: flex; align-items: center; gap: var(--space-2);
          width: 100%; padding: 6px 10px;
          border-radius: var(--radius-sm);
          background: rgba(var(--primary-rgb), 0.05);
          border: 1px solid rgba(var(--primary-rgb), 0.15);
          font-family: var(--font-mono); font-size: 11.5px; color: var(--primary);
          text-align: left;
        }
        .fd-entry:hover { background: rgba(var(--primary-rgb), 0.1); border-color: rgba(var(--primary-rgb), 0.3); }
        .fd-entry-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .fd-entry-open { flex-shrink: 0; opacity: 0; }
        .fd-entry:hover .fd-entry-open { opacity: 0.8; }

        .fd-doc { font-size: 13px; line-height: 1.6; color: var(--text-secondary); }
        .fd-doc :is(h1, h2, h3) { color: var(--text); font-weight: 600; margin: 0.6em 0 0.3em; }
        .fd-doc h1 { font-size: 1.25em; } .fd-doc h2 { font-size: 1.15em; } .fd-doc h3 { font-size: 1.05em; }
        .fd-doc p { margin: 0.5em 0; }
        .fd-doc ul, .fd-doc ol { margin: 0.5em 0; padding-left: 1.3em; }
        .fd-doc code {
          font-family: var(--font-mono); font-size: 0.88em;
          background: var(--hljs-code-bg); padding: 1px 5px; border-radius: var(--radius-sm);
          color: var(--hljs-inline-code);
        }
        .fd-doc a { color: var(--primary); }
        .fd-doc-empty { display: flex; align-items: center; gap: var(--space-3); }
        .fd-doc-index {
          font-size: 11px; font-weight: 500; font-family: var(--font-mono);
          padding: 3px 10px; border-radius: var(--radius-sm);
          color: var(--primary); background: var(--bg-accent);
          border: 1px solid var(--border-glow);
        }
        .fd-doc-index:hover { background: var(--primary-glow); }
      `}</style>
    </div>
  );
}
