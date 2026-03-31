import { createSignal, createEffect, For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { PullRequest } from "../../ipc";

interface Props {
  projectId: string;
  repoPath: string;
  onLinkPr?: (pr: PullRequest) => void;
}

export function PrDashboard(props: Props) {
  const { store, newThread, setStore } = appStore;
  const [prs, setPrs] = createSignal<PullRequest[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [filter, setFilter] = createSignal<"open" | "closed" | "all">("open");
  const [search, setSearch] = createSignal("");
  const [visibleCount, setVisibleCount] = createSignal(10);

  const filteredPrs = () => {
    const q = search().toLowerCase();
    if (!q) return prs();
    return prs().filter((pr) =>
      pr.title.toLowerCase().includes(q) ||
      pr.author.toLowerCase().includes(q) ||
      pr.branch.toLowerCase().includes(q) ||
      String(pr.number).includes(q) ||
      pr.labels.some((l) => l.toLowerCase().includes(q))
    );
  };

  const visiblePrs = () => filteredPrs().slice(0, visibleCount());
  const hasMore = () => filteredPrs().length > visibleCount();

  // Reload when repoPath changes (thread/project switch)
  createEffect(() => {
    const _ = props.repoPath;
    loadPrs();
  });

  async function loadPrs() {
    setLoading(true);
    try {
      const list = await ipc.listPrs(props.repoPath, filter());
      setPrs(list);
    } catch (e) {
      console.error("Failed to load PRs:", e);
      setPrs([]);
    } finally {
      setLoading(false);
    }
  }

  const [linkingPr, setLinkingPr] = createSignal<number | null>(null);

  async function linkPrToThread(pr: PullRequest) {
    const threadId = store.activeTab;
    if (!threadId || linkingPr() === pr.number) return;
    setLinkingPr(pr.number);

    try {
      // Rename the current thread to reflect the PR
      await ipc.renameThread(threadId, `PR #${pr.number}: ${pr.title}`);
      setStore("projects", (projects) =>
        projects.map((p) => ({
          ...p,
          threads: p.threads.map((t) =>
            t.id === threadId ? { ...t, title: `PR #${pr.number}: ${pr.title}` } : t
          ),
        }))
      );

      // Store the PR link
      ipc.setSetting(`pr:${threadId}`, String(pr.number)).catch(() => {});

      // Update the PR map in the store
      setStore("projectPrMap", props.projectId, (map) => ({
        ...(map || {}),
        [threadId]: pr.number,
      }));

      // Inject PR context as first message
      const context = `I'm working on PR #${pr.number}: "${pr.title}" by ${pr.author}\n\nBranch: ${pr.branch} → ${pr.base}\n+${pr.additions} -${pr.deletions} across ${pr.changed_files} files\n${pr.labels.length > 0 ? `Labels: ${pr.labels.join(", ")}\n` : ""}\nHelp me review or continue work on this PR.`;

      const msgId = await ipc.persistUserMessage(threadId, context);
      setStore("threadMessages", threadId, (msgs) => [
        ...(msgs || []),
        { id: msgId, thread_id: threadId, role: "user" as const, content: context },
      ]);
    } catch (e) {
      console.error("Failed to link PR:", e);
    } finally {
      setLinkingPr(null);
    }
  }

  function timeAgo(dateStr: string): string {
    const now = Date.now();
    const then = new Date(dateStr).getTime();
    const diff = now - then;
    const mins = Math.floor(diff / 60000);
    if (mins < 60) return `${mins}m ago`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.floor(hours / 24);
    return `${days}d ago`;
  }

  function reviewBadge(status: string) {
    switch (status) {
      case "APPROVED": return { text: "Approved", color: "var(--green)" };
      case "CHANGES_REQUESTED": return { text: "Changes", color: "var(--red)" };
      case "REVIEW_REQUIRED": return { text: "Review needed", color: "var(--amber)" };
      default: return null;
    }
  }

  return (
    <div class="prd">
      <div class="prd-header">
        <h3 class="prd-title">Pull Requests</h3>
        <div class="prd-filters">
          <For each={["open", "closed", "all"] as const}>
            {(f) => (
              <button
                class="prd-filter"
                classList={{ "prd-filter--active": filter() === f }}
                onClick={() => { setFilter(f); loadPrs(); }}
              >{f}</button>
            )}
          </For>
        </div>
        <button class="prd-refresh" onClick={loadPrs} title="Refresh">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
            <polyline points="23 4 23 10 17 10" /><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10" />
          </svg>
        </button>
      </div>

      <Show when={prs().length > 5}>
        <input
          class="prd-search"
          placeholder="Search PRs by title, author, branch, #number…"
          value={search()}
          onInput={(e) => { setSearch(e.currentTarget.value); setVisibleCount(10); }}
        />
      </Show>

      <Show when={loading()}>
        <div class="prd-loading">Loading pull requests…</div>
      </Show>

      <Show when={!loading() && filteredPrs().length === 0}>
        <div class="prd-empty">{search() ? `No PRs matching "${search()}"` : `No ${filter()} pull requests`}</div>
      </Show>

      <div class="prd-list">
        <For each={visiblePrs()}>
          {(pr) => {
            const badge = reviewBadge(pr.review_status);
            return (
              <div class="prd-item" classList={{
                "prd-item--draft": pr.draft,
                "prd-item--approved": pr.review_status === "APPROVED",
                "prd-item--changes": pr.review_status === "CHANGES_REQUESTED",
              }}>
                <div class="prd-item-main">
                  <div class="prd-item-top">
                    <span class="prd-number">#{pr.number}</span>
                    <span class="prd-item-title">{pr.title}</span>
                    {pr.draft && <span class="prd-draft">Draft</span>}
                  </div>
                  <div class="prd-item-meta">
                    <span>{pr.author}</span>
                    <span>{pr.branch}</span>
                    <span class="prd-changes">
                      <span class="prd-add">+{pr.additions}</span>
                      <span class="prd-del">-{pr.deletions}</span>
                    </span>
                    <span>{timeAgo(pr.updated_at)}</span>
                    <Show when={badge}>
                      <span class="prd-review" style={{ color: badge!.color }}>{badge!.text}</span>
                    </Show>
                  </div>
                  <Show when={pr.labels.length > 0}>
                    <div class="prd-labels">
                      <For each={pr.labels}>{(l) => <span class="prd-label">{l}</span>}</For>
                    </div>
                  </Show>
                </div>
                <button
                  class="prd-thread-btn"
                  onClick={() => props.onLinkPr ? props.onLinkPr(pr) : linkPrToThread(pr)}
                  disabled={linkingPr() === pr.number}
                  title={props.onLinkPr ? "Select this PR" : "Link this PR to the current thread"}
                >
                  <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M10 13a5 5 0 007.54.54l3-3a5 5 0 00-7.07-7.07l-1.72 1.71" />
                    <path d="M14 11a5 5 0 00-7.54-.54l-3 3a5 5 0 007.07 7.07l1.71-1.71" />
                  </svg>
                  {linkingPr() === pr.number ? "Linking…" : props.onLinkPr ? "Select" : "Link"}
                </button>
              </div>
            );
          }}
        </For>
        <Show when={hasMore()}>
          <button class="prd-show-more" onClick={() => setVisibleCount((c) => c + 10)}>
            Show {Math.min(10, filteredPrs().length - visibleCount())} more of {filteredPrs().length} PRs
          </button>
        </Show>
        <Show when={filteredPrs().length > 0}>
          <div class="prd-count">{filteredPrs().length} PR{filteredPrs().length !== 1 ? "s" : ""}{search() ? ` matching "${search()}"` : ""}</div>
        </Show>
      </div>
    </div>
  );
}

if (!document.getElementById("prd-styles")) {
  const s = document.createElement("style");
  s.id = "prd-styles";
  s.textContent = `
    .prd {
      padding: 0 16px 16px;
      max-width: 768px;
      margin: 0 auto;
      width: 100%;
    }
    .prd-header {
      display: flex;
      align-items: center;
      gap: 10px;
      margin-bottom: 12px;
      padding-top: 12px;
    }
    .prd-title {
      font-size: 14px;
      font-weight: 700;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .prd-filters {
      display: flex;
      gap: 2px;
      background: var(--bg-muted);
      border-radius: var(--radius-sm);
      padding: 2px;
      margin-left: auto;
    }
    .prd-filter {
      font-size: 11px;
      font-weight: 500;
      padding: 3px 10px;
      border-radius: 4px;
      color: var(--text-tertiary);
      text-transform: capitalize;
      transition: all 0.1s;
    }
    .prd-filter:hover { color: var(--text-secondary); }
    .prd-filter--active {
      background: var(--bg-accent);
      color: var(--text);
    }
    .prd-refresh {
      color: var(--text-tertiary);
      padding: 4px;
      border-radius: var(--radius-sm);
      transition: color 0.1s, background 0.1s;
    }
    .prd-refresh:hover { color: var(--text-secondary); background: var(--bg-hover); }
    .prd-search {
      width: 100%;
      padding: 7px 10px;
      font-size: 12px;
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      font-family: var(--font-body);
      outline: none;
      margin-bottom: 10px;
    }
    .prd-search:focus { border-color: var(--primary); box-shadow: 0 0 0 2px var(--primary-glow); }
    .prd-search::placeholder { color: var(--text-tertiary); }
    .prd-show-more {
      width: 100%;
      padding: 8px;
      font-size: 12px;
      font-weight: 500;
      color: var(--primary);
      background: rgba(107, 124, 255, 0.06);
      border: 1px solid rgba(107, 124, 255, 0.12);
      border-radius: var(--radius-sm);
      margin-top: 6px;
      transition: all 0.12s;
    }
    .prd-show-more:hover { background: rgba(107, 124, 255, 0.1); border-color: rgba(107, 124, 255, 0.2); }
    .prd-count {
      font-size: 10px;
      color: var(--text-tertiary);
      text-align: center;
      margin-top: 8px;
    }
    .prd-loading, .prd-empty {
      font-size: 12px;
      color: var(--text-tertiary);
      padding: 20px 0;
      text-align: center;
    }
    .prd-list {
      display: flex;
      flex-direction: column;
      gap: 6px;
    }
    .prd-item {
      display: flex;
      align-items: flex-start;
      gap: 10px;
      padding: 10px 12px;
      background: var(--bg-card);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
      transition: border-color 0.12s;
    }
    .prd-item:hover { border-color: var(--border-strong); }
    .prd-item--draft { opacity: 0.7; }
    .prd-item--approved { border-left: 2px solid var(--green); }
    .prd-item--changes { border-left: 2px solid var(--red); }
    .prd-item-main { flex: 1; min-width: 0; }
    .prd-item-top {
      display: flex;
      align-items: baseline;
      gap: 6px;
      margin-bottom: 4px;
    }
    .prd-number {
      font-family: var(--font-mono);
      font-size: 12px;
      font-weight: 600;
      color: var(--primary);
      flex-shrink: 0;
    }
    .prd-item-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .prd-draft {
      font-size: 9px;
      font-weight: 600;
      color: var(--text-tertiary);
      border: 1px solid var(--border);
      border-radius: var(--radius-pill);
      padding: 1px 6px;
      flex-shrink: 0;
    }
    .prd-item-meta {
      display: flex;
      align-items: center;
      gap: 4px;
      font-size: 11px;
      color: var(--text-tertiary);
      flex-wrap: wrap;
    }
    .prd-item-meta span + span::before {
      content: "·";
      margin-right: 4px;
      opacity: 0.4;
    }
    .prd-changes { font-family: var(--font-mono); font-size: 10px; }
    .prd-add { color: var(--green); }
    .prd-del { color: var(--red); margin-left: 2px; }
    .prd-review { font-weight: 600; }
    .prd-labels {
      display: flex;
      gap: 4px;
      margin-top: 4px;
      flex-wrap: wrap;
    }
    .prd-label {
      font-size: 9px;
      font-weight: 500;
      padding: 1px 6px;
      border-radius: var(--radius-pill);
      background: var(--bg-accent);
      color: var(--text-secondary);
    }
    .prd-thread-btn {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 5px 10px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-tertiary);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      flex-shrink: 0;
      transition: all 0.12s;
      margin-top: 2px;
    }
    .prd-thread-btn:hover {
      color: var(--primary);
      border-color: rgba(107, 124, 255, 0.3);
      background: rgba(107, 124, 255, 0.06);
    }
  `;
  document.head.appendChild(s);
}
