import { For, onMount, onCleanup, createSignal, createResource } from "solid-js";
import { appStore } from "../../stores/app-store";
import { type Theme, getThemes, applyTheme } from "../../themes";
import { getSetting, importTheme, deleteCustomTheme, exportTheme } from "../../ipc";

export function ThemeSelector(props?: { inline?: boolean }) {
  const { setStore } = appStore;
  const [activeId, setActiveId] = createSignal("obsidian-forge");
  const [themes, { refetch }] = createResource<Theme[]>(getThemes, { initialValue: [] });

  onMount(async () => {
    try {
      const saved = await getSetting("theme");
      if (saved) setActiveId(saved);
    } catch {}
  });

  function close() {
    setStore("themeOpen", false);
  }

  function handleKeyDown(e: KeyboardEvent) {
    if (e.key === "Escape") {
      e.stopPropagation();
      close();
    }
  }

  onMount(() => window.addEventListener("keydown", handleKeyDown));
  onCleanup(() => window.removeEventListener("keydown", handleKeyDown));

  function selectTheme(id: string) {
    applyTheme(id, themes());
    setActiveId(id);
  }

  async function handleImport() {
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const file = await open({
        multiple: false,
        filters: [{ name: "Theme", extensions: ["json"] }],
      });
      if (!file) return;
      const path = typeof file === "string" ? file : file.path;
      // Read file via fetch (Tauri asset protocol) or use fs plugin
      // Since we have the path, read it via a small invoke or use the Rust side
      // Actually, we pass the path content through the backend — but import_theme expects JSON string.
      // Use the web File API via an input element instead for simplicity:
      const response = await fetch(`asset://localhost/${path}`);
      const content = await response.text();
      await importTheme(content);
      refetch();
    } catch (e) {
      console.error("Failed to import theme:", e);
    }
  }

  async function handleExport(id: string, name: string) {
    try {
      const { save } = await import("@tauri-apps/plugin-dialog");
      const path = await save({
        defaultPath: `${id}.json`,
        filters: [{ name: "Theme", extensions: ["json"] }],
      });
      if (!path) return;
      const content = await exportTheme(id);
      // Write via Tauri fs — but we don't have fs plugin. Use invoke to write or
      // use the export content differently. For now, download via blob as fallback.
      // Actually, let's write via a simple Rust helper or use the content.
      // We can use the backend: the save dialog gives us a path, but we need to write.
      // Simplest: use the existing IPC to get content + Blob download.
      const blob = new Blob([content], { type: "application/json" });
      const url = URL.createObjectURL(blob);
      const a = document.createElement("a");
      a.href = url;
      a.download = `${name.toLowerCase().replace(/\s+/g, "-")}.json`;
      a.click();
      URL.revokeObjectURL(url);
    } catch (e) {
      console.error("Failed to export theme:", e);
    }
  }

  async function handleDelete(id: string) {
    try {
      await deleteCustomTheme(id);
      refetch();
      // If the deleted theme was active, switch to default
      if (activeId() === id) {
        selectTheme("obsidian-forge");
      }
    } catch (e) {
      console.error("Failed to delete theme:", e);
    }
  }

  const content = (
    <>
      <div class="theme-selector-header">
        <h2 class="theme-selector-title">Themes</h2>
        <div class="theme-header-actions">
          <button class="theme-import-btn" onClick={handleImport} title="Import theme">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
              <polyline points="17 8 12 3 7 8" />
              <line x1="12" y1="3" x2="12" y2="15" />
            </svg>
            <span>Import</span>
          </button>
          {!props?.inline && (
            <button class="theme-close-btn" onClick={close} title="Close">
              <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          )}
        </div>
      </div>
      <div class="theme-grid">
          <For each={themes()}>
            {(theme) => (
              <button
                class="theme-card"
                classList={{ active: activeId() === theme.id }}
                onClick={() => selectTheme(theme.id)}
              >
                <div class="theme-preview" style={{ background: theme.vars["--bg-base"] || theme.preview[0] }}>
                  <div class="theme-preview-sidebar" style={{ background: theme.vars["--bg-surface"] || theme.preview[1] }}>
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--primary"] || theme.preview[2], opacity: "0.7" }} />
                    <div class="theme-preview-sidebar-line" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                  </div>
                  <div class="theme-preview-main">
                    <div class="theme-preview-topbar" style={{ background: theme.vars["--text-tertiary"] || "#888" }} />
                    <div class="theme-preview-msg-user" style={{ background: theme.vars["--primary"] || theme.preview[2] }} />
                    <div class="theme-preview-msg-bot" style={{ background: theme.vars["--text"] || "#fff" }} />
                    <div class="theme-preview-composer" style={{ background: theme.vars["--text"] || "#fff" }} />
                  </div>
                </div>
                <div class="theme-info">
                  <span class="theme-name">{theme.name}</span>
                  <span class="theme-desc">{theme.description}</span>
                </div>
                <div class="theme-card-badges">
                  {activeId() === theme.id && <span class="theme-badge">Active</span>}
                  {theme.is_custom && <span class="theme-badge theme-badge-custom">Custom</span>}
                </div>
                <div class="theme-card-actions" onClick={(e) => e.stopPropagation()}>
                  <button
                    class="theme-action-btn"
                    onClick={() => handleExport(theme.id, theme.name)}
                    title="Export theme"
                  >
                    <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
                      <polyline points="7 10 12 15 17 10" />
                      <line x1="12" y1="15" x2="12" y2="3" />
                    </svg>
                  </button>
                  {theme.is_custom && (
                    <button
                      class="theme-action-btn theme-action-delete"
                      onClick={() => handleDelete(theme.id)}
                      title="Delete custom theme"
                    >
                      <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                        <polyline points="3 6 5 6 21 6" />
                        <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                      </svg>
                    </button>
                  )}
                </div>
              </button>
            )}
          </For>
        </div>
    </>
  );

  if (props?.inline) {
    return <div class="theme-selector-inline">{content}</div>;
  }

  return (
    <div class="overlay" onClick={(e) => { if (e.target === e.currentTarget) close(); }}>
      <div class="overlay-panel theme-selector-panel">{content}</div>
    </div>
  );
}

