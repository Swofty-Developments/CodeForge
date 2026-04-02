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

  const activeThemeName = () => {
    const t = themes().find((th) => th.id === activeId());
    return t ? t.name : "Obsidian Forge";
  };

  const content = (
    <>
      <div class="theme-selector-header">
        <div>
          <h2 class="theme-selector-title">Themes</h2>
          <p class="theme-selector-desc">Choose a theme or import a custom one</p>
          <span class="theme-selector-active">Active: {activeThemeName()}</span>
        </div>
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
              <div
                class="theme-card"
                classList={{ active: activeId() === theme.id }}
                onClick={() => selectTheme(theme.id)}
              >
                <div class="tp" style={{ background: theme.vars["--bg-base"] || "#0f1012" }}>
                  <div class="tp-side" style={{ background: theme.vars["--bg-surface"] || "#18181c", "border-right": `1px solid ${theme.vars["--border"] || "rgba(255,255,255,0.06)"}` }}>
                    <div class="tp-sln" style={{ background: theme.vars["--primary"] || "#6b7cff", opacity: "0.7" }} />
                    <div class="tp-sln" style={{ background: theme.vars["--bg-accent"] || "#222", height: "4px" }} />
                    <div class="tp-sln" style={{ background: theme.vars["--text-tertiary"] || "#888", opacity: "0.2" }} />
                    <div class="tp-sln" style={{ background: theme.vars["--text-tertiary"] || "#888", opacity: "0.15" }} />
                  </div>
                  <div class="tp-body">
                    <div class="tp-tabbar" style={{ background: theme.vars["--bg-surface"] || "#18181c" }}>
                      <div class="tp-tab-a" style={{ background: theme.vars["--bg-base"] || "#0f1012", border: `1px solid ${theme.vars["--border"] || "rgba(255,255,255,0.06)"}` }} />
                      <div class="tp-tab-i" style={{ background: theme.vars["--text-tertiary"] || "#888", opacity: "0.2" }} />
                    </div>
                    <div class="tp-msgs">
                      <div class="tp-u" style={{ background: `${theme.vars["--primary"] || "#6b7cff"}15`, border: `1px solid ${theme.vars["--primary"] || "#6b7cff"}20` }}>
                        <div class="tp-l" style={{ background: theme.vars["--text"] || "#eee", width: "70%" }} />
                      </div>
                      <div class="tp-a">
                        <div class="tp-l tp-h" style={{ background: theme.vars["--text"] || "#eee", width: "35%" }} />
                        <div class="tp-l" style={{ background: theme.vars["--text-secondary"] || "#aaa", width: "88%" }} />
                        <div class="tp-l" style={{ background: theme.vars["--text-secondary"] || "#aaa", width: "55%" }} />
                        <div class="tp-cb" style={{ background: theme.vars["--bg-card"] || "#151518", border: `1px solid ${theme.vars["--border"] || "rgba(255,255,255,0.06)"}` }}>
                          <div class="tp-l" style={{ background: theme.vars["--hljs-keyword"] || "#c678dd", width: "30%" }} />
                          <div class="tp-l" style={{ background: theme.vars["--hljs-string"] || "#98c379", width: "50%" }} />
                          <div class="tp-l" style={{ background: theme.vars["--hljs-function"] || "#61afef", width: "38%" }} />
                        </div>
                        <div class="tp-il">
                          <div class="tp-l" style={{ background: theme.vars["--text-secondary"] || "#aaa", width: "22%" }} />
                          <div class="tp-ic" style={{ background: `${theme.vars["--text"] || "#fff"}14` }}>
                            <div style={{ background: theme.vars["--hljs-keyword"] || "#dda0f7", height: "2px", "border-radius": "1px" }} />
                          </div>
                          <div class="tp-l" style={{ background: theme.vars["--text-secondary"] || "#aaa", width: "18%" }} />
                        </div>
                      </div>
                      <div class="tp-dots">
                        <div class="tp-d" style={{ background: theme.vars["--primary"] || "#6b7cff" }} />
                        <div class="tp-d" style={{ background: theme.vars["--green"] || "#4cd694" }} />
                        <div class="tp-d" style={{ background: theme.vars["--amber"] || "#f0b840" }} />
                        <div class="tp-d" style={{ background: theme.vars["--red"] || "#f25f67" }} />
                        <div class="tp-d" style={{ background: theme.vars["--purple"] || "#b47aff" }} />
                      </div>
                    </div>
                    <div class="tp-comp" style={{ background: theme.vars["--bg-card"] || "#151518", border: `1px solid ${theme.vars["--border"] || "rgba(255,255,255,0.06)"}` }}>
                      <div class="tp-comp-in" style={{ background: theme.vars["--text-tertiary"] || "#888", opacity: "0.12" }} />
                      <div class="tp-comp-btn" style={{ background: theme.vars["--primary"] || "#6b7cff" }} />
                    </div>
                  </div>
                  <div class="tp-glow" style={{ background: `radial-gradient(ellipse at 65% 30%, ${theme.vars["--primary-glow"] || "rgba(107,124,255,0.12)"} 0%, transparent 70%)` }} />
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
              </div>
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
    .theme-selector-desc {
      font-size: 12px;
      color: var(--text-tertiary);
      margin-top: 2px;
    }
    .theme-selector-active {
      font-size: 11px;
      font-weight: 500;
      color: var(--primary);
      margin-top: 2px;
      display: inline-block;
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
    /* Rich theme preview — mini app mockup */
    .tp {
      display: flex;
      height: 120px;
      border-radius: 6px;
      overflow: hidden;
      border: 1px solid rgba(128,128,128,0.12);
      position: relative;
    }
    .tp-side {
      width: 20%;
      display: flex;
      flex-direction: column;
      gap: 3px;
      padding: 5px 3px;
    }
    .tp-sln { height: 3px; border-radius: 1px; }
    .tp-body {
      flex: 1;
      display: flex;
      flex-direction: column;
      min-width: 0;
    }
    .tp-tabbar {
      display: flex;
      gap: 2px;
      padding: 2px 3px;
      align-items: flex-end;
      height: 10px;
    }
    .tp-tab-a { width: 20px; height: 6px; border-radius: 2px 2px 0 0; }
    .tp-tab-i { width: 14px; height: 3px; border-radius: 1px; margin-bottom: 1px; }
    .tp-msgs {
      flex: 1;
      display: flex;
      flex-direction: column;
      padding: 4px 5px;
      gap: 3px;
      overflow: hidden;
    }
    .tp-u {
      align-self: flex-end;
      padding: 2px 4px;
      border-radius: 3px;
      max-width: 55%;
    }
    .tp-a {
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    .tp-l {
      height: 2px;
      border-radius: 1px;
    }
    .tp-h {
      height: 3px;
      margin-bottom: 1px;
    }
    .tp-cb {
      padding: 3px 4px;
      border-radius: 3px;
      display: flex;
      flex-direction: column;
      gap: 2px;
      margin: 1px 0;
    }
    .tp-il {
      display: flex;
      align-items: center;
      gap: 2px;
    }
    .tp-ic {
      padding: 1px 3px;
      border-radius: 2px;
    }
    .tp-dots {
      display: flex;
      gap: 3px;
      margin-top: auto;
      padding-top: 2px;
    }
    .tp-d {
      width: 4px;
      height: 4px;
      border-radius: 50%;
    }
    .tp-comp {
      display: flex;
      gap: 3px;
      padding: 3px 4px;
      margin: 0 4px 4px;
      border-radius: 4px;
      align-items: center;
    }
    .tp-comp-in {
      flex: 1;
      height: 4px;
      border-radius: 2px;
    }
    .tp-comp-btn {
      width: 6px;
      height: 6px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .tp-glow {
      position: absolute;
      inset: 0;
      pointer-events: none;
      opacity: 0.5;
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
