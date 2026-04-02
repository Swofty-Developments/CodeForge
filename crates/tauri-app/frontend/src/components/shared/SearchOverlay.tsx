import { onCleanup, For, Show, createEffect } from "solid-js";
import { createSignal } from "solid-js";
import { searchMessages, type SearchResult } from "../../ipc";
import { appStore } from "../../stores/app-store";

interface GroupedResults {
  thread_id: string;
  thread_title: string;
  project_name: string;
  messages: SearchResult[];
}

export function SearchOverlay(props?: { inline?: boolean }) {
  const { store, setStore } = appStore;
  const [query, setQuery] = createSignal("");
  const [results, setResults] = createSignal<SearchResult[]>([]);
  const [loading, setLoading] = createSignal(false);
  let inputRef: HTMLInputElement | undefined;
  let debounceTimer: ReturnType<typeof setTimeout> | undefined;

  onCleanup(() => {
    if (debounceTimer) clearTimeout(debounceTimer);
  });

  createEffect(() => {
    if (store.searchOpen) {
      setTimeout(() => inputRef?.focus(), 50);
    } else {
      setQuery("");
      setResults([]);
    }
  });

  function close() {
    setStore("searchOpen", false);
  }

  function onInput(value: string) {
    setQuery(value);
    if (debounceTimer) clearTimeout(debounceTimer);
    if (value.trim().length < 2) {
      setResults([]);
      return;
    }
    debounceTimer = setTimeout(async () => {
      setLoading(true);
      try {
        const res = await searchMessages(value);
        setResults(res);
      } catch (err) {
        console.error("Search failed:", err);
        setResults([]);
      } finally {
        setLoading(false);
      }
    }, 300);
  }

  function groupedResults(): GroupedResults[] {
    const groups = new Map<string, GroupedResults>();
    for (const r of results()) {
      if (!groups.has(r.thread_id)) {
        groups.set(r.thread_id, {
          thread_id: r.thread_id,
          thread_title: r.thread_title,
          project_name: r.project_name,
          messages: [],
        });
      }
      groups.get(r.thread_id)!.messages.push(r);
    }
    return Array.from(groups.values());
  }

  function handleResultClick(result: SearchResult) {
    const { selectThread } = appStore;
    selectThread(result.thread_id);
    close();
    setTimeout(() => {
      const el = document.querySelector(`[data-message-id="${result.message_id}"]`);
      if (el) {
        el.scrollIntoView({ behavior: "smooth", block: "center" });
        el.classList.add("search-highlight-flash");
        setTimeout(() => el.classList.remove("search-highlight-flash"), 2000);
      }
    }, 300);
  }

  function highlightSnippet(snippet: string, q: string) {
    if (!q) return snippet;
    const idx = snippet.toLowerCase().indexOf(q.toLowerCase());
    if (idx === -1) return snippet;
    const before = snippet.slice(0, idx);
    const match = snippet.slice(idx, idx + q.length);
    const after = snippet.slice(idx + q.length);
    return (
      <>
        {before}
        <mark class="search-highlight">{match}</mark>
        {after}
      </>
    );
  }

  const searchContent = (
        <div class={props?.inline ? "search-panel-inline" : "search-panel"}>
          <div class="search-input-row">
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <circle cx="11" cy="11" r="8" /><line x1="21" y1="21" x2="16.65" y2="16.65" />
            </svg>
            <input
              ref={inputRef}
              class="search-input"
              type="text"
              placeholder="Search across all messages..."
              value={query()}
              onInput={(e) => onInput(e.currentTarget.value)}
              onKeyDown={(e) => { if (e.key === "Escape") close(); }}
            />
            <Show when={loading()}>
              <span class="search-loading-text">Searching...</span>
            </Show>
            <kbd class="search-kbd">ESC</kbd>
          </div>

          <div class="search-results">
            <Show
              when={results().length > 0}
              fallback={
                <Show when={query().length >= 2 && !loading()}>
                  <div class="search-empty">No results found</div>
                </Show>
              }
            >
              <For each={groupedResults()}>
                {(group) => (
                  <div class="search-group">
                    <div class="search-group-header">
                      <span class="search-group-title">{group.thread_title}</span>
                      <span class="search-group-sep">in</span>
                      <span class="search-group-project">{group.project_name}</span>
                    </div>

                    <For each={group.messages}>
                      {(result) => (
                        <button class="search-result-btn" onClick={() => handleResultClick(result)}>
                          <span class="search-result-role" classList={{ "role-user": result.role === "user", "role-assistant": result.role !== "user" }}>
                            {result.role}
                          </span>
                          <span class="search-result-snippet">
                            ...{highlightSnippet(result.content_snippet, query())}...
                          </span>
                        </button>
                      )}
                    </For>
                  </div>
                )}
              </For>
            </Show>
          </div>
        </div>
  );

  if (props?.inline) return searchContent;

  return (
    <Show when={store.searchOpen}>
      <div class="search-backdrop" onClick={(e) => { if (e.target === e.currentTarget) close(); }}>
        {searchContent}
      </div>
    </Show>
  );
}

