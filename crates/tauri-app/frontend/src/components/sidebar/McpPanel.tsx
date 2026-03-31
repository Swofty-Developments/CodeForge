import { createSignal, For, Show, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";
import type { McpServer } from "../../ipc";

export function McpPanel() {
  const { store } = appStore;
  const [servers, setServers] = createSignal<McpServer[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [adding, setAdding] = createSignal(false);
  const [expanded, setExpanded] = createSignal(true);
  const [provider, setProvider] = createSignal(store.selectedProvider === "codex" ? "codex" : "claude_code");

  // Add form state
  const [newName, setNewName] = createSignal("");
  const [newUrl, setNewUrl] = createSignal("");
  const [newTransport, setNewTransport] = createSignal("http");
  const [newScope, setNewScope] = createSignal("user");

  async function loadServers() {
    setLoading(true);
    try {
      const p = provider() === "codex" ? "codex" : "claude_code";
      const list = await ipc.mcpListServers(p);
      setServers(list);
    } catch (e) {
      console.error("Failed to load MCP servers:", e);
      setServers([]);
    } finally {
      setLoading(false);
    }
  }

  // Load on first expand
  function handleExpand() {
    const next = !expanded();
    setExpanded(next);
    if (next && servers().length === 0) loadServers();
  }

  async function handleAdd() {
    const name = newName().trim();
    const url = newUrl().trim();
    if (!name || !url) return;

    try {
      const p = provider() === "codex" ? "codex" : "claude_code";
      await ipc.mcpAddServer(p, name, url, newTransport(), newScope(), [], []);
      setNewName("");
      setNewUrl("");
      setAdding(false);
      await loadServers();
    } catch (e) {
      console.error("Failed to add MCP server:", e);
    }
  }

  async function handleRemove(name: string) {
    try {
      const p = provider() === "codex" ? "codex" : "claude_code";
      await ipc.mcpRemoveServer(p, name, "user");
      await loadServers();
    } catch (e) {
      console.error("Failed to remove MCP server:", e);
    }
  }

  function statusDot(status: string) {
    if (status === "connected") return "var(--green)";
    if (status === "needs_auth") return "var(--amber)";
    return "var(--text-tertiary)";
  }

  return (
    <div class="mcp-panel">
      {/* Collapsible header */}
      <button class="mcp-header" onClick={handleExpand}>
        <svg
          class="mcp-chevron"
          classList={{ "mcp-chevron--open": expanded() }}
          width="10" height="10" viewBox="0 0 24 24"
          fill="none" stroke="currentColor" stroke-width="2.5"
          stroke-linecap="round" stroke-linejoin="round"
        >
          <polyline points="9 18 15 12 9 6" />
        </svg>
        <span class="mcp-title">MCP Servers</span>
        <Show when={!expanded() && servers().length > 0}>
          <span class="mcp-count">{servers().length}</span>
        </Show>
      </button>

      <Show when={expanded()}>
        {/* Provider picker */}
        <div class="mcp-provider-row">
          <button
            class="mcp-provider-btn"
            classList={{ "mcp-provider-btn--active": provider() === "claude_code" }}
            onClick={() => { setProvider("claude_code"); setTimeout(loadServers, 0); }}
          >Claude</button>
          <button
            class="mcp-provider-btn"
            classList={{ "mcp-provider-btn--active": provider() === "codex" }}
            onClick={() => { setProvider("codex"); setTimeout(loadServers, 0); }}
          >Codex</button>
        </div>

        {/* Server list */}
        <div class="mcp-list">
        <Show when={loading()}>
          <div class="mcp-loading">Loading…</div>
        </Show>
        <Show when={!loading() && servers().length === 0}>
          <div class="mcp-empty">No servers configured</div>
        </Show>
        <For each={servers()}>
          {(server) => (
            <div class="mcp-server">
              <div class="mcp-server-info">
                <div class="mcp-server-name">
                  <span class="mcp-dot" style={{ background: statusDot(server.status) }} />
                  {server.name}
                </div>
                <div class="mcp-server-url">{server.url_or_command}</div>
              </div>
              <button
                class="mcp-remove-btn"
                onClick={() => handleRemove(server.name)}
                title="Remove server"
              >
                <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                  <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
                </svg>
              </button>
            </div>
          )}
        </For>
      </div>

      {/* Add form */}
      <Show when={adding()}>
        <div class="mcp-add-form">
          <input
            class="mcp-input"
            placeholder="Server name"
            value={newName()}
            onInput={(e) => setNewName(e.currentTarget.value)}
          />
          <input
            class="mcp-input"
            placeholder="URL or command"
            value={newUrl()}
            onInput={(e) => setNewUrl(e.currentTarget.value)}
          />
          <div class="mcp-add-row">
            <div class="mcp-btn-group">
              <For each={["http", "sse", "stdio"]}>
                {(t) => (
                  <button
                    class="mcp-toggle"
                    classList={{ "mcp-toggle--active": newTransport() === t }}
                    onClick={() => setNewTransport(t)}
                  >{t.toUpperCase()}</button>
                )}
              </For>
            </div>
            <div class="mcp-btn-group">
              <For each={["user", "project", "local"]}>
                {(s) => (
                  <button
                    class="mcp-toggle"
                    classList={{ "mcp-toggle--active": newScope() === s }}
                    onClick={() => setNewScope(s)}
                  >{s.charAt(0).toUpperCase() + s.slice(1)}</button>
                )}
              </For>
            </div>
          </div>
          <div class="mcp-add-actions">
            <button class="mcp-cancel-btn" onClick={() => setAdding(false)}>Cancel</button>
            <button class="mcp-confirm-btn" onClick={handleAdd}>Add</button>
          </div>
        </div>
      </Show>

        {/* Add button */}
        <Show when={!adding()}>
          <button class="mcp-add-btn" onClick={() => setAdding(true)}>
            <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
              <line x1="12" y1="5" x2="12" y2="19" /><line x1="5" y1="12" x2="19" y2="12" />
            </svg>
            Add server
          </button>
        </Show>
      </Show>
    </div>
  );
}

if (!document.getElementById("mcp-styles")) {
  const s = document.createElement("style");
  s.id = "mcp-styles";
  s.textContent = `
    .mcp-panel {
      padding: 24px;
      max-width: 600px;
      margin: 0 auto;
      width: 100%;
    }
    .mcp-header {
      display: flex;
      align-items: center;
      gap: 6px;
      width: 100%;
      padding: 4px 0;
      margin-bottom: 2px;
      cursor: pointer;
      transition: color 0.1s;
    }
    .mcp-header:hover { color: var(--text-secondary); }
    .mcp-chevron {
      color: var(--text-tertiary);
      flex-shrink: 0;
      transition: transform 0.15s ease;
    }
    .mcp-chevron--open { transform: rotate(90deg); }
    .mcp-title {
      font-size: 10px;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.06em;
      color: var(--text-tertiary);
    }
    .mcp-count {
      font-size: 9px;
      font-weight: 600;
      color: var(--text-tertiary);
      background: var(--bg-accent);
      border-radius: var(--radius-pill);
      padding: 0 5px;
      line-height: 16px;
      margin-left: auto;
    }
    .mcp-provider-row {
      display: flex;
      gap: 2px;
      margin-bottom: 8px;
      background: var(--bg-muted);
      border-radius: var(--radius-sm);
      padding: 2px;
    }
    .mcp-provider-btn {
      flex: 1;
      font-size: 10px;
      font-weight: 500;
      padding: 3px 0;
      border-radius: 4px;
      color: var(--text-tertiary);
      transition: all 0.12s;
      text-align: center;
    }
    .mcp-provider-btn:hover { color: var(--text-secondary); }
    .mcp-provider-btn--active {
      background: var(--bg-accent);
      color: var(--text);
      box-shadow: 0 1px 3px rgba(0,0,0,0.2);
    }
    .mcp-list {
      display: flex;
      flex-direction: column;
      gap: 2px;
    }
    .mcp-loading, .mcp-empty {
      font-size: 11px;
      color: var(--text-tertiary);
      padding: 6px 0;
    }
    .mcp-server {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 5px 6px;
      border-radius: var(--radius-sm);
      transition: background 0.1s;
    }
    .mcp-server:hover {
      background: var(--bg-hover);
    }
    .mcp-server-info {
      flex: 1;
      min-width: 0;
    }
    .mcp-server-name {
      font-size: 12px;
      font-weight: 500;
      color: var(--text);
      display: flex;
      align-items: center;
      gap: 6px;
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
    }
    .mcp-dot {
      width: 5px;
      height: 5px;
      border-radius: 50%;
      flex-shrink: 0;
    }
    .mcp-server-url {
      font-size: 10px;
      color: var(--text-tertiary);
      font-family: var(--font-mono);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      padding-left: 11px;
    }
    .mcp-remove-btn {
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
    .mcp-server:hover .mcp-remove-btn { opacity: 1; }
    .mcp-remove-btn:hover {
      background: rgba(242, 95, 103, 0.15);
      color: var(--red);
    }
    .mcp-add-form {
      display: flex;
      flex-direction: column;
      gap: 5px;
      margin-top: 6px;
      padding: 8px;
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-md);
    }
    .mcp-input {
      font-size: 12px;
      padding: 5px 8px;
      background: var(--bg-base);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      font-family: var(--font-body);
      outline: none;
    }
    .mcp-input:focus {
      border-color: var(--primary);
      box-shadow: 0 0 0 2px var(--primary-glow);
    }
    .mcp-input::placeholder {
      color: var(--text-tertiary);
    }
    .mcp-add-row {
      display: flex;
      flex-direction: column;
      gap: 4px;
    }
    .mcp-btn-group {
      display: flex;
      gap: 2px;
      background: var(--bg-base);
      border-radius: var(--radius-sm);
      padding: 2px;
    }
    .mcp-toggle {
      flex: 1;
      font-size: 9px;
      font-weight: 600;
      padding: 3px 0;
      border-radius: 3px;
      color: var(--text-tertiary);
      text-align: center;
      transition: all 0.1s;
      letter-spacing: 0.03em;
    }
    .mcp-toggle:hover { color: var(--text-secondary); }
    .mcp-toggle--active {
      background: var(--bg-accent);
      color: var(--text);
    }
    .mcp-add-actions {
      display: flex;
      gap: 5px;
      justify-content: flex-end;
      margin-top: 2px;
    }
    .mcp-cancel-btn, .mcp-confirm-btn {
      font-size: 11px;
      font-weight: 500;
      padding: 4px 10px;
      border-radius: var(--radius-sm);
      transition: filter 0.12s;
    }
    .mcp-cancel-btn {
      color: var(--text-tertiary);
    }
    .mcp-cancel-btn:hover { color: var(--text-secondary); }
    .mcp-confirm-btn {
      background: var(--primary);
      color: white;
    }
    .mcp-confirm-btn:hover { filter: brightness(1.1); }
    .mcp-add-btn {
      display: flex;
      align-items: center;
      gap: 5px;
      width: 100%;
      padding: 6px;
      margin-top: 6px;
      font-size: 11px;
      font-weight: 500;
      color: var(--text-tertiary);
      border-radius: var(--radius-sm);
      transition: color 0.1s, background 0.1s;
    }
    .mcp-add-btn:hover {
      color: var(--text-secondary);
      background: var(--bg-hover);
    }
  `;
  document.head.appendChild(s);
}
