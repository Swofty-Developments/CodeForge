import { createSignal, onMount, For, Show, createEffect, createMemo } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { ChangedFile, FileDiff, DiffHunk } from "../../ipc";
import { GitPanel } from "../git/GitPanel";

// --- File tree types ---
interface TreeNode {
  name: string;
  path: string;
  isDir: boolean;
  children: TreeNode[];
  file?: ChangedFile;
  insertions: number;
  deletions: number;
}

function buildFileTree(files: ChangedFile[]): TreeNode[] {
  const root: TreeNode = { name: "", path: "", isDir: true, children: [], insertions: 0, deletions: 0 };

  for (const file of files) {
    const parts = file.path.split("/");
    let current = root;
    for (let i = 0; i < parts.length; i++) {
      const part = parts[i];
      const isLast = i === parts.length - 1;
      const partPath = parts.slice(0, i + 1).join("/");

      if (isLast) {
        current.children.push({
          name: part,
          path: file.path,
          isDir: false,
          children: [],
          file,
          insertions: file.insertions,
          deletions: file.deletions,
        });
      } else {
        let dir = current.children.find((c) => c.isDir && c.name === part);
        if (!dir) {
          dir = { name: part, path: partPath, isDir: true, children: [], insertions: 0, deletions: 0 };
          current.children.push(dir);
        }
        current = dir;
      }
    }
  }

  // Aggregate stats up the tree
  function aggregate(node: TreeNode): void {
    if (!node.isDir) return;
    node.insertions = 0;
    node.deletions = 0;
    for (const child of node.children) {
      aggregate(child);
      node.insertions += child.insertions;
      node.deletions += child.deletions;
    }
  }

  // Collapse single-child directories (src/components -> src/components)
  function collapse(nodes: TreeNode[]): TreeNode[] {
    return nodes.map((node) => {
      if (node.isDir && node.children.length === 1 && node.children[0].isDir) {
        const child = node.children[0];
        return collapse([{ ...child, name: `${node.name}/${child.name}` }])[0];
      }
      if (node.isDir) {
        return { ...node, children: collapse(node.children) };
      }
      return node;
    });
  }

  aggregate(root);
  // Sort: dirs first, then files, alphabetically
  function sortTree(nodes: TreeNode[]): TreeNode[] {
    return nodes
      .map((n) => (n.isDir ? { ...n, children: sortTree(n.children) } : n))
      .sort((a, b) => {
        if (a.isDir && !b.isDir) return -1;
        if (!a.isDir && b.isDir) return 1;
        return a.name.localeCompare(b.name);
      });
  }

  return sortTree(collapse(root.children));
}

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

function FileTree(props: { nodes: TreeNode[]; selectedFile: string | null; onSelect: (path: string) => void; depth?: number }) {
  const depth = props.depth ?? 0;
  return (
    <For each={props.nodes}>
      {(node) => {
        if (node.isDir) {
          const [open, setOpen] = createSignal(true);
          return (
            <div class="de-tree-dir">
              <button
                class="de-tree-dir-header"
                style={{ "padding-left": `${8 + depth * 12}px` }}
                onClick={() => setOpen(!open())}
              >
                <svg class="de-tree-chevron" classList={{ "de-tree-chevron--open": open() }} width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><polyline points="9 18 15 12 9 6" /></svg>
                <svg class="de-tree-folder" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><path d="M22 19a2 2 0 01-2 2H4a2 2 0 01-2-2V5a2 2 0 012-2h5l2 3h9a2 2 0 012 2z" /></svg>
                <span class="de-tree-dir-name">{node.name}</span>
                <span class="de-tree-dir-stats">
                  <Show when={node.insertions > 0}><span class="de-stat-ins">+{node.insertions}</span></Show>
                  <Show when={node.deletions > 0}><span class="de-stat-del">-{node.deletions}</span></Show>
                </span>
              </button>
              <Show when={open()}>
                <FileTree nodes={node.children} selectedFile={props.selectedFile} onSelect={props.onSelect} depth={depth + 1} />
              </Show>
            </div>
          );
        }
        const badge = statusBadge(node.file!.status);
        const isSelected = () => props.selectedFile === node.path;
        return (
          <button
            class={`de-file-item ${isSelected() ? "de-file-selected" : ""}`}
            style={{ "padding-left": `${8 + depth * 12}px` }}
            onClick={() => props.onSelect(node.path)}
            title={node.path}
          >
            <span class={`de-badge ${badge.cls}`}>{badge.label}</span>
            <span class="de-file-name">{node.name}</span>
            <div class="de-file-stats">
              <Show when={node.insertions > 0}><span class="de-stat-ins">+{node.insertions}</span></Show>
              <Show when={node.deletions > 0}><span class="de-stat-del">-{node.deletions}</span></Show>
            </div>
          </button>
        );
      }}
    </For>
  );
}

