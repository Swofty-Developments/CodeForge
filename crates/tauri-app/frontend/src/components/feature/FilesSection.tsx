/* Files section — the feature's files grouped by FileRole, each with a role
 * tint chip and a purple "also in N" chip when the path participates in other
 * features. Rows open the file in the OS editor. */

import { For, Show, createMemo } from "solid-js";
import type { Feature, FeatureFile, FileRole } from "../../types";

const ROLE_ORDER: FileRole[] = ["core", "support", "test", "config"];
const ROLE_TINT: Record<FileRole, string> = {
  core: "tint-primary",
  support: "tint-sky",
  test: "tint-green",
  config: "tint-amber",
};

export function FilesSection(props: {
  files: FeatureFile[];
  allFeatures: Feature[];
  currentSlug: string;
  onOpen: (path: string) => void;
}) {
  const grouped = createMemo(() => {
    const g = new Map<FileRole, FeatureFile[]>();
    for (const file of props.files) {
      const list = g.get(file.role) ?? [];
      list.push(file);
      g.set(file.role, list);
    }
    return ROLE_ORDER.filter((r) => g.has(r)).map((role) => ({ role, files: g.get(role)! }));
  });

  // How many OTHER features reference each path (many-to-many overlap).
  const alsoIn = createMemo(() => {
    const counts = new Map<string, number>();
    for (const feat of props.allFeatures) {
      if (feat.slug === props.currentSlug) continue;
      for (const ff of feat.files) counts.set(ff.path, (counts.get(ff.path) ?? 0) + 1);
    }
    return counts;
  });

  return (
    <div class="fl">
      <For each={grouped()}>
        {(group) => (
          <div class="fl-group">
            <div class="fl-group-head">
              <span class={`fd-chip ${ROLE_TINT[group.role]}`}>{group.role}</span>
              <span class="fl-group-count">{group.files.length}</span>
            </div>
            <For each={group.files}>
              {(file) => {
                const shared = () => alsoIn().get(file.path) ?? 0;
                return (
                  <button class="fl-row" onClick={() => props.onOpen(file.path)} title={`Open ${file.path}`}>
                    <span class="fl-path">{file.path}</span>
                    <Show when={shared() > 0}>
                      <span class="fd-chip tint-purple fl-shared">also in {shared()}</span>
                    </Show>
                    <svg
                      class="fl-open" width="11" height="11" viewBox="0 0 24 24"
                      fill="none" stroke="currentColor" stroke-width="2"
                    >
                      <path d="M14 4h6v6M20 4l-9 9M18 13v6H5V6h6" />
                    </svg>
                  </button>
                );
              }}
            </For>
          </div>
        )}
      </For>

      <style>{`
        .fl { display: flex; flex-direction: column; gap: var(--space-3); }
        .fl-group { display: flex; flex-direction: column; gap: 2px; }
        .fl-group-head { display: flex; align-items: center; gap: var(--space-2); margin-bottom: 2px; }
        .fl-group-count { font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary); }

        .fl-row {
          display: flex; align-items: center; gap: var(--space-2);
          width: 100%;
          padding: 6px 10px;
          border-radius: var(--radius-sm);
          background: rgba(255, 255, 255, 0.02);
          border: 1px solid var(--border);
          font-family: var(--font-mono); font-size: 11.5px;
          color: var(--text-secondary);
          text-align: left;
          transition: background 0.15s, border-color 0.15s, color 0.15s;
        }
        .fl-row:hover { background: var(--bg-hover); border-color: var(--border-strong); color: var(--text); }
        .fl-path { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .fl-shared { flex-shrink: 0; }
        .fl-open { flex-shrink: 0; color: var(--text-tertiary); opacity: 0; transition: opacity 0.15s; }
        .fl-row:hover .fl-open { opacity: 0.8; }

        .fd-chip {
          font-size: 9px; font-weight: 500; font-family: var(--font-mono);
          padding: 1px 6px; border-radius: var(--radius-pill);
          text-transform: uppercase; letter-spacing: 0.04em;
        }
      `}</style>
    </div>
  );
}