if (!document.getElementById("search-overlay-styles")) {
  const style = document.createElement("style");
  style.id = "search-overlay-styles";
  style.textContent = `
    .search-panel-inline {
      max-width: 680px;
      margin: 0 auto;
      padding: 24px;
      width: 100%;
    }
    .search-backdrop {
      position: fixed;
      inset: 0;
      z-index: 9999;
      display: flex;
      justify-content: center;
      padding-top: 80px;
      background: rgba(0, 0, 0, 0.6);
      backdrop-filter: blur(8px);
      -webkit-backdrop-filter: blur(8px);
      animation: overlay-backdrop-in 120ms ease-out both;
    }
    .search-panel {
      width: 600px;
      max-height: 70vh;
      background: var(--bg-card);
      border-radius: 12px;
      border: 1px solid var(--border-strong);
      box-shadow: 0 24px 64px rgba(0, 0, 0, 0.5), 0 0 0 1px rgba(255,255,255,0.03);
      display: flex;
      flex-direction: column;
      overflow: hidden;
      animation: overlay-panel-in 200ms cubic-bezier(0.16, 1, 0.3, 1) both;
    }
    .search-input-row {
      padding: 14px 16px;
      border-bottom: 1px solid var(--border);
      display: flex;
      align-items: center;
      gap: 10px;
      color: var(--text-tertiary);
    }
    .search-input {
      flex: 1;
      background: transparent;
      border: none;
      outline: none;
      color: var(--text);
      font-size: 15px;
      font-family: var(--font-body);
      caret-color: var(--primary);
    }
    .search-input::placeholder { color: var(--text-tertiary); }
    .search-loading-text {
      color: var(--text-tertiary);
      font-size: 11px;
      font-weight: 500;
    }
    .search-kbd {
      padding: 2px 6px;
      background: var(--bg-accent);
      border: 1px solid var(--border);
      border-radius: 4px;
      color: var(--text-tertiary);
      font-size: 10px;
      font-family: var(--font-body);
    }
    .search-results {
      overflow-y: auto;
      flex: 1;
      padding: 6px;
    }
    .search-empty {
      padding: 24px;
      text-align: center;
      color: var(--text-tertiary);
      font-size: 13px;
    }
    .search-group { margin-bottom: 6px; }
    .search-group-header {
      padding: 8px 12px;
      display: flex;
      align-items: center;
      gap: 8px;
      font-size: 12px;
    }
    .search-group-title { color: var(--primary); font-weight: 600; }
    .search-group-sep { color: var(--text-tertiary); opacity: 0.5; }
    .search-group-project { color: var(--text-tertiary); }
    .search-result-btn {
      width: 100%;
      padding: 10px 12px;
      background: transparent;
      border: none;
      border-radius: 8px;
      cursor: pointer;
      text-align: left;
      color: var(--text-secondary);
      font-size: 13px;
      font-family: var(--font-body);
      display: flex;
      flex-direction: column;
      gap: 4px;
      transition: background 0.12s;
    }
    .search-result-btn:hover { background: var(--bg-accent); }
    .search-result-role {
      font-size: 10px;
      text-transform: uppercase;
      font-weight: 600;
      letter-spacing: 0.04em;
    }
    .search-result-role.role-user { color: var(--sky); }
    .search-result-role.role-assistant { color: var(--purple); }
    .search-result-snippet {
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
      line-height: 1.4;
    }
    .search-highlight {
      background: var(--amber);
      color: #000;
      border-radius: 2px;
      padding: 0 2px;
    }
  `;
  document.head.appendChild(style);
}
