import { createSignal, onMount, For, Show, createEffect } from "solid-js";
import * as ipc from "../../ipc";
import type { GitLogEntry, GitBranch, GitStatusEntry } from "../../ipc";

type Section = "status" | "branches" | "log";

export function GitPanel(props: { cwd: string }) {
  const [activeSection, setActiveSection] = createSignal<Section>("status");
  const [statusEntries, setStatusEntries] = createSignal<GitStatusEntry[]>([]);
  const [branches, setBranches] = createSignal<GitBranch[]>([]);
  const [logEntries, setLogEntries] = createSignal<GitLogEntry[]>([]);
  const [checkedFiles, setCheckedFiles] = createSignal<Set<string>>(new Set());
  const [commitMsg, setCommitMsg] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [feedback, setFeedback] = createSignal<{ type: "success" | "error"; text: string } | null>(null);
  const [newBranchName, setNewBranchName] = createSignal("");
  const [showNewBranch, setShowNewBranch] = createSignal(false);
  const [pushing, setPushing] = createSignal(false);
  const [committing, setCommitting] = createSignal(false);

  function showFeedback(type: "success" | "error", text: string) {
    setFeedback({ type, text });
    setTimeout(() => setFeedback(null), 4000);
  }

  async function loadStatus() {
    try {
      const entries = await ipc.gitStatus(props.cwd);
      setStatusEntries(entries);
    } catch (e) {
      console.error("git status failed:", e);
    }
  }

  async function loadBranches() {
    try {
      const b = await ipc.gitBranches(props.cwd);
      setBranches(b);
    } catch (e) {
      console.error("git branches failed:", e);
    }
  }

  async function loadLog() {
    try {
      const entries = await ipc.gitLog(props.cwd, 10);
      setLogEntries(entries);
    } catch (e) {
      console.error("git log failed:", e);
    }
  }

  async function loadAll() {
    setLoading(true);
    await Promise.all([loadStatus(), loadBranches(), loadLog()]);
    setLoading(false);
  }

  createEffect(() => {
    const _ = props.cwd;
    loadAll();
  });

  function toggleFile(path: string) {
    setCheckedFiles((prev) => {
      const next = new Set(prev);
      if (next.has(path)) next.delete(path);
      else next.add(path);
      return next;
    });
  }

  function toggleAll() {
    const all = statusEntries().map((e) => e.path);
    const checked = checkedFiles();
    if (all.every((p) => checked.has(p))) {
      setCheckedFiles(new Set());
    } else {
      setCheckedFiles(new Set(all));
    }
  }

  async function handleCommit() {
    const files = Array.from(checkedFiles());
    const msg = commitMsg().trim();
    if (!msg) {
      showFeedback("error", "Commit message is required");
      return;
    }
    if (files.length === 0) {
      showFeedback("error", "No files selected");
      return;
    }
    setCommitting(true);
    try {
      const result = await ipc.gitCommit(props.cwd, msg, files);
      showFeedback("success", result);
      setCommitMsg("");
      setCheckedFiles(new Set());
      await loadAll();
    } catch (e) {
      showFeedback("error", String(e));
    } finally {
      setCommitting(false);
    }
  }

  async function handleCheckout(branch: string) {
    try {
      const result = await ipc.gitCheckout(props.cwd, branch);
      showFeedback("success", result);
      await loadAll();
    } catch (e) {
      showFeedback("error", String(e));
    }
  }

  async function handleNewBranch() {
    const name = newBranchName().trim();
    if (!name) return;
    try {
      const result = await ipc.gitCreateBranch(props.cwd, name);
      showFeedback("success", result);
      setNewBranchName("");
      setShowNewBranch(false);
      await loadAll();
    } catch (e) {
      showFeedback("error", String(e));
    }
  }

  async function handlePush() {
    setPushing(true);
    try {
      const result = await ipc.gitPush(props.cwd);
      showFeedback("success", result);
    } catch (e) {
      showFeedback("error", String(e));
    } finally {
      setPushing(false);
    }
  }

  const currentBranch = () => branches().find((b) => b.current)?.name ?? "unknown";
  const localBranches = () => branches().filter((b) => !b.remote);

  function statusIcon(status: string): { label: string; cls: string } {
    switch (status) {
      case "modified": return { label: "M", cls: "gp-st-modified" };
      case "added": return { label: "A", cls: "gp-st-added" };
      case "deleted": return { label: "D", cls: "gp-st-deleted" };
      case "untracked": return { label: "?", cls: "gp-st-untracked" };
      case "renamed": return { label: "R", cls: "gp-st-renamed" };
      default: return { label: "?", cls: "gp-st-modified" };
    }
  }

  onMount(() => {
    if (document.getElementById("git-panel-styles")) return;
    const style = document.createElement("style");
    style.id = "git-panel-styles";
    style.textContent = GIT_PANEL_STYLES;
    document.head.appendChild(style);
  });

  return (
    <div class="gp-root">
      {/* Section tabs */}
      <div class="gp-tabs">
        <button
          class="gp-tab"
          classList={{ "gp-tab-active": activeSection() === "status" }}
          onClick={() => setActiveSection("status")}
        >
          Status
          <Show when={statusEntries().length > 0}>
            <span class="gp-tab-badge">{statusEntries().length}</span>
          </Show>
        </button>
        <button
          class="gp-tab"
          classList={{ "gp-tab-active": activeSection() === "branches" }}
          onClick={() => setActiveSection("branches")}
        >
          Branches
        </button>
        <button
          class="gp-tab"
          classList={{ "gp-tab-active": activeSection() === "log" }}
          onClick={() => setActiveSection("log")}
        >
          Log
        </button>

        <div class="gp-tabs-right">
          <span class="gp-branch-label">{currentBranch()}</span>
          <button class="gp-icon-btn" onClick={loadAll} title="Refresh">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M23 4v6h-6M1 20v-6h6" />
              <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" />
            </svg>
          </button>
        </div>
      </div>

      {/* Feedback toast */}
      <Show when={feedback()}>
        {(fb) => (
          <div class={`gp-feedback gp-feedback-${fb().type}`}>
            {fb().text}
          </div>
        )}
      </Show>

      {/* Status section */}
      <Show when={activeSection() === "status"}>
        <div class="gp-section">
          <Show when={statusEntries().length === 0 && !loading()}>
            <div class="gp-empty">Working tree clean</div>
          </Show>
          <Show when={statusEntries().length > 0}>
            <div class="gp-status-header">
              <label class="gp-check-all">
                <input
                  type="checkbox"
                  checked={statusEntries().length > 0 && statusEntries().every((e) => checkedFiles().has(e.path))}
                  onChange={toggleAll}
                />
                <span>Select all</span>
              </label>
            </div>
            <div class="gp-file-list">
              <For each={statusEntries()}>
                {(entry) => {
                  const st = statusIcon(entry.status);
                  return (
                    <label class="gp-file-row">
                      <input
                        type="checkbox"
                        checked={checkedFiles().has(entry.path)}
                        onChange={() => toggleFile(entry.path)}
                      />
                      <span class={`gp-st-badge ${st.cls}`}>{st.label}</span>
                      <span class="gp-file-path" title={entry.path}>
                        {entry.path}
                      </span>
                    </label>
                  );
                }}
              </For>
            </div>

            {/* Commit area */}
            <div class="gp-commit-area">
              <input
                class="gp-commit-input"
                type="text"
                placeholder="Commit message..."
                value={commitMsg()}
                onInput={(e) => setCommitMsg(e.currentTarget.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter" && !e.shiftKey) {
                    e.preventDefault();
                    handleCommit();
                  }
                }}
              />
              <div class="gp-commit-actions">
                <button
                  class="gp-btn gp-btn-primary"
                  disabled={committing() || checkedFiles().size === 0 || !commitMsg().trim()}
                  onClick={handleCommit}
                >
                  {committing() ? "Committing..." : `Commit (${checkedFiles().size})`}
                </button>
                <button
                  class="gp-btn gp-btn-secondary"
                  disabled={pushing()}
                  onClick={handlePush}
                >
                  {pushing() ? "Pushing..." : "Push"}
                </button>
              </div>
            </div>
          </Show>
        </div>
      </Show>

      {/* Branches section */}
      <Show when={activeSection() === "branches"}>
        <div class="gp-section">
          <div class="gp-branch-actions">
            <Show
              when={showNewBranch()}
              fallback={
                <button class="gp-btn gp-btn-secondary gp-btn-sm" onClick={() => setShowNewBranch(true)}>
                  + New branch
                </button>
              }
            >
              <div class="gp-new-branch">
                <input
                  class="gp-commit-input"
                  type="text"
                  placeholder="Branch name..."
                  value={newBranchName()}
                  onInput={(e) => setNewBranchName(e.currentTarget.value)}
                  onKeyDown={(e) => {
                    if (e.key === "Enter") handleNewBranch();
                    if (e.key === "Escape") setShowNewBranch(false);
                  }}
                  autofocus
                />
                <button class="gp-btn gp-btn-primary gp-btn-sm" onClick={handleNewBranch}>
                  Create
                </button>
                <button class="gp-btn gp-btn-secondary gp-btn-sm" onClick={() => setShowNewBranch(false)}>
                  Cancel
                </button>
              </div>
            </Show>
          </div>
          <div class="gp-branch-list">
            <For each={localBranches()}>
              {(branch) => (
                <button
                  class="gp-branch-row"
                  classList={{ "gp-branch-current": branch.current }}
                  onClick={() => { if (!branch.current) handleCheckout(branch.name); }}
                  disabled={branch.current}
                >
                  <Show when={branch.current}>
                    <svg class="gp-check-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="3">
                      <polyline points="20 6 9 17 4 12" />
                    </svg>
                  </Show>
                  <span class="gp-branch-name">{branch.name}</span>
                </button>
              )}
            </For>
          </div>
        </div>
      </Show>

      {/* Log section */}
      <Show when={activeSection() === "log"}>
        <div class="gp-section">
          <Show when={logEntries().length === 0 && !loading()}>
            <div class="gp-empty">No commits yet</div>
          </Show>
          <div class="gp-log-list">
            <For each={logEntries()}>
              {(entry) => (
                <div class="gp-log-row">
                  <span class="gp-log-hash">{entry.hash}</span>
                  <span class="gp-log-msg">{entry.message}</span>
                  <span class="gp-log-date">{entry.date}</span>
                </div>
              )}
            </For>
          </div>
        </div>
      </Show>
    </div>
  );
}

