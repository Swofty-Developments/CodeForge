import { createSignal, onMount, For, Show, createEffect } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { ChangedFile, FileDiff, DiffHunk } from "../../ipc";

function statusBadge(status: string): { label: string; cls: string } {
  switch (status) {
    case "modified": return { label: "M", cls: "de-badge-modified" };
    case "added": return { label: "A", cls: "de-badge-added" };
    case "deleted": return { label: "D", cls: "de-badge-deleted" };
    case "renamed": return { label: "R", cls: "de-badge-renamed" };
    default: return { label: "?", cls: "de-badge-modified" };
  }
}

function fileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1];
}

function dirPath(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  return parts.slice(0, -1).join("/") + "/";
}

export function DiffEditor(props: { cwd: string }) {
  const { setStore } = appStore;
  const [files, setFiles] = createSignal<ChangedFile[]>([]);
  const [diffs, setDiffs] = createSignal<FileDiff[]>([]);
  const [selectedFile, setSelectedFile] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [collapsedHunks, setCollapsedHunks] = createSignal<Set<string>>(new Set());

  function close() {
    setStore("diffPanelOpen", false);
  }

  async function loadAll() {
    setLoading(true);
    setError(null);
    try {
      const [changedFiles, sessionDiffs] = await Promise.all([
        ipc.getChangedFiles(props.cwd),
        ipc.getSessionDiff(props.cwd),
      ]);
      setFiles(changedFiles);
      setDiffs(sessionDiffs);
      if (changedFiles.length > 0 && !selectedFile()) {
        setSelectedFile(changedFiles[0].path);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  onMount(loadAll);

  const totalInsertions = () => files().reduce((s, f) => s + f.insertions, 0);
  const totalDeletions = () => files().reduce((s, f) => s + f.deletions, 0);

  const selectedDiff = () => diffs().find((d) => d.path === selectedFile()) ?? null;

  function toggleHunk(id: string) {
    setCollapsedHunks((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }

  function hunkId(filePath: string, idx: number) {
    return `${filePath}:${idx}`;
  }

  // Inject styles
  onMount(() => {
    if (document.getElementById("diff-editor-styles")) return;
    const style = document.createElement("style");
    style.id = "diff-editor-styles";
    style.textContent = DIFF_STYLES;
    document.head.appendChild(style);
  });

  return (
    <div class="de-pane">
        {/* Header */}
        <div class="de-header">
          <div class="de-header-left">
            <svg class="de-header-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M12 3v18" /><path d="M3 12h18" />
            </svg>
            <h3>Changes</h3>
            <Show when={!loading() && files().length > 0}>
              <span class="de-stat-summary">
                <span class="de-stat-files">{files().length} file{files().length !== 1 ? "s" : ""}</span>
                <Show when={totalInsertions() > 0}>
                  <span class="de-stat-ins">+{totalInsertions()}</span>
                </Show>
                <Show when={totalDeletions() > 0}>
                  <span class="de-stat-del">-{totalDeletions()}</span>
                </Show>
              </span>
            </Show>
          </div>
          <div class="de-header-actions">
            <button class="de-icon-btn" onClick={loadAll} title="Refresh">
              <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <path d="M23 4v6h-6M1 20v-6h6" />
                <path d="M3.51 9a9 9 0 0114.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0020.49 15" />
              </svg>
            </button>
            <button class="de-icon-btn" onClick={close} title="Close">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </div>
        </div>

        <div class="de-body">
          {/* File sidebar */}
          <div class="de-file-list">
            <Show when={loading()}>
              <div class="de-empty">Loading...</div>
            </Show>
            <Show when={error()}>
              <div class="de-empty de-error">{error()}</div>
            </Show>
            <Show when={!loading() && files().length === 0 && !error()}>
              <div class="de-empty">No changes detected</div>
            </Show>
            <For each={files()}>
              {(file) => {
                const badge = statusBadge(file.status);
                const isSelected = () => selectedFile() === file.path;
                return (
                  <button
                    class={`de-file-item ${isSelected() ? "de-file-selected" : ""}`}
                    onClick={() => setSelectedFile(file.path)}
                    title={file.path}
                  >
                    <span class={`de-badge ${badge.cls}`}>{badge.label}</span>
                    <div class="de-file-info">
                      <span class="de-file-name">{fileName(file.path)}</span>
                      <span class="de-file-dir">{dirPath(file.path)}</span>
                    </div>
                    <div class="de-file-stats">
                      <Show when={file.insertions > 0}>
                        <span class="de-stat-ins">+{file.insertions}</span>
                      </Show>
                      <Show when={file.deletions > 0}>
                        <span class="de-stat-del">-{file.deletions}</span>
                      </Show>
                    </div>
                  </button>
                );
              }}
            </For>
          </div>

          {/* Diff view */}
          <div class="de-diff-view">
            <Show when={!selectedDiff() && !loading()}>
              <div class="de-diff-empty">
                <Show when={files().length > 0} fallback={<span>No changes to display</span>}>
                  <span>Select a file to view its diff</span>
                </Show>
              </div>
            </Show>
            <Show when={selectedDiff()}>
              {(diff) => (
                <div class="de-diff-content">
                  <div class="de-diff-file-header">{diff().path}</div>
                  <For each={diff().hunks}>
                    {(hunk, idx) => {
                      const id = () => hunkId(diff().path, idx());
                      const collapsed = () => collapsedHunks().has(id());
                      return (
                        <div class="de-hunk">
                          <button
                            class="de-hunk-header"
                            onClick={() => toggleHunk(id())}
                          >
                            <svg
                              class={`de-chevron ${collapsed() ? "" : "de-chevron-open"}`}
                              width="12" height="12" viewBox="0 0 24 24"
                              fill="none" stroke="currentColor" stroke-width="2"
                            >
                              <polyline points="9 18 15 12 9 6" />
                            </svg>
                            <span class="de-hunk-header-text">{hunk.header}</span>
                          </button>
                          <Show when={!collapsed()}>
                            <div class="de-hunk-lines">
                              <For each={hunk.lines}>
                                {(line) => (
                                  <div class={`de-line de-line-${line.line_type}`}>
                                    <span class="de-gutter de-gutter-old">
                                      {line.old_line ?? ""}
                                    </span>
                                    <span class="de-gutter de-gutter-new">
                                      {line.new_line ?? ""}
                                    </span>
                                    <span class="de-line-prefix">
                                      {line.line_type === "add" ? "+" : line.line_type === "remove" ? "-" : " "}
                                    </span>
                                    <span class="de-line-content">{line.content || "\n"}</span>
                                  </div>
                                )}
                              </For>
                            </div>
                          </Show>
                        </div>
                      );
                    }}
                  </For>
                </div>
              )}
            </Show>
          </div>
        </div>
    </div>
  );
}

const DIFF_STYLES = `
  .de-pane {
    background: var(--bg-card);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  .de-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .de-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .de-header-icon {
    color: var(--primary);
    flex-shrink: 0;
  }
  .de-header h3 {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.2px;
    margin: 0;
  }
  .de-stat-summary {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 11px;
    margin-left: 4px;
  }
  .de-stat-files {
    color: var(--text-tertiary);
  }
  .de-stat-ins {
    color: var(--green);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
  }
  .de-stat-del {
    color: var(--red);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 500;
  }

  .de-header-actions { display: flex; align-items: center; gap: 4px; }
  .de-icon-btn {
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
  .de-icon-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }

  .de-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* File list sidebar */
  .de-file-list {
    width: 260px;
    min-width: 200px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    flex-shrink: 0;
    padding: 4px;
  }
  .de-empty {
    text-align: center;
    padding: 32px 12px;
    color: var(--text-tertiary);
    font-size: 12px;
  }
  .de-error { color: var(--red); }

  .de-file-item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: none;
    background: none;
    cursor: pointer;
    border-radius: var(--radius-sm);
    text-align: left;
    transition: background 0.1s;
    color: var(--text);
  }
  .de-file-item:hover { background: var(--bg-accent); }
  .de-file-selected {
    background: var(--bg-surface) !important;
    box-shadow: inset 2px 0 0 var(--primary);
  }

  .de-badge {
    font-family: var(--font-mono);
    font-size: 10px;
    font-weight: 700;
    width: 18px;
    height: 18px;
    display: flex;
    align-items: center;
    justify-content: center;
    border-radius: 4px;
    flex-shrink: 0;
    letter-spacing: 0;
  }
  .de-badge-modified { background: rgba(107, 124, 255, 0.15); color: var(--primary); }
  .de-badge-added { background: rgba(76, 214, 148, 0.15); color: var(--green); }
  .de-badge-deleted { background: rgba(242, 95, 103, 0.15); color: var(--red); }
  .de-badge-renamed { background: rgba(255, 180, 80, 0.15); color: #ffb450; }

  .de-file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .de-file-name {
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .de-file-dir {
    font-size: 10px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .de-file-stats {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    font-size: 10px;
    font-family: var(--font-mono);
  }

  /* Diff viewer */
  .de-diff-view {
    flex: 1;
    overflow: auto;
    min-width: 0;
  }
  .de-diff-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-tertiary);
    font-size: 13px;
  }
  .de-diff-content {
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
  }
  .de-diff-file-header {
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

  /* Hunks */
  .de-hunk {
    border-bottom: 1px solid var(--border);
  }
  .de-hunk-header {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 4px 12px;
    background: var(--bg-accent);
    border: none;
    cursor: pointer;
    color: var(--text-tertiary);
    font-family: var(--font-mono);
    font-size: 11px;
    text-align: left;
    transition: background 0.1s;
  }
  .de-hunk-header:hover {
    background: rgba(255, 255, 255, 0.04);
  }
  .de-hunk-header-text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .de-chevron {
    flex-shrink: 0;
    transition: transform 0.15s;
  }
  .de-chevron-open {
    transform: rotate(90deg);
  }

  /* Diff lines */
  .de-hunk-lines {
    overflow-x: auto;
  }
  .de-line {
    display: flex;
    min-height: 20px;
    white-space: pre;
  }
  .de-line-context {
    background: transparent;
  }
  .de-line-add {
    background: rgba(76, 214, 148, 0.08);
  }
  .de-line-add .de-line-prefix {
    color: var(--green);
  }
  .de-line-add .de-line-content {
    color: var(--green);
  }
  .de-line-remove {
    background: rgba(242, 95, 103, 0.08);
  }
  .de-line-remove .de-line-prefix {
    color: var(--red);
  }
  .de-line-remove .de-line-content {
    color: var(--red);
  }

  .de-gutter {
    width: 48px;
    min-width: 48px;
    text-align: right;
    padding-right: 8px;
    color: var(--text-tertiary);
    font-size: 11px;
    user-select: none;
    opacity: 0.5;
    border-right: 1px solid var(--border);
  }
  .de-gutter-old {
    border-right: none;
  }
  .de-line-prefix {
    width: 16px;
    min-width: 16px;
    text-align: center;
    user-select: none;
    color: var(--text-tertiary);
  }
  .de-line-content {
    flex: 1;
    padding-right: 16px;
    color: var(--text);
  }

  /* Keyboard hint */
  .de-kbd-hint {
    position: absolute;
    bottom: 12px;
    right: 16px;
    font-size: 10px;
    color: var(--text-tertiary);
    opacity: 0.5;
  }
`;
