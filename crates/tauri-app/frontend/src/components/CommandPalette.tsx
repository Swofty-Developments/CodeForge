/* Cmd+K palette — 520px, 22vh top, blur(8px) backdrop, per the design system. */

import { For, Show, createMemo, createSignal, onMount } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { appStore } from "../stores/app-store";

interface PaletteItem {
  id: string;
  label: string;
  category: "action" | "view" | "feature";
  run: () => void | Promise<void>;
}

export function CommandPalette() {
  const { store } = appStore;
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal(0);
  let inputRef: HTMLInputElement | undefined;

  const items = createMemo<PaletteItem[]>(() => {
    const base: PaletteItem[] = [
      {
        id: "open-repo",
        label: "Open repository…",
        category: "action",
        run: async () => {
          const dir = await open({ directory: true, multiple: false });
          if (typeof dir === "string") await appStore.openRepo(dir);
        },
      },
      { id: "reindex", label: "Re-index repository", category: "action", run: () => appStore.reindex(false) },
      { id: "view-timeline", label: "Go to Timeline", category: "view", run: () => appStore.setActiveView("timeline") },
      { id: "view-diff", label: "Go to Diff review", category: "view", run: () => appStore.setActiveView("diff") },
      { id: "toggle-sessions", label: "Toggle session pane", category: "view", run: () => appStore.toggleSessionPane() },
    ];
    const features: PaletteItem[] = store.features.map((f) => ({
      id: `feature-${f.slug}`,
      label: f.name,
      category: "feature" as const,
      run: () => appStore.selectFeature(f.slug),
    }));
    const q = query().toLowerCase();
    const all = [...base, ...features];
    return q ? all.filter((i) => i.label.toLowerCase().includes(q)) : all;
  });

  function runItem(item: PaletteItem | undefined) {
    if (!item) return;
    appStore.setPaletteOpen(false);
    void item.run();
  }

  function onKeyDown(e: KeyboardEvent) {
    const n = items().length;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => (n === 0 ? 0 : (s + 1) % n));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => (n === 0 ? 0 : (s - 1 + n) % n));
    } else if (e.key === "Enter") {
      e.preventDefault();
      runItem(items()[selected()]);
    }
  }

  onMount(() => inputRef?.focus());

  return (
    <div class="cmd-palette-overlay" onClick={() => appStore.setPaletteOpen(false)}>
      <div class="cmd-palette" onClick={(e) => e.stopPropagation()}>
        <div class="cmd-palette-input-wrap">
          <input
            ref={inputRef}
            class="cmd-palette-input"
            placeholder="Type a command or feature…"
            value={query()}
            onInput={(e) => {
              setQuery(e.currentTarget.value);
              setSelected(0);
            }}
            onKeyDown={onKeyDown}
          />
          <span class="cmd-palette-kbd">esc</span>
        </div>
        <div class="cmd-palette-list">
          <For each={items()}>
            {(item, i) => (
              <div
                class="cmd-palette-item"
                classList={{ selected: selected() === i() }}
                onMouseEnter={() => setSelected(i())}
                onClick={() => runItem(item)}
              >
                <span class="cmd-palette-item-label">{item.label}</span>
                <span class="cmd-palette-item-category" classList={{
                  "cat-action": item.category === "action",
                  "cat-view": item.category === "view",
                  "cat-feature": item.category === "feature",
                }}>
                  {item.category}
                </span>
              </div>
            )}
          </For>
          <Show when={items().length === 0}>
            <div class="cmd-palette-empty">No matches</div>
          </Show>
        </div>
      </div>

      <style>{`
        .cmd-palette-overlay {
          position: fixed;
          inset: 0;
          z-index: 9999;
          background: rgba(0, 0, 0, 0.6);
          display: flex;
          align-items: flex-start;
          justify-content: center;
          padding-top: 22vh;
          backdrop-filter: blur(8px);
          -webkit-backdrop-filter: blur(8px);
          animation: cmd-fade-in 0.1s ease-out;
        }
        .cmd-palette {
          width: 520px;
          max-height: 420px;
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: 12px;
          box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255,255,255,0.03);
          display: flex; flex-direction: column;
          overflow: hidden;
          animation: cmd-slide-in 0.15s var(--ease-out);
        }
        .cmd-palette-input-wrap {
          display: flex; align-items: center; gap: var(--space-3);
          padding: var(--space-4);
          border-bottom: 1px solid var(--border);
        }
        .cmd-palette-input {
          flex: 1;
          background: transparent; border: none; outline: none; box-shadow: none;
          color: var(--text);
          font-size: 15px;
          font-family: var(--font-body);
          caret-color: var(--primary);
          padding: 0;
        }
        .cmd-palette-kbd {
          font-size: 10px;
          color: var(--text-tertiary);
          background: var(--bg-accent);
          border: 1px solid var(--border);
          border-radius: 4px;
          padding: 2px 6px;
        }
        .cmd-palette-list { overflow-y: auto; padding: var(--space-1); max-height: 350px; }
        .cmd-palette-item {
          display: flex; align-items: center; justify-content: space-between;
          padding: var(--space-2) var(--space-3);
          border-radius: var(--radius-sm);
          cursor: pointer;
          transition: background 0.08s;
        }
        .cmd-palette-item.selected { background: var(--bg-accent); }
        .cmd-palette-item-label { color: var(--text-secondary); font-size: 13px; }
        .cmd-palette-item.selected .cmd-palette-item-label { color: var(--text); }
        .cmd-palette-item-category {
          font-size: 10px; padding: 2px 8px; border-radius: 4px;
          font-weight: 600; letter-spacing: 0.02em;
        }
        .cat-action  { color: var(--green);   background: rgba(var(--green-rgb), 0.1); }
        .cat-view    { color: var(--amber);   background: rgba(var(--amber-rgb), 0.1); }
        .cat-feature { color: var(--primary); background: rgba(var(--primary-rgb), 0.1); }
        .cmd-palette-empty { padding: var(--space-6); text-align: center; color: var(--text-tertiary); font-size: 13px; }
      `}</style>
    </div>
  );
}
