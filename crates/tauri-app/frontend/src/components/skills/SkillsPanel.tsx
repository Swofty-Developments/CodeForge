import { createSignal, For, Show, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { SkillInfo, MarketplaceSource } from "../../ipc";

export function SkillsPanel() {
  const { store } = appStore;
  const [provider, setProvider] = createSignal(store.selectedProvider === "codex" ? "codex" : "claude_code");
  const [skills, setSkills] = createSignal<SkillInfo[]>([]);
  const [marketplaces, setMarketplaces] = createSignal<MarketplaceSource[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  // Install form
  const [installName, setInstallName] = createSignal("");
  const [installing, setInstalling] = createSignal(false);

  // Marketplace form
  const [newMarketplace, setNewMarketplace] = createSignal("");
  const [addingMarketplace, setAddingMarketplace] = createSignal(false);

  // Search
  const [search, setSearch] = createSignal("");

  const providerArg = () => provider() === "codex" ? "codex" : "claude_code";

  async function loadAll() {
    setLoading(true);
    setError(null);
    try {
      const [s, m] = await Promise.all([
        ipc.listSkills(providerArg()),
        ipc.listMarketplaces(providerArg()),
      ]);
      setSkills(s);
      setMarketplaces(m);
    } catch (e: any) {
      console.error("Failed to load skills:", e);
      setError(String(e));
      setSkills([]);
      setMarketplaces([]);
    } finally {
      setLoading(false);
    }
  }

  onMount(() => loadAll());

  function switchProvider(p: string) {
    setProvider(p);
    setTimeout(loadAll, 0);
  }

  async function handleInstall() {
    const name = installName().trim();
    if (!name) return;
    setInstalling(true);
    try {
      await ipc.installSkill(providerArg(), name);
      setInstallName("");
      await loadAll();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setInstalling(false);
    }
  }

  async function handleUninstall(name: string) {
    try {
      await ipc.uninstallSkill(providerArg(), name);
      await loadAll();
    } catch (e: any) {
      setError(String(e));
    }
  }

  async function handleToggle(name: string, currentlyEnabled: boolean) {
    try {
      if (currentlyEnabled) {
        await ipc.disableSkill(providerArg(), name);
      } else {
        await ipc.enableSkill(providerArg(), name);
      }
      await loadAll();
    } catch (e: any) {
      setError(String(e));
    }
  }

  async function handleAddMarketplace() {
    const source = newMarketplace().trim();
    if (!source) return;
    setAddingMarketplace(true);
    try {
      await ipc.addMarketplace(providerArg(), source);
      setNewMarketplace("");
      await loadAll();
    } catch (e: any) {
      setError(String(e));
    } finally {
      setAddingMarketplace(false);
    }
  }

  const filteredSkills = () => {
    const q = search().toLowerCase();
    if (!q) return skills();
    return skills().filter(
      (s) => s.name.toLowerCase().includes(q) || s.source.toLowerCase().includes(q)
    );
  };

  return (
    <div class="sk-panel">
      <div class="sk-header">
        <h2 class="sk-heading">Skills & Plugins</h2>
      </div>

      {/* Provider toggle */}
      <div class="sk-provider-row">
        <button
          class="sk-provider-btn"
          classList={{ "sk-provider-btn--active": provider() === "claude_code" }}
          onClick={() => switchProvider("claude_code")}
        >Claude</button>
        <button
          class="sk-provider-btn"
          classList={{ "sk-provider-btn--active": provider() === "codex" }}
          onClick={() => switchProvider("codex")}
        >Codex</button>
      </div>

      <Show when={error()}>
        <div class="sk-error">{error()}</div>
      </Show>

      <Show when={loading()}>
        <div class="sk-loading">Loading...</div>
      </Show>

      <Show when={!loading()}>
        {/* Empty state */}
        <Show when={skills().length === 0 && !error()}>
          <div class="sk-empty">
            <svg class="sk-empty-icon" width="36" height="36" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round">
              <path d="M14.7 6.3a1 1 0 000 1.4l1.6 1.6a1 1 0 001.4 0l3.77-3.77a6 6 0 01-7.94 7.94l-6.91 6.91a2.12 2.12 0 01-3-3l6.91-6.91a6 6 0 017.94-7.94l-3.76 3.76z" />
            </svg>
            <span class="sk-empty-title">No Skills Installed</span>
            <p class="sk-empty-desc">
              Skills extend Claude's capabilities with custom slash commands and behaviors.
            </p>
            <p class="sk-empty-desc">
              Install from a marketplace or browse available skills below.
            </p>
          </div>
        </Show>

        {/* Installed section */}
        <Show when={skills().length > 0}>
          <div class="sk-section">
            <div class="sk-section-header">
              <span class="sk-section-title">Installed</span>
              <span class="sk-count">{skills().length}</span>
            </div>

            {/* Search */}
            <input
              class="sk-input"
              placeholder="Filter installed skills..."
              value={search()}
              onInput={(e) => setSearch(e.currentTarget.value)}
            />

            <div class="sk-list">
              <For each={filteredSkills()}>
                {(skill) => (
                  <div class="sk-item">
                    <div class="sk-item-info">
                      <span class="sk-item-name">{skill.name}</span>
                      <Show when={skill.source}>
                        <span class="sk-item-source">@{skill.source}</span>
                      </Show>
                      <Show when={skill.version && skill.version !== "unknown"}>
                        <span class="sk-item-version">v{skill.version}</span>
                      </Show>
                    </div>
                    <div class="sk-item-actions">
                      <button
                        class="sk-toggle"
                        classList={{
                          "sk-toggle--on": skill.enabled,
                          "sk-toggle--off": !skill.enabled,
                        }}
                        onClick={() => handleToggle(skill.name, skill.enabled)}
                        title={skill.enabled ? "Disable" : "Enable"}
                      >
                        <span class="sk-toggle-thumb" />
                      </button>
                      <button
                        class="sk-uninstall-btn"
                        onClick={() => handleUninstall(skill.name)}
                        title="Uninstall"
                      >
                        <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                          <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                        </svg>
                      </button>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </div>
        </Show>

        {/* Install section */}
        <div class="sk-section">
          <span class="sk-section-title">Install Plugin</span>
          <div class="sk-install-row">
            <input
              class="sk-input sk-input--grow"
              placeholder="Plugin name (e.g. my-skill)"
              value={installName()}
              onInput={(e) => setInstallName(e.currentTarget.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleInstall(); }}
            />
            <button
              class="sk-btn sk-btn--primary"
              onClick={handleInstall}
              disabled={installing() || !installName().trim()}
            >
              {installing() ? "Installing..." : "Install"}
            </button>
          </div>
        </div>

        {/* Marketplaces section */}
        <div class="sk-section">
          <span class="sk-section-title">Marketplaces</span>
          <Show when={marketplaces().length === 0}>
            <p class="sk-hint">No marketplace sources configured.</p>
          </Show>
          <div class="sk-list">
            <For each={marketplaces()}>
              {(mp) => (
                <div class="sk-mp-item">
                  <span class="sk-mp-name">{mp.name}</span>
                  <Show when={mp.source}>
                    <span class="sk-mp-url">{mp.source}</span>
                  </Show>
                </div>
              )}
            </For>
          </div>
          <div class="sk-install-row">
            <input
              class="sk-input sk-input--grow"
              placeholder="Marketplace URL"
              value={newMarketplace()}
              onInput={(e) => setNewMarketplace(e.currentTarget.value)}
              onKeyDown={(e) => { if (e.key === "Enter") handleAddMarketplace(); }}
            />
            <button
              class="sk-btn sk-btn--primary"
              onClick={handleAddMarketplace}
              disabled={addingMarketplace() || !newMarketplace().trim()}
            >
              {addingMarketplace() ? "Adding..." : "Add"}
            </button>
          </div>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("skills-panel-styles")) {
  const s = document.createElement("style");
  s.id = "skills-panel-styles";
  s.textContent = `
    .sk-panel {
      padding: 24px;
      max-width: 600px;
      margin: 0 auto;
      width: 100%;
    }
    .sk-header {
      margin-bottom: 16px;
    }
    .sk-heading {
      font-size: 16px;
      font-weight: 700;
      color: var(--text);
      margin: 0;
    }
    .sk-provider-row {
      display: flex;
      gap: 2px;
      margin-bottom: 16px;
      background: var(--bg-muted);
      border-radius: var(--radius-sm);
      padding: 2px;
    }
    .sk-provider-btn {
      flex: 1;
      font-size: 10px;
      font-weight: 500;
      padding: 3px 0;
      border-radius: 4px;
      color: var(--text-tertiary);
      transition: all 0.12s;
      text-align: center;
    }
    .sk-provider-btn:hover { color: var(--text-secondary); }
    .sk-provider-btn--active {
      background: var(--bg-accent);
      color: var(--text);
      box-shadow: 0 1px 3px rgba(0,0,0,0.2);
    }
    .sk-error {
      font-size: 11px;
      color: var(--red);
      background: rgba(242, 95, 103, 0.08);
      border: 1px solid rgba(242, 95, 103, 0.2);
      border-radius: var(--radius-sm);
      padding: 8px 10px;
      margin-bottom: 12px;
    }
    .sk-loading {
      font-size: 11px;
      color: var(--text-tertiary);
      padding: 12px 0;
    }
    .sk-empty {
      display: flex;
      flex-direction: column;
      align-items: center;
      text-align: center;
      padding: 32px 12px;
      gap: 8px;
    }
    .sk-empty-icon {
      color: var(--text-tertiary);
      opacity: 0.5;
      margin-bottom: 4px;
    }
    .sk-empty-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--text-secondary);
    }
    .sk-empty-desc {
      font-size: 11px;
      color: var(--text-tertiary);
      line-height: 1.5;
      max-width: 360px;
      margin: 0;
    }
    .sk-section {
      margin-bottom: 20px;
    }
    .sk-section-header {
      display: flex;
      align-items: center;
      gap: 6px;
      margin-bottom: 8px;
    }
    .sk-section-title {
      font-size: 10px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--text-tertiary);
      display: block;
      margin-bottom: 8px;
    }
    .sk-section-header .sk-section-title {
      margin-bottom: 0;
    }
    .sk-count {
      font-size: 9px;
      font-weight: 600;
      color: var(--text-tertiary);
      background: var(--bg-accent);
      border-radius: var(--radius-pill);
      padding: 0 5px;
      line-height: 16px;
    }
    .sk-list {
      display: flex;
      flex-direction: column;
      gap: 2px;
      margin-bottom: 8px;
    }
    .sk-item {
      display: flex;
      align-items: center;
      gap: 8px;
      padding: 6px 8px;
      border-radius: var(--radius-sm);
      transition: background 0.1s;
    }
    .sk-item:hover {
      background: var(--bg-hover);
    }
    .sk-item-info {
      flex: 1;
      min-width: 0;
      display: flex;
      align-items: baseline;
      gap: 4px;
    }
    .sk-item-name {
      font-size: 12px;
      font-weight: 500;
      color: var(--text);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .sk-item-source {
      font-size: 10px;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .sk-item-actions {
      display: flex;
      align-items: center;
      gap: 6px;
      flex-shrink: 0;
    }
    .sk-toggle {
      position: relative;
      width: 28px;
      height: 16px;
      border-radius: 8px;
      cursor: pointer;
      transition: background 0.2s;
      flex-shrink: 0;
    }
    .sk-toggle--on {
      background: var(--primary);
    }
    .sk-toggle--off {
      background: var(--bg-accent);
      border: 1px solid var(--border);
    }
    .sk-toggle-thumb {
      position: absolute;
      top: 2px;
      left: 2px;
      width: 12px;
      height: 12px;
      border-radius: 50%;
      background: #fff;
      transition: transform 0.2s;
      box-shadow: 0 1px 2px rgba(0,0,0,0.2);
    }
    .sk-toggle--on .sk-toggle-thumb {
      transform: translateX(12px);
    }
    .sk-toggle--off .sk-toggle-thumb {
      transform: translateX(0);
    }
    .sk-uninstall-btn {
      width: 18px;
      height: 18px;
      display: flex;
      align-items: center;
      justify-content: center;
      border-radius: 3px;
      color: var(--text-tertiary);
      opacity: 0;
      transition: opacity 0.1s, background 0.1s, color 0.1s;
      flex-shrink: 0;
    }
    .sk-item:hover .sk-uninstall-btn { opacity: 1; }
    .sk-uninstall-btn:hover {
      background: rgba(242, 95, 103, 0.15);
      color: var(--red);
    }
    .sk-input {
      font-size: 12px;
      padding: 6px 8px;
      background: var(--bg-base);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      font-family: var(--font-body);
      outline: none;
      width: 100%;
      box-sizing: border-box;
    }
    .sk-input:focus {
      border-color: var(--primary);
      box-shadow: 0 0 0 2px var(--primary-glow);
    }
    .sk-input::placeholder {
      color: var(--text-tertiary);
    }
    .sk-input--grow {
      flex: 1;
      min-width: 0;
    }
    .sk-install-row {
      display: flex;
      gap: 6px;
      align-items: center;
    }
    .sk-btn {
      font-size: 11px;
      font-weight: 500;
      padding: 6px 14px;
      border-radius: var(--radius-sm);
      transition: filter 0.12s, opacity 0.12s;
      white-space: nowrap;
      flex-shrink: 0;
    }
    .sk-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }
    .sk-btn--primary {
      background: var(--primary);
      color: white;
    }
    .sk-btn--primary:hover:not(:disabled) { filter: brightness(1.1); }
    .sk-hint {
      font-size: 11px;
      color: var(--text-tertiary);
      margin: 0 0 8px;
    }
    .sk-mp-item {
      display: flex;
      align-items: center;
      padding: 5px 8px;
      border-radius: var(--radius-sm);
      transition: background 0.1s;
    }
    .sk-mp-item:hover {
      background: var(--bg-hover);
    }
    .sk-mp-url {
      font-size: 11px;
      color: var(--text-secondary);
      font-family: var(--font-mono);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      flex: 1;
    }
  `;
  document.head.appendChild(s);
}