export function DiffEditor(props: { cwd: string; prNumber?: number | null }) {
  const { setStore } = appStore;
  const [files, setFiles] = createSignal<ChangedFile[]>([]);
  const [diffs, setDiffs] = createSignal<FileDiff[]>([]);
  const [selectedFile, setSelectedFile] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [collapsedHunks, setCollapsedHunks] = createSignal<Set<string>>(new Set());
  const [gitPanelOpen, setGitPanelOpen] = createSignal(true);
  const [diffMode, setDiffMode] = createSignal<"both" | "pr" | "worktree">("both");

  // Separate storage for PR diff and worktree diff
  const [prFiles, setPrFiles] = createSignal<ChangedFile[]>([]);
  const [prDiffs, setPrDiffs] = createSignal<FileDiff[]>([]);
  const [wtFiles, setWtFiles] = createSignal<ChangedFile[]>([]);
  const [wtDiffs, setWtDiffs] = createSignal<FileDiff[]>([]);

  const hasPr = () => !!props.prNumber;

  function close() {
    const tab = appStore.store.activeTab;
    if (tab) setStore("threadDiffOpen", tab, false);
  }

  async function loadAll() {
    const cwd = props.cwd;
    const prNum = props.prNumber;
    if (!cwd || cwd === ".") {
      setFiles([]); setDiffs([]);
      setLoading(false);
      return;
    }

    setLoading(true);
    setError(null);
    setFiles([]); setDiffs([]);
    setPrFiles([]); setPrDiffs([]);
    setWtFiles([]); setWtDiffs([]);
    setSelectedFile(null);

    try {
      // Always fetch worktree/local changes
      const [localFiles, localDiffs] = await Promise.all([
        ipc.getChangedFiles(cwd).catch(() => []),
        ipc.getSessionDiff(cwd).catch(() => []),
      ]);
      if (props.cwd === cwd) {
        setWtFiles(localFiles as any);
        setWtDiffs(localDiffs as any);
      }

      // If PR linked, also fetch PR diff
      if (prNum) {
        try {
          const prDiffRaw = await ipc.getPrDiff(cwd, prNum);
          const parsed = parsePrDiff(prDiffRaw);
          if (props.cwd === cwd) {
            setPrFiles(parsed.files);
            setPrDiffs(parsed.diffs);
          }
        } catch {}
      }

      // Apply current mode
      if (props.cwd === cwd) applyMode();
    } catch (e) {
      if (props.cwd === cwd) setError(String(e));
    } finally {
      if (props.cwd === cwd) setLoading(false);
    }
  }

  function applyMode() {
    const mode = diffMode();
    let mergedFiles: any[] = [];
    let mergedDiffs: any[] = [];

    if (mode === "pr" && hasPr()) {
      mergedFiles = prFiles();
      mergedDiffs = prDiffs();
    } else if (mode === "worktree") {
      mergedFiles = wtFiles();
      mergedDiffs = wtDiffs();
    } else {
      // "both" — merge PR + worktree, deduplicating by path (worktree wins)
      const fileMap = new Map<string, any>();
      const diffMap = new Map<string, any>();
      for (const f of prFiles()) { fileMap.set(f.path, { ...f, source: "pr" }); }
      for (const d of prDiffs()) { diffMap.set(d.path, d); }
      for (const f of wtFiles()) { fileMap.set(f.path, { ...f, source: "worktree" }); }
      for (const d of wtDiffs()) { diffMap.set(d.path, d); }
      mergedFiles = Array.from(fileMap.values());
      mergedDiffs = Array.from(diffMap.values());
    }

    setFiles(mergedFiles);
    setDiffs(mergedDiffs);
    if (mergedFiles.length > 0 && !selectedFile()) {
      setSelectedFile(mergedFiles[0].path);
    }
  }

  // Re-apply mode when diffMode changes (without re-fetching)
  createEffect(() => {
    const _ = diffMode();
    if (!loading()) applyMode();
  });

  /** Parse raw unified diff text into ChangedFile[] and FileDiff[] */
  function parsePrDiff(raw: string): { files: any[]; diffs: any[] } {
    const files: any[] = [];
    const diffs: any[] = [];
    if (!raw) return { files, diffs };

    const fileSections = raw.split(/^diff --git /m).filter(Boolean);
    for (const section of fileSections) {
      const lines = section.split("\n");
      // Extract file path from "a/path b/path"
      const headerMatch = lines[0]?.match(/a\/(.+?) b\/(.+)/);
      const path = headerMatch ? headerMatch[2] : "unknown";

      let insertions = 0, deletions = 0;
      const hunkLines: any[] = [];
      let hunkHeader = "";

      for (const line of lines.slice(1)) {
        if (line.startsWith("@@")) {
          hunkHeader = line;
        } else if (line.startsWith("+") && !line.startsWith("+++")) {
          insertions++;
          hunkLines.push({ line_type: "add", content: line.slice(1), old_line: null, new_line: null });
        } else if (line.startsWith("-") && !line.startsWith("---")) {
          deletions++;
          hunkLines.push({ line_type: "delete", content: line.slice(1), old_line: null, new_line: null });
        } else if (!line.startsWith("\\") && !line.startsWith("index") && !line.startsWith("---") && !line.startsWith("+++") && !line.startsWith("new") && !line.startsWith("old") && !line.startsWith("deleted") && !line.startsWith("similarity")) {
          hunkLines.push({ line_type: "context", content: line.startsWith(" ") ? line.slice(1) : line, old_line: null, new_line: null });
        }
      }

      const status = deletions > 0 && insertions > 0 ? "modified" : insertions > 0 ? "added" : "deleted";
      files.push({ path, status, insertions, deletions });
      diffs.push({ path, hunks: [{ header: hunkHeader, lines: hunkLines }] });
    }

    return { files, diffs };
  }

  // Reload when cwd or prNumber changes (thread switch)
  createEffect(() => {
    const _ = props.cwd;
    const _pr = props.prNumber;
    loadAll();
  });

  const totalInsertions = () => files().reduce((s, f) => s + f.insertions, 0);
  const totalDeletions = () => files().reduce((s, f) => s + f.deletions, 0);
  const fileTree = createMemo(() => buildFileTree(files()));

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
            <h3>{props.prNumber ? `PR #${props.prNumber}` : "Changes"}</h3>
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
          <Show when={hasPr()}>
            <div class="de-mode-toggle">
              <button
                class="de-mode-btn"
                classList={{ "de-mode-btn--active": diffMode() === "both" }}
                onClick={() => setDiffMode("both")}
              >Both</button>
              <button
                class="de-mode-btn"
                classList={{ "de-mode-btn--active": diffMode() === "pr" }}
                onClick={() => setDiffMode("pr")}
              >PR</button>
              <button
                class="de-mode-btn"
                classList={{ "de-mode-btn--active": diffMode() === "worktree" }}
                onClick={() => setDiffMode("worktree")}
              >Local</button>
            </div>
          </Show>
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
              <div class="de-loading-skeleton">
                <div class="de-skel-line" /><div class="de-skel-line short" /><div class="de-skel-line" />
              </div>
            </Show>
            <Show when={error()}>
              <div class="de-empty de-error">{error()}</div>
            </Show>
            <Show when={!loading() && files().length === 0 && !error()}>
              <div class="de-empty">No changes detected</div>
            </Show>
            <FileTree
              nodes={fileTree()}
              selectedFile={selectedFile()}
              onSelect={setSelectedFile}
            />
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

        {/* Git management panel — collapsible */}
        <div class="de-git-section">
          <button class="de-git-toggle" onClick={() => setGitPanelOpen(!gitPanelOpen())}>
            <svg class="de-git-chevron" classList={{ "de-git-chevron--open": gitPanelOpen() }} width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round">
              <polyline points="9 18 15 12 9 6" />
            </svg>
            <span>Git</span>
          </button>
          <Show when={gitPanelOpen()}>
            <GitPanel cwd={props.cwd} />
          </Show>
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

  .de-mode-toggle {
    display: flex;
    gap: 1px;
    background: var(--bg-muted);
    border-radius: var(--radius-sm);
    padding: 2px;
    margin-left: auto;
  }
  .de-mode-btn {
    font-size: 10px;
    font-weight: 600;
    padding: 3px 8px;
    border-radius: 4px;
    color: var(--text-tertiary);
    transition: all 0.1s;
  }
  .de-mode-btn:hover { color: var(--text-secondary); }
  .de-mode-btn--active {
    background: var(--bg-accent);
    color: var(--text);
    box-shadow: 0 1px 3px rgba(0,0,0,0.15);
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

  /* Loading skeleton */
  .de-loading-skeleton {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .de-skel-line {
    height: 10px;
    background: var(--bg-accent);
    border-radius: 4px;
    animation: de-pulse 1.2s ease-in-out infinite;
  }
  .de-skel-line.short { width: 60%; }
  @keyframes de-pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

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

  /* Collapsible git section */
  .de-git-section {
    flex-shrink: 0;
    border-top: 1px solid var(--border);
  }
  .de-git-toggle {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
    color: var(--text-tertiary);
    text-transform: uppercase;
    letter-spacing: 0.04em;
    transition: color 0.1s, background 0.1s;
  }
  .de-git-toggle:hover {
    color: var(--text-secondary);
    background: var(--bg-hover);
  }
  .de-git-chevron {
    flex-shrink: 0;
    transition: transform 0.15s ease;
  }
  .de-git-chevron--open {
    transform: rotate(90deg);
  }

  /* File tree styles */
  .de-tree-dir { display: flex; flex-direction: column; }
  .de-tree-dir-header {
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 4px 8px;
    font-size: 11px;
    color: var(--text-secondary);
    border-radius: var(--radius-sm);
    transition: background 0.1s;
    text-align: left;
    width: 100%;
  }
  .de-tree-dir-header:hover { background: var(--bg-hover); }
  .de-tree-chevron {
    flex-shrink: 0;
    color: var(--text-tertiary);
    transition: transform 0.15s ease;
  }
  .de-tree-chevron--open { transform: rotate(90deg); }
  .de-tree-folder { flex-shrink: 0; color: var(--text-tertiary); }
  .de-tree-dir-name {
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1;
    min-width: 0;
  }
  .de-tree-dir-stats {
    display: flex;
    gap: 4px;
    flex-shrink: 0;
    font-family: var(--font-mono);
    font-size: 10px;
  }
`;
