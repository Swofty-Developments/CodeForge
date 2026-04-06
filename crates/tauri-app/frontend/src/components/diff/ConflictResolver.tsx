import { createSignal, onMount, For, Show } from "solid-js";
import * as ipc from "../../ipc";
import type { ConflictFile } from "../../ipc";

function fileName(path: string): string {
  const parts = path.split("/");
  return parts[parts.length - 1];
}

function dirPath(path: string): string {
  const parts = path.split("/");
  if (parts.length <= 1) return "";
  return parts.slice(0, -1).join("/") + "/";
}

export function ConflictResolver(props: { cwd: string; onDone: () => void }) {
  const [files, setFiles] = createSignal<string[]>([]);
  const [selectedFile, setSelectedFile] = createSignal<string | null>(null);
  const [markers, setMarkers] = createSignal<ConflictFile | null>(null);
  const [resolved, setResolved] = createSignal<Set<string>>(new Set());
  const [loading, setLoading] = createSignal(true);
  const [loadingMarkers, setLoadingMarkers] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  async function loadFiles() {
    setLoading(true);
    setError(null);
    try {
      const result = await ipc.getConflictFiles(props.cwd);
      setFiles(result);
      if (result.length > 0 && !selectedFile()) {
        await selectFile(result[0]);
      }
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function selectFile(filePath: string) {
    setSelectedFile(filePath);
    setMarkers(null);
    setLoadingMarkers(true);
    try {
      const m = await ipc.getConflictFile(props.cwd, filePath);
      if (selectedFile() === filePath) setMarkers(m);
    } catch (e) {
      setError(`Failed to load markers for ${filePath}: ${e}`);
    } finally {
      setLoadingMarkers(false);
    }
  }

  async function resolve(resolution: "ours" | "theirs" | "both") {
    const file = selectedFile();
    if (!file) return;
    try {
      await ipc.resolveConflict(props.cwd, file, resolution);
      setResolved((prev) => {
        const next = new Set(prev);
        next.add(file);
        return next;
      });
      // Auto-advance to next unresolved file
      const unresolvedFiles = files().filter((f) => !resolved().has(f) && f !== file);
      if (unresolvedFiles.length > 0) {
        await selectFile(unresolvedFiles[0]);
      } else {
        setMarkers(null);
      }
    } catch (e) {
      setError(`Failed to resolve ${file}: ${e}`);
    }
  }

  async function handleFinalize() {
    try {
      await ipc.finalizeMerge(props.cwd);
      props.onDone();
    } catch (e) {
      setError(`Finalize failed: ${e}`);
    }
  }

  async function handleAbort() {
    try {
      await ipc.abortMerge(props.cwd);
    } catch (e) {
      // still call onDone even if abort had issues
    }
    props.onDone();
  }

  const allResolved = () => {
    const f = files();
    return f.length > 0 && f.every((file) => resolved().has(file));
  };

  const unresolvedCount = () => files().filter((f) => !resolved().has(f)).length;

  onMount(() => {
    loadFiles();
    if (document.getElementById("cr-styles")) return;
    const style = document.createElement("style");
    style.id = "cr-styles";
    style.textContent = CR_STYLES;
    document.head.appendChild(style);
  });

  return (
    <div class="cr-pane">
      {/* Header */}
      <div class="cr-header">
        <div class="cr-header-left">
          <svg class="cr-header-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 3h5v5" /><path d="M8 3H3v5" />
            <path d="M12 22v-8.3a4 4 0 0 0-1.172-2.872L3 3" />
            <path d="m15 9 6-6" />
          </svg>
          <h3>Merge Conflicts</h3>
          <Show when={!loading() && files().length > 0}>
            <span class="cr-counter">
              {resolved().size}/{files().length} resolved
            </span>
          </Show>
        </div>
        <div class="cr-header-actions">
          <button class="cr-btn cr-btn-danger" onClick={handleAbort} title="Abort merge">
            Abort Merge
          </button>
          <Show when={allResolved()}>
            <button class="cr-btn cr-btn-primary" onClick={handleFinalize}>
              Finalize Merge
            </button>
          </Show>
        </div>
      </div>

      <Show when={error()}>
        <div class="cr-error">{error()}</div>
      </Show>

      <div class="cr-body">
        {/* File list sidebar */}
        <div class="cr-file-list">
          <Show when={loading()}>
            <div class="cr-loading">
              <div class="cr-skel-line" /><div class="cr-skel-line short" /><div class="cr-skel-line" />
            </div>
          </Show>
          <Show when={!loading() && files().length === 0 && !error()}>
            <div class="cr-empty">No merge conflicts</div>
          </Show>
          <For each={files()}>
            {(file) => {
              const isSelected = () => selectedFile() === file;
              const isResolved = () => resolved().has(file);
              return (
                <button
                  class="cr-file-item"
                  classList={{
                    "cr-file-selected": isSelected(),
                    "cr-file-resolved": isResolved(),
                  }}
                  onClick={() => selectFile(file)}
                  title={file}
                >
                  <span class="cr-file-status" classList={{ "cr-file-status--done": isResolved() }}>
                    {isResolved() ? "\u2713" : "!"}
                  </span>
                  <div class="cr-file-info">
                    <span class="cr-file-name">{fileName(file)}</span>
                    <span class="cr-file-dir">{dirPath(file)}</span>
                  </div>
                </button>
              );
            }}
          </For>
        </div>

        {/* Conflict panes */}
        <div class="cr-conflict-view">
          <Show when={loadingMarkers()}>
            <div class="cr-conflict-empty">Loading conflict markers...</div>
          </Show>

          <Show when={!loadingMarkers() && !markers() && !loading()}>
            <div class="cr-conflict-empty">
              <Show when={allResolved()} fallback={<span>Select a file to resolve</span>}>
                <div class="cr-all-done">
                  <svg width="32" height="32" viewBox="0 0 24 24" fill="none" stroke="var(--green)" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                    <path d="M22 11.08V12a10 10 0 1 1-5.93-9.14" />
                    <polyline points="22 4 12 14.01 9 11.01" />
                  </svg>
                  <span>All conflicts resolved</span>
                  <span class="cr-all-done-hint">Click "Finalize Merge" to complete</span>
                </div>
              </Show>
            </div>
          </Show>

          <Show when={markers()}>
            {(m) => (
              <div class="cr-panes-wrapper">
                <div class="cr-panes">
                  <div class="cr-pane-col">
                    <div class="cr-pane-label cr-pane-label-ours">Ours (current)</div>
                    <pre class="cr-code">{m().ours}</pre>
                  </div>
                  <div class="cr-pane-col">
                    <div class="cr-pane-label cr-pane-label-theirs">Theirs (incoming)</div>
                    <pre class="cr-code">{m().theirs}</pre>
                  </div>
                </div>

                <Show when={!resolved().has(selectedFile()!)}>
                  <div class="cr-actions">
                    <button class="cr-btn cr-btn-ours" onClick={() => resolve("ours")}>
                      Accept Ours
                    </button>
                    <button class="cr-btn cr-btn-both" onClick={() => resolve("both")}>
                      Accept Both
                    </button>
                    <button class="cr-btn cr-btn-theirs" onClick={() => resolve("theirs")}>
                      Accept Theirs
                    </button>
                  </div>
                </Show>

                <Show when={resolved().has(selectedFile()!)}>
                  <div class="cr-resolved-banner">Resolved</div>
                </Show>
              </div>
            )}
          </Show>
        </div>
      </div>
    </div>
  );
}

const CR_STYLES = `
  .cr-pane {
    background: var(--bg-card);
    border-left: 1px solid var(--border);
    display: flex;
    flex-direction: column;
    overflow: hidden;
    height: 100%;
  }

  .cr-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 14px 18px;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .cr-header-left {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .cr-header-icon {
    color: var(--primary);
    flex-shrink: 0;
  }
  .cr-header h3 {
    font-size: 14px;
    font-weight: 600;
    letter-spacing: -0.2px;
    margin: 0;
  }
  .cr-counter {
    font-size: 11px;
    color: var(--text-tertiary);
    margin-left: 4px;
  }
  .cr-header-actions {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .cr-error {
    padding: 8px 18px;
    font-size: 12px;
    color: var(--red);
    background: rgba(242, 95, 103, 0.08);
    border-bottom: 1px solid var(--border);
  }

  .cr-body {
    display: flex;
    flex: 1;
    min-height: 0;
  }

  /* File list */
  .cr-file-list {
    width: 240px;
    min-width: 180px;
    border-right: 1px solid var(--border);
    overflow-y: auto;
    flex-shrink: 0;
    padding: 4px;
  }
  .cr-empty {
    text-align: center;
    padding: 32px 12px;
    color: var(--text-tertiary);
    font-size: 12px;
  }
  .cr-loading {
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }
  .cr-skel-line {
    height: 10px;
    background: var(--bg-accent);
    border-radius: 4px;
    animation: cr-pulse 1.2s ease-in-out infinite;
  }
  .cr-skel-line.short { width: 60%; }
  @keyframes cr-pulse {
    0%, 100% { opacity: 0.4; }
    50% { opacity: 0.8; }
  }

  .cr-file-item {
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
  .cr-file-item:hover { background: var(--bg-accent); }
  .cr-file-selected {
    background: var(--bg-surface) !important;
    box-shadow: inset 2px 0 0 var(--primary);
  }
  .cr-file-resolved { opacity: 0.6; }

  .cr-file-status {
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
    background: rgba(242, 95, 103, 0.15);
    color: var(--red);
  }
  .cr-file-status--done {
    background: rgba(76, 214, 148, 0.15);
    color: var(--green);
  }

  .cr-file-info {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 1px;
  }
  .cr-file-name {
    font-size: 12px;
    font-weight: 500;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .cr-file-dir {
    font-size: 10px;
    color: var(--text-tertiary);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  /* Conflict view */
  .cr-conflict-view {
    flex: 1;
    overflow: auto;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .cr-conflict-empty {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    color: var(--text-tertiary);
    font-size: 13px;
  }
  .cr-all-done {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
  }
  .cr-all-done-hint {
    font-size: 11px;
    color: var(--text-tertiary);
  }

  .cr-panes-wrapper {
    display: flex;
    flex-direction: column;
    flex: 1;
    min-height: 0;
  }

  .cr-panes {
    display: flex;
    flex: 1;
    min-height: 0;
    gap: 1px;
    background: var(--border);
  }
  .cr-pane-col {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
    background: var(--bg-card);
  }
  .cr-pane-label {
    padding: 6px 12px;
    font-size: 11px;
    font-weight: 600;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    border-bottom: 1px solid var(--border);
    flex-shrink: 0;
  }
  .cr-pane-label-ours {
    color: var(--green);
    background: rgba(76, 214, 148, 0.06);
  }
  .cr-pane-label-theirs {
    color: var(--primary);
    background: rgba(107, 124, 255, 0.06);
  }

  .cr-code {
    flex: 1;
    overflow: auto;
    margin: 0;
    padding: 12px;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 1.6;
    color: var(--text);
    white-space: pre-wrap;
    word-break: break-all;
  }

  /* Resolution actions */
  .cr-actions {
    display: flex;
    justify-content: center;
    gap: 8px;
    padding: 12px 16px;
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  .cr-resolved-banner {
    text-align: center;
    padding: 10px;
    font-size: 12px;
    font-weight: 600;
    color: var(--green);
    background: rgba(76, 214, 148, 0.08);
    border-top: 1px solid var(--border);
    flex-shrink: 0;
  }

  /* Buttons */
  .cr-btn {
    font-size: 12px;
    font-weight: 600;
    padding: 6px 14px;
    border-radius: var(--radius-sm);
    border: 1px solid var(--border);
    background: var(--bg-surface);
    color: var(--text);
    cursor: pointer;
    transition: background 0.12s, color 0.12s, border-color 0.12s;
  }
  .cr-btn:hover {
    background: var(--bg-accent);
  }

  .cr-btn-primary {
    background: var(--green);
    color: #fff;
    border-color: var(--green);
  }
  .cr-btn-primary:hover {
    filter: brightness(1.1);
    background: var(--green);
  }

  .cr-btn-danger {
    color: var(--red);
    border-color: rgba(242, 95, 103, 0.3);
  }
  .cr-btn-danger:hover {
    background: rgba(242, 95, 103, 0.1);
  }

  .cr-btn-ours {
    color: var(--green);
    border-color: rgba(76, 214, 148, 0.3);
  }
  .cr-btn-ours:hover {
    background: rgba(76, 214, 148, 0.1);
  }

  .cr-btn-theirs {
    color: var(--primary);
    border-color: rgba(107, 124, 255, 0.3);
  }
  .cr-btn-theirs:hover {
    background: rgba(107, 124, 255, 0.1);
  }

  .cr-btn-both {
    color: var(--text-secondary);
  }
  .cr-btn-both:hover {
    background: var(--bg-accent);
  }
`;
