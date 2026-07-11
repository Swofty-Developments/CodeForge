/* Cmd+K palette — 520px, 22vh top, blur(8px) backdrop (design-system §5.4).
 * Fuzzy subsequence filter with matched-char highlighting; arrow/enter nav.
 * Registers no global shortcuts — App.tsx owns Cmd+K and Escape. */

import { For, Show, createEffect, createMemo, createSignal, onMount } from "solid-js";
import { open } from "@tauri-apps/plugin-dialog";
import { appStore } from "../stores/app-store";
import { getFeatureDoc, getTimeline } from "../ipc";

type Category = "action" | "view" | "feature" | "file" | "doc" | "note";

interface Cmd {
  id: string;
  label: string;
  category: Category;
  run: () => void | Promise<void>;
}

/** How many rows each content group (files / docs / notes) may contribute. */
const MAX_PER_GROUP = 8;
/** Content search kicks in at this query length; below it, commands + features only. */
const MIN_CONTENT_QUERY = 2;

/** Greedy left-to-right subsequence match. `[]` = matches all (empty query). */
function fuzzyMatch(query: string, text: string): number[] | null {
  if (!query) return [];
  const q = query.toLowerCase();
  const t = text.toLowerCase();
  const hits: number[] = [];
  let qi = 0;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) { hits.push(ti); qi++; }
  }
  return qi === q.length ? hits : null;
}

/** Case-insensitive substring match returning the matched index range. */
function substrMatch(query: string, text: string): number[] | null {
  const start = text.toLowerCase().indexOf(query.toLowerCase());
  if (start < 0) return null;
  return Array.from({ length: query.length }, (_, i) => start + i);
}

/** First line of `body` containing `query` (case-insensitive), trimmed. */
function matchingLine(body: string, query: string): string | null {
  const q = query.toLowerCase();
  for (const line of body.split("\n")) {
    if (line.toLowerCase().includes(q)) return line.trim();
  }
  return null;
}

function truncate(text: string, max: number): string {
  const oneLine = text.replace(/\s+/g, " ").trim();
  return oneLine.length > max ? `${oneLine.slice(0, max)}…` : oneLine;
}

function Highlight(props: { text: string; match: number[] }) {
  const chars = createMemo(() => {
    const set = new Set(props.match);
    return [...props.text].map((ch, i) => ({ ch, hit: set.has(i) }));
  });
  return (
    <For each={chars()}>
      {(c) => (c.hit ? <span class="cmd-hl">{c.ch}</span> : c.ch)}
    </For>
  );
}

