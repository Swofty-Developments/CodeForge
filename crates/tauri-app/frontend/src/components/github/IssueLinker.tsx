import { createSignal, For, Show, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { Issue } from "../../ipc";

interface Props {
  threadId: string;
  repoPath: string;
  linkedIssues: number[];
  onLink: (issueNumber: number) => void;
  onUnlink: (issueNumber: number) => void;
}

export function IssueLinker(props: Props) {
  const [open, setOpen] = createSignal(false);
  const [search, setSearch] = createSignal("");
  const [results, setResults] = createSignal<Issue[]>([]);
  const [searching, setSearching] = createSignal(false);
  const [fetchingContext, setFetchingContext] = createSignal<number | null>(null);

  // Click outside to close
  let containerRef: HTMLDivElement | undefined;
  function handleClickOutside(e: MouseEvent) {
    if (containerRef && !containerRef.contains(e.target as Node)) {
      setOpen(false);
    }
  }
  // Register/unregister based on open state
  function watchOpen(isOpen: boolean) {
    if (isOpen) {
      document.addEventListener("mousedown", handleClickOutside);
    } else {
      document.removeEventListener("mousedown", handleClickOutside);
    }
  }
  onCleanup(() => document.removeEventListener("mousedown", handleClickOutside));

  async function handleSearch() {
    const q = search().trim();
    if (!q) return;
    setSearching(true);
    try {
      const issues = await ipc.listIssues(props.repoPath, "open", q);
      setResults(issues.filter((i) => !props.linkedIssues.includes(i.number)));
    } catch (e) {
      console.error("Issue search failed:", e);
      setResults([]);
    } finally {
      setSearching(false);
    }
  }

  async function linkIssue(issue: Issue) {
    setFetchingContext(issue.number);
    try {
      // Fetch full context
      const context = await ipc.getIssueContext(props.repoPath, issue.number);

      // Store the link
      const current = props.linkedIssues;
      const updated = [...current, issue.number];
      await ipc.setSetting(`issues:${props.threadId}`, updated.join(","));
      props.onLink(issue.number);

      // Add context as a system message in the thread
      const { store } = appStore;
      const msgId = await ipc.persistUserMessage(props.threadId, `[Linked Issue #${issue.number}]\n\n${context}`);
      appStore.setStore("threadMessages", props.threadId, (msgs) => [
        ...(msgs || []),
        {
          id: msgId,
          thread_id: props.threadId,
          role: "user" as const,
          content: `[Linked Issue #${issue.number}]\n\n${context}`,
        },
      ]);

      // Remove from results
      setResults((r) => r.filter((i) => i.number !== issue.number));
    } catch (e) {
      console.error("Failed to link issue:", e);
    } finally {
      setFetchingContext(null);
    }
  }

  async function unlinkIssue(num: number) {
    const updated = props.linkedIssues.filter((n) => n !== num);
    await ipc.setSetting(`issues:${props.threadId}`, updated.join(","));
    props.onUnlink(num);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      handleSearch();
    }
    if (e.key === "Escape") {
      setOpen(false);
    }
  }

  return (
    <div class="il" ref={containerRef}>
      {/* Linked issue pills */}
      <Show when={props.linkedIssues.length > 0}>
        <div class="il-linked">
          <For each={props.linkedIssues}>
            {(num) => (
              <span class="il-pill">
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2.5" stroke-linecap="round"><circle cx="12" cy="12" r="6" /></svg>
                #{num}
                <button class="il-pill-x" onClick={() => unlinkIssue(num)}>
                  <svg width="8" height="8" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" /></svg>
                </button>
              </span>
            )}
          </For>
        </div>
      </Show>

      {/* Link button */}
      <button class="il-trigger" onClick={() => { const next = !open(); setOpen(next); watchOpen(next); }}>
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
          <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" />
          <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" />
        </svg>
        Link issue
      </button>

      {/* Search popup */}
      <Show when={open()}>
        <div class="il-popup">
          <div class="il-search-row">
            <input
              class="il-input"
              placeholder="Search issues…"
              value={search()}
              onInput={(e) => setSearch(e.currentTarget.value)}
              onKeyDown={handleKeyDown}
              autofocus
            />
            <button class="il-search-btn" onClick={handleSearch} disabled={searching()}>
              {searching() ? "…" : "Search"}
            </button>
          </div>
          <div class="il-results">
            <Show when={results().length === 0 && !searching()}>
              <div class="il-empty">Type to search issues</div>
            </Show>
            <For each={results()}>
              {(issue) => (
                <button
                  class="il-result"
                  onClick={() => linkIssue(issue)}
                  disabled={fetchingContext() === issue.number}
                >
                  <span class="il-result-num">#{issue.number}</span>
                  <span class="il-result-title">{issue.title}</span>
                  <Show when={issue.labels.length > 0}>
                    <span class="il-result-labels">
                      {issue.labels.slice(0, 2).join(", ")}
                    </span>
                  </Show>
                  <span class="il-result-action">
                    {fetchingContext() === issue.number ? "Fetching…" : "Link"}
                  </span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("il-styles")) {
  const s = document.createElement("style");
  s.id = "il-styles";
  s.textContent = `
    .il { position: relative; display: inline-flex; align-items: center; gap: 6px; flex-wrap: wrap; }
    .il-linked { display: flex; gap: 4px; flex-wrap: wrap; }
    .il-pill {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-pill);
      padding: 2px 6px 2px 5px;
      font-family: var(--font-mono);
    }
    .il-pill-x {
      width: 14px; height: 14px;
      display: flex; align-items: center; justify-content: center;
      border-radius: 50%;
      color: var(--text-tertiary);
      transition: all 0.1s;
    }
    .il-pill-x:hover { background: rgba(242,95,103,0.15); color: var(--red); }
    .il-trigger {
      display: inline-flex;
      align-items: center;
      gap: 4px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-tertiary);
      padding: 3px 8px;
      border-radius: var(--radius-pill);
      transition: all 0.1s;
    }
    .il-trigger:hover { color: var(--text-secondary); background: var(--bg-hover); }
    .il-popup {
      position: absolute;
      top: 100%;
      left: 0;
      margin-top: 4px;
      width: 340px;
      background: var(--bg-card);
      border: 1px solid var(--border-strong);
      border-radius: var(--radius-md);
      box-shadow: 0 8px 32px rgba(0,0,0,0.4);
      z-index: 50;
      padding: 6px;
    }
    .il-search-row { display: flex; gap: 4px; margin-bottom: 4px; }
    .il-input {
      flex: 1;
      font-size: 12px;
      padding: 6px 8px;
      background: var(--bg-base);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      font-family: var(--font-body);
      outline: none;
    }
    .il-input:focus { border-color: var(--primary); }
    .il-input::placeholder { color: var(--text-tertiary); }
    .il-search-btn {
      font-size: 11px;
      font-weight: 500;
      padding: 4px 10px;
      background: var(--primary);
      color: white;
      border-radius: var(--radius-sm);
    }
    .il-search-btn:disabled { opacity: 0.5; }
    .il-results { max-height: 200px; overflow-y: auto; }
    .il-empty {
      font-size: 11px;
      color: var(--text-tertiary);
      padding: 12px;
      text-align: center;
    }
    .il-result {
      display: flex;
      align-items: center;
      gap: 6px;
      width: 100%;
      padding: 6px 8px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      text-align: left;
      transition: background 0.08s;
    }
    .il-result:hover { background: var(--bg-accent); }
    .il-result:disabled { opacity: 0.5; }
    .il-result-num {
      font-family: var(--font-mono);
      font-weight: 600;
      color: var(--primary);
      flex-shrink: 0;
      font-size: 11px;
    }
    .il-result-title {
      flex: 1;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      color: var(--text);
    }
    .il-result-labels {
      font-size: 9px;
      color: var(--text-tertiary);
      flex-shrink: 0;
    }
    .il-result-action {
      font-size: 10px;
      font-weight: 500;
      color: var(--primary);
      flex-shrink: 0;
    }
  `;
  document.head.appendChild(s);
}
