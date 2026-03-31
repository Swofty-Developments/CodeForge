import { createSignal, createEffect, For, Show, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { PrDetails, PrComment, PrCheck, PrFile } from "../../ipc";

interface Props {
  repoPath: string;
  prNumber: number;
}

type Tab = "overview" | "files" | "checks" | "comments";

export function PrReviewPanel(props: Props) {
  const { setStore } = appStore;
  const [tab, setTab] = createSignal<Tab>("overview");
  const [details, setDetails] = createSignal<PrDetails | null>(null);
  const [comments, setComments] = createSignal<PrComment[]>([]);
  const [checks, setChecks] = createSignal<PrCheck[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [diff, setDiff] = createSignal<string>("");
  const [selectedFile, setSelectedFile] = createSignal<string | null>(null);

  // Review submission
  const [reviewBody, setReviewBody] = createSignal("");
  const [submitting, setSubmitting] = createSignal(false);
  const [submitMsg, setSubmitMsg] = createSignal<string | null>(null);

  // Add comment
  const [commentBody, setCommentBody] = createSignal("");
  const [addingComment, setAddingComment] = createSignal(false);

  function close() {
    setStore("prReviewOpen", false);
  }

  async function loadDetails() {
    setLoading(true);
    setError(null);
    try {
      const [d, c, ch] = await Promise.all([
        ipc.getPrDetails(props.repoPath, props.prNumber),
        ipc.getPrComments(props.repoPath, props.prNumber),
        ipc.getPrChecks(props.repoPath, props.prNumber).catch(() => [] as PrCheck[]),
      ]);
      setDetails(d);
      setComments(c);
      setChecks(ch);
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  createEffect(() => {
    const _ = props.prNumber;
    loadDetails();
  });

  async function loadDiff() {
    try {
      const d = await ipc.getPrDiff(props.repoPath, props.prNumber);
      setDiff(d);
    } catch (e) {
      console.error("Failed to load diff:", e);
    }
  }

  // Load diff when switching to files tab
  createEffect(() => {
    if (tab() === "files" && !diff()) {
      loadDiff();
    }
  });

  async function submitReview(event: string) {
    setSubmitting(true);
    setSubmitMsg(null);
    try {
      await ipc.submitPrReview(props.repoPath, props.prNumber, event, reviewBody());
      setSubmitMsg(event === "APPROVE" ? "Approved" : event === "REQUEST_CHANGES" ? "Changes requested" : "Comment submitted");
      setReviewBody("");
      await loadDetails();
    } catch (e) {
      setSubmitMsg(`Error: ${e}`);
    } finally {
      setSubmitting(false);
    }
  }

  async function addComment() {
    if (!commentBody().trim()) return;
    setAddingComment(true);
    try {
      await ipc.addPrComment(props.repoPath, props.prNumber, commentBody());
      setCommentBody("");
      // Reload comments
      const c = await ipc.getPrComments(props.repoPath, props.prNumber);
      setComments(c);
    } catch (e) {
      console.error("Failed to add comment:", e);
    } finally {
      setAddingComment(false);
    }
  }

  function reviewBadge(state: string) {
    switch (state) {
      case "APPROVED": return { text: "Approved", cls: "prr-badge-approved" };
      case "CHANGES_REQUESTED": return { text: "Changes Requested", cls: "prr-badge-changes" };
      case "COMMENTED": return { text: "Commented", cls: "prr-badge-commented" };
      case "DISMISSED": return { text: "Dismissed", cls: "prr-badge-dismissed" };
      default: return { text: state || "Pending", cls: "prr-badge-pending" };
    }
  }

  function checkIcon(conclusion: string) {
    switch (conclusion.toUpperCase()) {
      case "SUCCESS": return { icon: "check", cls: "prr-check-success" };
      case "FAILURE": case "ERROR": case "TIMED_OUT": return { icon: "x", cls: "prr-check-failure" };
      case "CANCELLED": case "SKIPPED": return { icon: "skip", cls: "prr-check-skipped" };
      default: return { icon: "pending", cls: "prr-check-pending" };
    }
  }

  function timeAgo(dateStr: string): string {
    if (!dateStr) return "";
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

  function parseDiffForFile(filePath: string): string[] {
    const lines = diff().split("\n");
    const result: string[] = [];
    let inFile = false;
    for (const line of lines) {
      if (line.startsWith("diff --git")) {
        inFile = line.includes(`b/${filePath}`);
        continue;
      }
      if (inFile) {
        if (line.startsWith("diff --git")) break;
        result.push(line);
      }
    }
    return result;
  }

  function diffLineClass(line: string): string {
    if (line.startsWith("+") && !line.startsWith("+++")) return "prr-diff-add";
    if (line.startsWith("-") && !line.startsWith("---")) return "prr-diff-remove";
    if (line.startsWith("@@")) return "prr-diff-hunk";
    return "prr-diff-context";
  }

  // Inject styles
  onMount(() => {
    if (document.getElementById("prr-styles")) return;
    const style = document.createElement("style");
    style.id = "prr-styles";
    style.textContent = PRR_STYLES;
    document.head.appendChild(style);
  });

  return (
    <div class="prr-pane">
      {/* Header */}
      <div class="prr-header">
        <div class="prr-header-left">
          <svg class="prr-header-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <circle cx="18" cy="18" r="3"/><circle cx="6" cy="6" r="3"/>
            <path d="M6 21V9a9 9 0 009 9"/>
          </svg>
          <h3>PR #{props.prNumber}</h3>
          <Show when={details()}>
            <span class={`prr-state prr-state-${details()!.state.toLowerCase()}`}>
              {details()!.state}
            </span>
          </Show>
        </div>
        <div class="prr-header-actions">
          <button class="prr-icon-btn" onClick={loadDetails} title="Refresh">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6M1 20v-6h6"/>
              <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15"/>
            </svg>
          </button>
          <button class="prr-icon-btn" onClick={close} title="Close">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
            </svg>
          </button>
        </div>
      </div>

      {/* Tabs */}
      <div class="prr-tabs">
        <For each={["overview", "files", "checks", "comments"] as Tab[]}>
          {(t) => (
            <button
              class="prr-tab"
              classList={{ "prr-tab-active": tab() === t }}
              onClick={() => setTab(t)}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
              <Show when={t === "checks" && checks().length > 0}>
                <span class="prr-tab-count">{checks().length}</span>
              </Show>
              <Show when={t === "comments" && comments().length > 0}>
                <span class="prr-tab-count">{comments().length}</span>
              </Show>
              <Show when={t === "files" && details()}>
                <span class="prr-tab-count">{details()!.changed_files}</span>
              </Show>
            </button>
          )}
        </For>
      </div>

      {/* Body */}
      <div class="prr-body">
        <Show when={loading()}>
          <div class="prr-empty">Loading PR details...</div>
        </Show>
        <Show when={error()}>
          <div class="prr-empty prr-error">{error()}</div>
        </Show>

        <Show when={!loading() && !error() && details()}>
          {/* ── Overview tab ── */}
          <Show when={tab() === "overview"}>
            <div class="prr-overview">
              <h2 class="prr-pr-title">{details()!.title}</h2>
              <div class="prr-meta-row">
                <span class="prr-author">{details()!.author}</span>
                <span class="prr-branch-info">{details()!.branch} → {details()!.base}</span>
              </div>
              <div class="prr-stats-row">
                <span class="prr-stat-add">+{details()!.additions}</span>
                <span class="prr-stat-del">-{details()!.deletions}</span>
                <span class="prr-stat-files">{details()!.changed_files} file{details()!.changed_files !== 1 ? "s" : ""}</span>
              </div>

              <Show when={details()!.labels.length > 0}>
                <div class="prr-labels">
                  <For each={details()!.labels}>{(l) => <span class="prr-label">{l}</span>}</For>
                </div>
              </Show>

              {/* Review Decision */}
              <Show when={details()!.review_decision}>
                <div class="prr-review-decision">
                  {(() => {
                    const b = reviewBadge(details()!.review_decision);
                    return <span class={`prr-badge ${b.cls}`}>{b.text}</span>;
                  })()}
                </div>
              </Show>

              {/* Reviews list */}
              <Show when={details()!.reviews.length > 0}>
                <div class="prr-reviews-section">
                  <h4 class="prr-section-title">Reviews</h4>
                  <For each={details()!.reviews}>
                    {(r) => {
                      const b = reviewBadge(r.state);
                      return (
                        <div class="prr-review-item">
                          <span class="prr-review-author">{r.author}</span>
                          <span class={`prr-badge prr-badge-sm ${b.cls}`}>{b.text}</span>
                          <Show when={r.body}>
                            <p class="prr-review-body">{r.body}</p>
                          </Show>
                        </div>
                      );
                    }}
                  </For>
                </div>
              </Show>

              {/* Description */}
              <Show when={details()!.body}>
                <div class="prr-description">
                  <h4 class="prr-section-title">Description</h4>
                  <pre class="prr-body-text">{details()!.body}</pre>
                </div>
              </Show>

              {/* Review Actions */}
              <div class="prr-review-actions">
                <h4 class="prr-section-title">Submit Review</h4>
                <textarea
                  class="prr-review-textarea"
                  placeholder="Leave a review comment..."
                  value={reviewBody()}
                  onInput={(e) => setReviewBody(e.currentTarget.value)}
                  rows={3}
                />
                <div class="prr-review-buttons">
                  <button
                    class="prr-btn prr-btn-approve"
                    onClick={() => submitReview("APPROVE")}
                    disabled={submitting()}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                      <polyline points="20 6 9 17 4 12"/>
                    </svg>
                    Approve
                  </button>
                  <button
                    class="prr-btn prr-btn-comment"
                    onClick={() => submitReview("COMMENT")}
                    disabled={submitting()}
                  >
                    Comment
                  </button>
                  <button
                    class="prr-btn prr-btn-request-changes"
                    onClick={() => submitReview("REQUEST_CHANGES")}
                    disabled={submitting()}
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M10.29 3.86L1.82 18a2 2 0 001.71 3h16.94a2 2 0 001.71-3L13.71 3.86a2 2 0 00-3.42 0z"/>
                      <line x1="12" y1="9" x2="12" y2="13"/><line x1="12" y1="17" x2="12.01" y2="17"/>
                    </svg>
                    Request Changes
                  </button>
                </div>
                <Show when={submitMsg()}>
                  <div class={`prr-submit-msg ${submitMsg()!.startsWith("Error") ? "prr-submit-error" : "prr-submit-ok"}`}>
                    {submitMsg()}
                  </div>
                </Show>
              </div>
            </div>
          </Show>

          {/* ── Files tab ── */}
          <Show when={tab() === "files"}>
            <div class="prr-files">
              <div class="prr-file-list">
                <For each={details()!.files}>
                  {(f: PrFile) => (
                    <button
                      class="prr-file-item"
                      classList={{ "prr-file-selected": selectedFile() === f.path }}
                      onClick={() => setSelectedFile(f.path)}
                    >
                      <span class="prr-file-name" title={f.path}>
                        {f.path.split("/").pop()}
                      </span>
                      <span class="prr-file-stats">
                        <Show when={f.additions > 0}>
                          <span class="prr-stat-add">+{f.additions}</span>
                        </Show>
                        <Show when={f.deletions > 0}>
                          <span class="prr-stat-del">-{f.deletions}</span>
                        </Show>
                      </span>
                    </button>
                  )}
                </For>
              </div>
              <div class="prr-diff-view">
                <Show when={!selectedFile()}>
                  <div class="prr-empty">Select a file to view its diff</div>
                </Show>
                <Show when={selectedFile()}>
                  <div class="prr-diff-content">
                    <div class="prr-diff-file-header">{selectedFile()}</div>
                    <div class="prr-diff-lines">
                      <For each={parseDiffForFile(selectedFile()!)}>
                        {(line) => (
                          <div class={`prr-diff-line ${diffLineClass(line)}`}>
                            <span class="prr-diff-line-content">{line || "\n"}</span>
                          </div>
                        )}
                      </For>
                      <Show when={parseDiffForFile(selectedFile()!).length === 0}>
                        <div class="prr-empty">Loading diff...</div>
                      </Show>
                    </div>
                  </div>
                </Show>
              </div>
            </div>
          </Show>

          {/* ── Checks tab ── */}
          <Show when={tab() === "checks"}>
            <div class="prr-checks">
              <Show when={checks().length === 0}>
                <div class="prr-empty">No status checks</div>
              </Show>
              <For each={checks()}>
                {(check) => {
                  const ci = checkIcon(check.conclusion || check.status);
                  return (
                    <div class={`prr-check-item ${ci.cls}`}>
                      <span class="prr-check-icon">
                        {ci.icon === "check" && (
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="20 6 9 17 4 12"/></svg>
                        )}
                        {ci.icon === "x" && (
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
                        )}
                        {ci.icon === "pending" && (
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
                        )}
                        {ci.icon === "skip" && (
                          <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><circle cx="12" cy="12" r="10"/><line x1="4.93" y1="4.93" x2="19.07" y2="19.07"/></svg>
                        )}
                      </span>
                      <span class="prr-check-name">{check.name}</span>
                      <span class="prr-check-status">
                        {check.conclusion || check.status || "Pending"}
                      </span>
                    </div>
                  );
                }}
              </For>
            </div>
          </Show>

          {/* ── Comments tab ── */}
          <Show when={tab() === "comments"}>
            <div class="prr-comments">
              <Show when={comments().length === 0}>
                <div class="prr-empty">No review comments</div>
              </Show>
              <For each={comments()}>
                {(c) => (
                  <div class="prr-comment">
                    <div class="prr-comment-header">
                      <span class="prr-comment-author">{c.author}</span>
                      <Show when={c.path}>
                        <span class="prr-comment-file">{c.path}{c.line ? `:${c.line}` : ""}</span>
                      </Show>
                      <span class="prr-comment-time">{timeAgo(c.created_at)}</span>
                    </div>
                    <Show when={c.diff_hunk}>
                      <div class="prr-comment-hunk">
                        <For each={c.diff_hunk.split("\n").slice(-4)}>
                          {(line) => (
                            <div class={`prr-diff-line ${diffLineClass(line)}`}>
                              <span class="prr-diff-line-content">{line}</span>
                            </div>
                          )}
                        </For>
                      </div>
                    </Show>
                    <div class="prr-comment-body">{c.body}</div>
                  </div>
                )}
              </For>

              {/* Add comment */}
              <div class="prr-add-comment">
                <textarea
                  class="prr-comment-textarea"
                  placeholder="Add a comment..."
                  value={commentBody()}
                  onInput={(e) => setCommentBody(e.currentTarget.value)}
                  rows={2}
                />
                <button
                  class="prr-btn prr-btn-comment"
                  onClick={addComment}
                  disabled={addingComment() || !commentBody().trim()}
                >
                  {addingComment() ? "Posting..." : "Add Comment"}
                </button>
              </div>
            </div>
          </Show>
        </Show>
      </div>
    </div>
  );
}

const PRR_STYLES = `
  .prr-pane {
    background: var(--bg-card);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  /* ── Header ── */
  .prr-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 12px 16px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .prr-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .prr-header-icon {
    color: var(--primary);
    flex-shrink: 0;
  }
  .prr-header h3 {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.2px;
    margin: 0;
  }
  .prr-state {
    font-size: 10px;
    font-weight: 600;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    text-transform: uppercase;
    letter-spacing: 0.3px;
  }
  .prr-state-open { background: rgba(76, 214, 148, 0.12); color: var(--green); }
  .prr-state-closed { background: rgba(242, 95, 103, 0.12); color: var(--red); }
  .prr-state-merged { background: rgba(163, 113, 247, 0.12); color: #a371f7; }
  .prr-header-actions { display: flex; align-items: center; gap: 4px; }
  .prr-icon-btn {
    color: var(--text-tertiary);
    padding: 5px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    background: none;
    border: none;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .prr-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

  /* ── Tabs ── */
  .prr-tabs {
    display: flex;
    gap: 0;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
    padding: 0 12px;
  }
  .prr-tab {
    display: flex;
    align-items: center;
    gap: 5px;
    padding: 8px 14px;
    font-size: 12px;
    font-weight: 500;
    color: var(--text-tertiary);
    border: none;
    background: none;
    cursor: pointer;
    border-bottom: 2px solid transparent;
    transition: color 0.12s, border-color 0.12s;
    margin-bottom: -1px;
  }
  .prr-tab:hover { color: var(--text-secondary); }
  .prr-tab-active {
    color: var(--text);
    border-bottom-color: var(--primary);
  }
  .prr-tab-count {
    font-size: 10px;
    font-weight: 600;
    background: var(--bg-muted);
    color: var(--text-tertiary);
    padding: 1px 6px;
    border-radius: var(--radius-pill);
  }

  /* ── Body ── */
  .prr-body {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }
  .prr-empty {
    text-align: center;
    padding: 32px 12px;
    color: var(--text-tertiary);
    font-size: 12px;
  }
  .prr-error { color: var(--red); }

  /* ── Overview ── */
  .prr-overview {
    padding: 16px;
  }
  .prr-pr-title {
    font-size: 16px;
    font-weight: 700;
    letter-spacing: -0.3px;
    margin: 0 0 8px;
    line-height: 1.3;
  }
  .prr-meta-row {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--text-tertiary);
    margin-bottom: 6px;
  }
  .prr-meta-row span + span::before {
    content: "\\00B7";
    margin-right: 4px;
    opacity: 0.4;
  }
  .prr-author { font-weight: 500; color: var(--text-secondary); }
  .prr-branch-info { font-family: var(--font-mono); font-size: 11px; }
  .prr-stats-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-family: var(--font-mono);
    font-size: 12px;
    margin-bottom: 12px;
  }
  .prr-stat-add { color: var(--green); font-weight: 500; }
  .prr-stat-del { color: var(--red); font-weight: 500; }
  .prr-stat-files { color: var(--text-tertiary); font-size: 11px; }
  .prr-labels {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    margin-bottom: 12px;
  }
  .prr-label {
    font-size: 10px;
    font-weight: 500;
    padding: 2px 8px;
    border-radius: var(--radius-pill);
    background: var(--bg-accent);
    color: var(--text-secondary);
  }
  .prr-review-decision { margin-bottom: 12px; }
  .prr-badge {
    display: inline-flex;
    align-items: center;
    font-size: 11px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: var(--radius-pill);
  }
  .prr-badge-sm { font-size: 10px; padding: 2px 8px; }
  .prr-badge-approved { background: rgba(76, 214, 148, 0.12); color: var(--green); }
  .prr-badge-changes { background: rgba(242, 95, 103, 0.12); color: var(--red); }
  .prr-badge-commented { background: rgba(107, 124, 255, 0.12); color: var(--primary); }
  .prr-badge-dismissed { background: var(--bg-muted); color: var(--text-tertiary); }
  .prr-badge-pending { background: rgba(255, 180, 80, 0.12); color: #ffb450; }
  .prr-section-title {
    font-size: 12px;
    font-weight: 600;
    color: var(--text-secondary);
    margin: 0 0 8px;
    text-transform: uppercase;
    letter-spacing: 0.5px;
  }

  /* Reviews list */
  .prr-reviews-section { margin-bottom: 16px; }
  .prr-review-item {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: 6px;
    padding: 6px 0;
    border-bottom: 1px solid var(--border);
  }
  .prr-review-item:last-child { border-bottom: none; }
  .prr-review-author {
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
  }
  .prr-review-body {
    width: 100%;
    font-size: 12px;
    color: var(--text-secondary);
    margin: 2px 0 0;
    line-height: 1.4;
  }

  /* Description */
  .prr-description { margin-bottom: 16px; }
  .prr-body-text {
    font-size: 13px;
    color: var(--text-secondary);
    white-space: pre-wrap;
    word-break: break-word;
    font-family: var(--font-body);
    line-height: 1.5;
    margin: 0;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    padding: 12px;
    max-height: 300px;
    overflow-y: auto;
  }

  /* Review actions */
  .prr-review-actions {
    border-top: 1px solid var(--border);
    padding-top: 14px;
    margin-top: 8px;
  }
  .prr-review-textarea, .prr-comment-textarea {
    width: 100%;
    font-size: 12px;
    font-family: var(--font-body);
    padding: 8px 10px;
    background: var(--bg-base);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    resize: vertical;
    outline: none;
    margin-bottom: 8px;
    box-sizing: border-box;
  }
  .prr-review-textarea:focus, .prr-comment-textarea:focus {
    border-color: var(--primary);
  }
  .prr-review-textarea::placeholder, .prr-comment-textarea::placeholder {
    color: var(--text-tertiary);
  }
  .prr-review-buttons {
    display: flex;
    gap: 6px;
    flex-wrap: wrap;
  }
  .prr-btn {
    display: inline-flex;
    align-items: center;
    gap: 5px;
    padding: 6px 14px;
    font-size: 12px;
    font-weight: 500;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    cursor: pointer;
    transition: all 0.12s;
    background: var(--bg-surface);
    color: var(--text);
  }
  .prr-btn:disabled { opacity: 0.4; cursor: not-allowed; }
  .prr-btn-approve {
    background: rgba(76, 214, 148, 0.1);
    border-color: rgba(76, 214, 148, 0.25);
    color: var(--green);
  }
  .prr-btn-approve:hover:not(:disabled) {
    background: rgba(76, 214, 148, 0.18);
    border-color: rgba(76, 214, 148, 0.4);
  }
  .prr-btn-comment {
    color: var(--text-secondary);
  }
  .prr-btn-comment:hover:not(:disabled) {
    background: var(--bg-accent);
    border-color: var(--border-strong);
  }
  .prr-btn-request-changes {
    background: rgba(242, 95, 103, 0.08);
    border-color: rgba(242, 95, 103, 0.2);
    color: var(--red);
  }
  .prr-btn-request-changes:hover:not(:disabled) {
    background: rgba(242, 95, 103, 0.15);
    border-color: rgba(242, 95, 103, 0.35);
  }
  .prr-submit-msg {
    font-size: 11px;
    font-weight: 500;
    margin-top: 8px;
    padding: 4px 8px;
    border-radius: var(--radius-sm);
  }
  .prr-submit-ok { color: var(--green); background: rgba(76, 214, 148, 0.08); }
  .prr-submit-error { color: var(--red); background: rgba(242, 95, 103, 0.08); }

  /* ── Files tab ── */
  .prr-files {
    display: flex;
    flex: 1;
    min-height: 0;
    height: 100%;
  }
  .prr-file-list {
    width: 220px;
    min-width: 160px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    flex-shrink: 0;
    padding: 4px;
  }
  .prr-file-item {
    display: flex;
    align-items: center;
    justify-content: space-between;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: var(--radius-sm);
    text-align: left;
    transition: background 0.1s;
    color: var(--text);
    gap: 6px;
  }
  .prr-file-item:hover { background: var(--bg-accent); }
  .prr-file-selected {
    background: var(--bg-surface) !important;
    box-shadow: inset 2px 0 0 var(--primary);
  }
  .prr-file-name {
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .prr-file-stats {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    font-size: 10px;
    font-family: var(--font-mono);
  }

  /* Diff view */
  .prr-diff-view {
    flex: 1;
    overflow: auto;
    min-width: 0;
  }
  .prr-diff-content {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
  }
  .prr-diff-file-header {
    padding: 8px 16px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-secondary);
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    position: sticky;
    top: 0;
    z-index: 2;
  }
  .prr-diff-lines {
    overflow-x: auto;
  }
  .prr-diff-line {
    display: flex;
    min-height: 20px;
    white-space: pre;
    padding: 0 12px;
  }
  .prr-diff-context { background: transparent; }
  .prr-diff-add {
    background: rgba(76, 214, 148, 0.08);
  }
  .prr-diff-add .prr-diff-line-content { color: var(--green); }
  .prr-diff-remove {
    background: rgba(242, 95, 103, 0.08);
  }
  .prr-diff-remove .prr-diff-line-content { color: var(--red); }
  .prr-diff-hunk {
    background: rgba(107, 124, 255, 0.06);
  }
  .prr-diff-hunk .prr-diff-line-content {
    color: var(--primary);
    font-weight: 500;
  }
  .prr-diff-line-content {
    flex: 1;
    color: var(--text);
  }

  /* ── Checks tab ── */
  .prr-checks {
    padding: 8px;
  }
  .prr-check-item {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 10px;
    border-radius: var(--radius-sm);
    transition: background 0.08s;
  }
  .prr-check-item:hover { background: var(--bg-accent); }
  .prr-check-icon {
    width: 20px;
    height: 20px;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
  }
  .prr-check-success .prr-check-icon { color: var(--green); }
  .prr-check-failure .prr-check-icon { color: var(--red); }
  .prr-check-pending .prr-check-icon { color: #ffb450; }
  .prr-check-skipped .prr-check-icon { color: var(--text-tertiary); }
  .prr-check-name {
    flex: 1;
    font-size: 12px;
    font-weight: 500;
    color: var(--text);
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .prr-check-status {
    font-size: 11px;
    color: var(--text-tertiary);
    flex-shrink: 0;
    text-transform: capitalize;
  }

  /* ── Comments tab ── */
  .prr-comments {
    padding: 12px;
  }
  .prr-comment {
    border: 1px solid var(--border);
    border-radius: var(--radius-md);
    margin-bottom: 10px;
    overflow: hidden;
  }
  .prr-comment-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 8px 12px;
    background: var(--bg-surface);
    border-bottom: 1px solid var(--border);
    font-size: 11px;
    color: var(--text-tertiary);
  }
  .prr-comment-header span + span::before {
    content: "\\00B7";
    margin-right: 4px;
    opacity: 0.4;
  }
  .prr-comment-author {
    font-weight: 600;
    color: var(--text-secondary);
  }
  .prr-comment-file {
    font-family: var(--font-mono);
    font-size: 10px;
    color: var(--primary);
  }
  .prr-comment-time { color: var(--text-tertiary); }
  .prr-comment-hunk {
    border-bottom: 1px solid var(--border);
    font-family: var(--font-mono);
    font-size: 11px;
    line-height: 1.5;
    max-height: 120px;
    overflow: hidden;
  }
  .prr-comment-body {
    padding: 10px 12px;
    font-size: 12px;
    color: var(--text);
    line-height: 1.5;
    white-space: pre-wrap;
    word-break: break-word;
  }

  /* Add comment */
  .prr-add-comment {
    margin-top: 12px;
    padding-top: 12px;
    border-top: 1px solid var(--border);
  }
`;