export function CommandPalette() {
  const { store } = appStore;
  const [query, setQuery] = createSignal("");
  const [selected, setSelected] = createSignal(0);
  // Content-search corpus, loaded once per palette open: living-doc bodies and
  // timeline notes. `corpusError` is a named state shown at the list foot —
  // command/feature search still works without the corpus.
  const [docs, setDocs] = createSignal<{ slug: string; name: string; body: string }[]>([]);
  const [notes, setNotes] = createSignal<{ id: number; text: string }[]>([]);
  const [corpusError, setCorpusError] = createSignal<string | null>(null);
  let inputRef: HTMLInputElement | undefined;
  let listRef: HTMLDivElement | undefined;

  async function openRepoDialog(): Promise<void> {
    const dir = await open({ directory: true, multiple: false });
    if (typeof dir === "string") await appStore.openRepo(dir);
  }

  const baseCmds: Cmd[] = [
    { id: "open-repo", label: "Open repository…", category: "action", run: openRepoDialog },
    { id: "reindex", label: "Reindex repository", category: "action", run: () => appStore.reindex(false) },
    { id: "new-session", label: "New session", category: "action", run: () => appStore.startSession() },
    { id: "toggle-sessions", label: "Toggle session pane", category: "action", run: () => appStore.toggleSessionPane() },
    { id: "view-feature", label: "Switch to Feature view", category: "view", run: () => appStore.setActiveView("feature") },
    { id: "view-timeline", label: "Switch to Timeline view", category: "view", run: () => appStore.setActiveView("timeline") },
    { id: "view-diff", label: "Switch to Diff view", category: "view", run: () => appStore.setActiveView("diff") },
  ];

  const results = createMemo<{ cmd: Cmd; match: number[] }[]>(() => {
    const q = query().trim();
    const openFeature = (slug: string) => () => appStore.openFeatureDetail(slug);
    const features: Cmd[] = store.features.map((f) => ({
      id: `feature-${f.slug}`,
      label: f.name,
      category: "feature" as const,
      run: openFeature(f.slug),
    }));
    const out: { cmd: Cmd; match: number[] }[] = [];
    for (const cmd of [...baseCmds, ...features]) {
      const match = fuzzyMatch(q, cmd.label);
      if (match) out.push({ cmd, match });
    }
    if (q.length < MIN_CONTENT_QUERY) return out;

    // Features whose *description or tags* match when the name fuzzy-miss'd.
    for (const f of store.features) {
      if (fuzzyMatch(q, f.name)) continue;
      if (substrMatch(q, f.description) || substrMatch(q, f.tags.join(" "))) {
        out.push({
          cmd: { id: `feature-${f.slug}`, label: f.name, category: "feature", run: openFeature(f.slug) },
          match: [],
        });
      }
    }

    // File paths across all features (deduped; a shared file lands on its first owner).
    const seenPaths = new Set<string>();
    let nFiles = 0;
    outer: for (const f of store.features) {
      for (const file of f.files) {
        if (seenPaths.has(file.path)) continue;
        seenPaths.add(file.path);
        const match = fuzzyMatch(q, file.path);
        if (!match) continue;
        out.push({
          cmd: { id: `file-${file.path}`, label: file.path, category: "file", run: openFeature(f.slug) },
          match,
        });
        if (++nFiles >= MAX_PER_GROUP) break outer;
      }
    }

    // Living-doc bodies: show the first matching line under the feature's name.
    let nDocs = 0;
    for (const d of docs()) {
      const line = matchingLine(d.body, q);
      if (line === null) continue;
      const label = `${d.name} — ${truncate(line, 64)}`;
      out.push({
        cmd: { id: `doc-${d.slug}`, label, category: "doc", run: openFeature(d.slug) },
        match: substrMatch(q, label) ?? [],
      });
      if (++nDocs >= MAX_PER_GROUP) break;
    }

    // Timeline notes; selecting one jumps to the timeline view.
    let nNotes = 0;
    for (const n of notes()) {
      if (!substrMatch(q, n.text)) continue;
      const label = truncate(n.text, 72);
      out.push({
        cmd: { id: `note-${n.id}`, label, category: "note", run: () => appStore.setActiveView("timeline") },
        match: substrMatch(q, label) ?? [],
      });
      if (++nNotes >= MAX_PER_GROUP) break;
    }
    return out;
  });

  function runItem(r: { cmd: Cmd } | undefined): void {
    if (!r) return;
    appStore.setPaletteOpen(false);
    void r.cmd.run();
  }

  function onKeyDown(e: KeyboardEvent): void {
    const n = results().length;
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelected((s) => (n === 0 ? 0 : (s + 1) % n));
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelected((s) => (n === 0 ? 0 : (s - 1 + n) % n));
    } else if (e.key === "Enter") {
      e.preventDefault();
      runItem(results()[selected()]);
    }
    // Escape intentionally left to App.tsx's global handler.
  }

  // Keep the highlighted row in view as selection moves.
  createEffect(() => {
    selected();
    queueMicrotask(() => listRef?.querySelector(".cmd-palette-item.selected")?.scrollIntoView({ block: "nearest" }));
  });

  onMount(() => {
    inputRef?.focus();
    const repoPath = store.repo?.path;
    if (!repoPath) return;
    void Promise.all(
      store.features.map(async (f) => ({
        slug: f.slug,
        name: f.name,
        body: (await getFeatureDoc(repoPath, f.slug)) ?? "",
      }))
    )
      .then((ds) => setDocs(ds.filter((d) => d.body.length > 0)))
      .catch((e) => setCorpusError(String(e)));
    void getTimeline(repoPath, { kinds: ["note"], limit: 200 })
      .then((evs) =>
        setNotes(evs.flatMap((ev) => (ev.kind === "note" ? [{ id: ev.id, text: ev.payload.text }] : [])))
      )
      .catch((e) => setCorpusError(String(e)));
  });

  return (
    <div class="cmd-palette-overlay" onClick={() => appStore.setPaletteOpen(false)}>
      <div class="cmd-palette" onClick={(e) => e.stopPropagation()}>
        <div class="cmd-palette-input-wrap">
          <svg class="cmd-palette-search" width="15" height="15" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
            <circle cx="11" cy="11" r="7" /><path d="M21 21l-4.3-4.3" />
          </svg>
          <input
            ref={inputRef}
            class="cmd-palette-input"
            placeholder="Search commands, features, files, docs, notes…"
            value={query()}
            onInput={(e) => { setQuery(e.currentTarget.value); setSelected(0); }}
            onKeyDown={onKeyDown}
          />
          <span class="cmd-palette-kbd">esc</span>
        </div>
        <div class="cmd-palette-list" ref={listRef}>
          <For each={results()}>
            {(r, i) => (
              <div
                class="cmd-palette-item"
                classList={{ selected: selected() === i() }}
                onMouseEnter={() => setSelected(i())}
                onClick={() => runItem(r)}
              >
                <span class="cmd-palette-item-label">
                  <Highlight text={r.cmd.label} match={r.match} />
                </span>
                <span
                  class="cmd-palette-item-category"
                  classList={{
                    "cat-action": r.cmd.category === "action",
                    "cat-view": r.cmd.category === "view",
                    "cat-feature": r.cmd.category === "feature",
                    "cat-file": r.cmd.category === "file",
                    "cat-doc": r.cmd.category === "doc",
                    "cat-note": r.cmd.category === "note",
                  }}
                >
                  {r.cmd.category}
                </span>
              </div>
            )}
          </For>
          <Show when={results().length === 0}>
            <div class="cmd-palette-empty">No matches</div>
          </Show>
          <Show when={corpusError()}>
            <div class="cmd-palette-corpus-err">Doc/note search unavailable: {corpusError()}</div>
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
          background: var(--bg-elevated);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          box-shadow: 0 24px 64px rgba(0, 0, 0, 0.6), 0 0 0 1px rgba(255, 255, 255, 0.03);
          display: flex; flex-direction: column;
          overflow: hidden;
          animation: cmd-slide-in 0.2s var(--ease-out);
        }
        .cmd-palette-input-wrap {
          display: flex; align-items: center; gap: var(--space-3);
          padding: var(--space-4);
          border-bottom: 1px solid var(--border);
        }
        .cmd-palette-search { color: var(--text-tertiary); flex-shrink: 0; }
        .cmd-palette-input {
          flex: 1;
          background: transparent; border: none; outline: none; box-shadow: none;
          color: var(--text);
          font-size: 14px;
          font-family: var(--font-body);
          caret-color: var(--primary);
          padding: 0;
        }
        .cmd-palette-input::placeholder { color: var(--text-tertiary); }
        .cmd-palette-kbd {
          font-size: 10px;
          color: var(--text-tertiary);
          background: var(--bg-muted);
          border: 1px solid var(--border);
          border-radius: var(--radius-sm);
          padding: 2px 6px;
          flex-shrink: 0;
        }
        .cmd-palette-list { overflow-y: auto; padding: var(--space-1); max-height: 350px; }
        .cmd-palette-item {
          display: flex; align-items: center; justify-content: space-between; gap: var(--space-3);
          padding: 5px var(--space-3);
          border-radius: var(--radius-sm);
          cursor: pointer;
        }
        .cmd-palette-item.selected { background: var(--bg-accent); }
        .cmd-palette-item-label { color: var(--text-secondary); font-size: 13px; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
        .cmd-palette-item.selected .cmd-palette-item-label { color: var(--text); }
        .cmd-hl { color: var(--primary); font-weight: 600; }
        .cmd-palette-item-category {
          font-size: 10px; padding: 2px 8px; border-radius: var(--radius-sm);
          font-weight: 600; letter-spacing: 0.02em; flex-shrink: 0;
        }
        .cat-action  { color: var(--green);   background: rgba(var(--green-rgb), 0.1); }
        .cat-view    { color: var(--amber);   background: rgba(var(--amber-rgb), 0.1); }
        .cat-feature { color: var(--primary); background: rgba(var(--primary-rgb), 0.1); font-family: var(--font-mono); }
        .cat-file    { color: var(--sky);     background: rgba(var(--sky-rgb), 0.1); font-family: var(--font-mono); }
        .cat-doc     { color: var(--purple);  background: rgba(var(--purple-rgb), 0.1); }
        .cat-note    { color: var(--text-secondary); background: var(--bg-muted); }
        .cmd-palette-empty { padding: var(--space-6); text-align: center; color: var(--text-tertiary); font-size: 13px; }
        .cmd-palette-corpus-err {
          padding: var(--space-2) var(--space-3);
          border-top: 1px solid var(--border);
          color: var(--text-tertiary);
          font-size: 11px;
        }
      `}</style>
    </div>
  );
}