if (!document.getElementById("theme-selector-styles")) {
  const s = document.createElement("style");
  s.id = "theme-selector-styles";
  s.textContent = `
    .theme-selector-inline {
      max-width: 680px;
      margin: 0 auto;
      padding: 24px;
      width: 100%;
    }
    .theme-selector-panel {
      max-width: 680px;
      width: 94%;
      max-height: 80vh;
      overflow-y: auto;
      padding: 20px 24px 24px;
    }
    .theme-selector-header {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 16px;
    }
    .theme-selector-title {
      font-size: 16px;
      font-weight: 700;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .theme-header-actions {
      display: flex;
      align-items: center;
      gap: 8px;
    }
    .theme-import-btn {
      display: flex;
      align-items: center;
      gap: 5px;
      padding: 5px 10px;
      border-radius: var(--radius-sm);
      font-size: 12px;
      font-weight: 500;
      color: var(--text-secondary);
      background: var(--bg-accent);
      border: 1px solid var(--border);
      cursor: pointer;
      transition: background 0.15s, color 0.15s, border-color 0.15s;
    }
    .theme-import-btn:hover {
      background: var(--bg-muted);
      color: var(--text);
      border-color: var(--border-strong);
    }
    .theme-close-btn {
      color: var(--text-tertiary);
      padding: 5px;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      transition: background 0.15s, color 0.15s;
    }
    .theme-close-btn:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
    .theme-grid {
      display: grid;
      grid-template-columns: repeat(2, 1fr);
      gap: 10px;
    }
    @media (min-width: 620px) {
      .theme-grid {
        grid-template-columns: repeat(3, 1fr);
      }
    }
    .theme-card {
      position: relative;
      display: flex;
      flex-direction: column;
      gap: 10px;
      padding: 12px;
      border-radius: var(--radius-md);
      border: 1px solid var(--border);
      background: var(--bg-muted);
      text-align: left;
      cursor: pointer;
      transition: border-color 0.15s, background 0.15s, box-shadow 0.15s;
    }
    .theme-card:hover {
      border-color: var(--border-strong);
      background: var(--bg-accent);
    }
    .theme-card.active {
      border-color: var(--primary);
      box-shadow: 0 0 0 1px var(--primary-glow);
    }
    .theme-preview {
      display: flex;
      height: 80px;
      border-radius: 6px;
      overflow: hidden;
      border: 1px solid rgba(128,128,128,0.15);
    }
    .theme-preview-sidebar {
      width: 22%;
      display: flex;
      flex-direction: column;
      gap: 3px;
      padding: 4px 3px;
    }
    .theme-preview-sidebar-line {
      height: 3px;
      border-radius: 1px;
      opacity: 0.5;
    }
    .theme-preview-main {
      flex: 1;
      display: flex;
      flex-direction: column;
      padding: 4px 5px;
      gap: 3px;
    }
    .theme-preview-topbar {
      height: 4px;
      border-radius: 1px;
      opacity: 0.3;
    }
    .theme-preview-msg-user {
      height: 6px;
      border-radius: 3px;
      align-self: flex-end;
      width: 50%;
      opacity: 0.15;
    }
    .theme-preview-msg-bot {
      height: 10px;
      border-radius: 3px;
      width: 70%;
      opacity: 0.08;
    }
    .theme-preview-composer {
      margin-top: auto;
      height: 8px;
      border-radius: 4px;
      opacity: 0.12;
    }
    .theme-info {
      display: flex;
      flex-direction: column;
      gap: 2px;
      min-width: 0;
    }
    .theme-name {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
      white-space: nowrap;
      overflow: hidden;
      text-overflow: ellipsis;
    }
    .theme-desc {
      font-size: 11px;
      color: var(--text-tertiary);
      line-height: 1.3;
      display: -webkit-box;
      -webkit-line-clamp: 2;
      -webkit-box-orient: vertical;
      overflow: hidden;
    }
    .theme-card-badges {
      position: absolute;
      top: 8px;
      right: 8px;
      display: flex;
      gap: 4px;
    }
    .theme-badge {
      font-size: 10px;
      font-weight: 600;
      padding: 2px 7px;
      border-radius: var(--radius-pill);
      background: var(--primary);
      color: #fff;
      letter-spacing: 0.02em;
    }
    .theme-badge-custom {
      background: var(--purple, #b47aff);
    }
    .theme-card-actions {
      position: absolute;
      bottom: 8px;
      right: 8px;
      display: flex;
      gap: 4px;
      opacity: 0;
      transition: opacity 0.15s;
    }
    .theme-card:hover .theme-card-actions {
      opacity: 1;
    }
    .theme-action-btn {
      display: flex;
      align-items: center;
      justify-content: center;
      width: 24px;
      height: 24px;
      border-radius: var(--radius-sm);
      background: var(--bg-surface);
      border: 1px solid var(--border);
      color: var(--text-secondary);
      cursor: pointer;
      transition: background 0.15s, color 0.15s;
    }
    .theme-action-btn:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
    .theme-action-delete:hover {
      color: var(--red);
    }
  `;
  document.head.appendChild(s);
}