const GIT_PANEL_STYLES = `
  @keyframes gp-slide-in {
    from { opacity: 0; max-height: 0; }
    to { opacity: 1; max-height: 320px; }
  }
  .gp-root {
    border-top: 1px solid var(--border);
    background: var(--bg-card);
    display: flex;
    flex-direction: column;
    max-height: 320px;
    overflow: hidden;
    animation: gp-slide-in 0.25s ease both;
  }

  /* Tabs */
  .gp-tabs {
    display: flex;
    align-items: center;
    gap: 0;
    border-bottom: 1px solid var(--border);
    padding: 0 8px;
    flex-shrink: 0;
  }
  .gp-tab {
    padding: 7px 12px;
    font-size: 11px;
    font-weight: 500;
    color: var(--text-tertiary);
    background: none;
    border: none;
    border-bottom: 2px solid transparent;
    cursor: pointer;
    transition: color 0.12s, border-color 0.12s;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .gp-tab:hover { color: var(--text-secondary); }
  .gp-tab-active {
    color: var(--text);
    border-bottom-color: var(--primary);
  }
  .gp-tab-badge {
    background: var(--primary);
    color: #fff;
    font-size: 9px;
    font-weight: 700;
    padding: 1px 5px;
    border-radius: 8px;
    min-width: 16px;
    text-align: center;
  }
  .gp-tabs-right {
    margin-left: auto;
    display: flex;
    align-items: center;
    gap: 6px;
  }
  .gp-branch-label {
    font-size: 11px;
    font-family: var(--font-mono);
    color: var(--text-tertiary);
    max-width: 140px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gp-icon-btn {
    color: var(--text-tertiary);
    padding: 4px;
    border-radius: var(--radius-sm);
    display: flex;
    align-items: center;
    background: none;
    border: none;
    cursor: pointer;
    transition: background 0.12s, color 0.12s;
  }
  .gp-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

  /* Feedback */
  .gp-feedback {
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 500;
    flex-shrink: 0;
  }
  .gp-feedback-success {
    background: rgba(76, 214, 148, 0.1);
    color: var(--green);
  }
  .gp-feedback-error {
    background: rgba(242, 95, 103, 0.1);
    color: var(--red);
  }

  /* Sections */
  .gp-section {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .gp-empty {
    text-align: center;
    padding: 24px 12px;
    color: var(--text-tertiary);
    font-size: 12px;
  }

  /* Status file list */
  .gp-status-header {
    padding: 6px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .gp-check-all {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--text-tertiary);
    cursor: pointer;
  }
  .gp-check-all input { margin: 0; }

  .gp-file-list {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .gp-file-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 4px 12px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.1s;
  }
  .gp-file-row:hover { background: var(--bg-accent); }
  .gp-file-row input { margin: 0; flex-shrink: 0; }
  .gp-st-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    width: 16px;
    height: 16px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 3px;
    flex-shrink: 0;
  }
  .gp-st-modified { background: rgba(107, 124, 255, 0.15); color: var(--primary); }
  .gp-st-added { background: rgba(76, 214, 148, 0.15); color: var(--green); }
  .gp-st-deleted { background: rgba(242, 95, 103, 0.15); color: var(--red); }
  .gp-st-untracked { background: rgba(255, 180, 80, 0.15); color: #ffb450; }
  .gp-st-renamed { background: rgba(255, 180, 80, 0.15); color: #ffb450; }
  .gp-file-path {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
    font-family: var(--font-mono);
    font-size: 11px;
  }

  /* Commit area */
  .gp-commit-area {
    border-top: 1px solid var(--border);
    padding: 8px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    flex-shrink: 0;
  }
  .gp-commit-input {
    width: 100%;
    padding: 6px 10px;
    font-size: 12px;
    background: var(--bg-surface);
    border: 1px solid var(--border);
    border-radius: var(--radius-sm);
    color: var(--text);
    outline: none;
    transition: border-color 0.12s;
  }
  .gp-commit-input:focus {
    border-color: var(--primary);
  }
  .gp-commit-input::placeholder {
    color: var(--text-tertiary);
  }
  .gp-commit-actions {
    display: flex;
    gap: 6px;
  }

  /* Buttons */
  .gp-btn {
    padding: 5px 12px;
    font-size: 11px;
    font-weight: 500;
    border-radius: var(--radius-sm);
    border: none;
    cursor: pointer;
    transition: background 0.12s, opacity 0.12s;
    white-space: nowrap;
  }
  .gp-btn:disabled {
    opacity: 0.45;
    cursor: default;
  }
  .gp-btn-primary {
    background: var(--primary);
    color: #fff;
  }
  .gp-btn-primary:hover:not(:disabled) {
    filter: brightness(1.1);
  }
  .gp-btn-secondary {
    background: var(--bg-accent);
    color: var(--text-secondary);
  }
  .gp-btn-secondary:hover:not(:disabled) {
    background: var(--bg-surface);
  }
  .gp-btn-sm {
    padding: 4px 10px;
    font-size: 11px;
  }

  /* Branches */
  .gp-branch-actions {
    padding: 8px 12px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .gp-new-branch {
    display: flex;
    gap: 6px;
    align-items: center;
  }
  .gp-branch-list {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .gp-branch-row {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 12px;
    font-size: 12px;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: var(--text);
    transition: background 0.1s;
  }
  .gp-branch-row:hover:not(:disabled) { background: var(--bg-accent); }
  .gp-branch-current {
    color: var(--primary);
    cursor: default;
  }
  .gp-branch-name {
    font-family: var(--font-mono);
    font-size: 12px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .gp-check-icon {
    color: var(--primary);
    flex-shrink: 0;
  }

  /* Log */
  .gp-log-list {
    overflow-y: auto;
    flex: 1;
    min-height: 0;
  }
  .gp-log-row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 5px 12px;
    font-size: 12px;
    border-bottom: 1px solid rgba(255,255,255,0.03);
  }
  .gp-log-hash {
    font-family: var(--font-mono);
    font-size: 11px;
    color: var(--primary);
    flex-shrink: 0;
    min-width: 56px;
  }
  .gp-log-msg {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--text);
  }
  .gp-log-date {
    font-size: 10px;
    color: var(--text-tertiary);
    flex-shrink: 0;
    white-space: nowrap;
  }
`;
