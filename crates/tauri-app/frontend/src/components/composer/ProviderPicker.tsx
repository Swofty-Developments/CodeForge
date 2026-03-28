import { createSignal, onMount, For, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { getProviderInfo, type ProviderInfo } from "../../ipc";

export function ProviderPicker() {
  const { store, setStore } = appStore;
  const [providers, setProviders] = createSignal<ProviderInfo[]>([]);
  const [loading, setLoading] = createSignal(true);

  onMount(async () => {
    try {
      const info = await getProviderInfo();
      setProviders(info);
    } catch (e) {
      console.error("Failed to get provider info:", e);
    }
    setLoading(false);
  });

  function select(id: string) {
    setStore("selectedProvider", id);
    setStore("providerPickerOpen", false);
  }

  function close() {
    setStore("providerPickerOpen", false);
  }

  return (
    <div class="overlay" onClick={close}>
      <div class="pp-panel" onClick={(e) => e.stopPropagation()}>
        <div class="pp-header">
          <h3>Select Model</h3>
          <button class="pp-close" onClick={close}>&times;</button>
        </div>

        <Show when={!loading()} fallback={<div class="pp-loading">Checking installations...</div>}>
          <div class="pp-list">
            <For each={providers()}>
              {(provider) => {
                const isSelected = () => store.selectedProvider === provider.id;
                return (
                  <div
                    class="pp-card"
                    classList={{ selected: isSelected(), unavailable: !provider.installed }}
                    onClick={() => provider.installed && select(provider.id)}
                  >
                    <div class="pp-card-header">
                      <div class="pp-card-title-row">
                        <Show when={isSelected()}>
                          <span class="pp-check">&#x2713;</span>
                        </Show>
                        <span class="pp-name">{provider.name}</span>
                        <span
                          class="pp-status"
                          classList={{ installed: provider.installed }}
                        >
                          {provider.installed ? "Installed" : "Not installed"}
                        </span>
                      </div>
                      <p class="pp-desc">{provider.description}</p>
                    </div>

                    <Show when={provider.installed}>
                      <div class="pp-details">
                        <Show when={provider.version}>
                          <div class="pp-detail-row">
                            <span class="pp-label">Version</span>
                            <span class="pp-value">{provider.version}</span>
                          </div>
                        </Show>
                        <div class="pp-detail-row">
                          <span class="pp-label">Path</span>
                          <span class="pp-value mono">{provider.path}</span>
                        </div>
                      </div>
                    </Show>

                    <Show when={!provider.installed}>
                      <div class="pp-install">
                        <p class="pp-install-label">Install with:</p>
                        <code class="pp-install-cmd">{provider.install_instructions}</code>
                        <a
                          class="pp-link"
                          href={provider.website}
                          target="_blank"
                          onClick={(e) => e.stopPropagation()}
                        >
                          Learn more &rarr;
                        </a>
                      </div>
                    </Show>
                  </div>
                );
              }}
            </For>
          </div>
        </Show>
      </div>

      <style>{`
        .pp-panel {
          background: var(--bg-card);
          border: 1px solid var(--border-strong);
          border-radius: var(--radius-lg);
          width: 90%;
          max-width: 480px;
          max-height: 80vh;
          overflow-y: auto;
        }
        .pp-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          padding: 20px 24px 12px;
        }
        .pp-header h3 {
          font-size: 16px;
          font-weight: 600;
          color: var(--text);
        }
        .pp-close {
          font-size: 20px;
          color: var(--text-tertiary);
          padding: 4px 8px;
          border-radius: var(--radius-sm);
          transition: background 0.12s;
        }
        .pp-close:hover { background: var(--bg-accent); }
        .pp-loading {
          padding: 32px 24px;
          text-align: center;
          color: var(--text-tertiary);
          font-size: 13px;
        }
        .pp-list {
          padding: 8px 16px 20px;
          display: flex;
          flex-direction: column;
          gap: 10px;
        }
        .pp-card {
          background: var(--bg-surface);
          border: 1px solid var(--border);
          border-radius: var(--radius-md);
          padding: 14px 16px;
          cursor: pointer;
          transition: border-color 0.15s, background 0.15s;
        }
        .pp-card:hover { border-color: var(--border-strong); background: var(--bg-muted); }
        .pp-card.selected {
          border-color: var(--primary);
          background: rgba(102, 128, 242, 0.06);
        }
        .pp-card.unavailable { opacity: 0.7; cursor: default; }
        .pp-card.unavailable:hover { border-color: var(--border); background: var(--bg-surface); }
        .pp-card-header { margin-bottom: 8px; }
        .pp-card-title-row {
          display: flex;
          align-items: center;
          gap: 8px;
          margin-bottom: 4px;
        }
        .pp-check {
          color: var(--primary);
          font-size: 13px;
          font-weight: 600;
        }
        .pp-name {
          font-size: 14px;
          font-weight: 500;
          color: var(--text);
        }
        .pp-status {
          margin-left: auto;
          font-size: 10px;
          padding: 2px 8px;
          border-radius: var(--radius-pill);
          background: rgba(230, 89, 97, 0.12);
          color: var(--red);
          font-weight: 500;
        }
        .pp-status.installed {
          background: rgba(89, 199, 140, 0.12);
          color: var(--green);
        }
        .pp-desc {
          font-size: 12px;
          color: var(--text-secondary);
          line-height: 1.4;
        }
        .pp-details {
          border-top: 1px solid var(--border);
          padding-top: 8px;
          display: flex;
          flex-direction: column;
          gap: 4px;
        }
        .pp-detail-row {
          display: flex;
          justify-content: space-between;
          align-items: center;
        }
        .pp-label {
          font-size: 11px;
          color: var(--text-tertiary);
          text-transform: uppercase;
          letter-spacing: 0.04em;
        }
        .pp-value {
          font-size: 12px;
          color: var(--text-secondary);
        }
        .pp-value.mono {
          font-family: "SF Mono", "Fira Code", "Consolas", monospace;
          font-size: 11px;
          color: var(--text-tertiary);
        }
        .pp-install {
          border-top: 1px solid var(--border);
          padding-top: 10px;
          display: flex;
          flex-direction: column;
          gap: 6px;
        }
        .pp-install-label {
          font-size: 11px;
          color: var(--text-tertiary);
        }
        .pp-install-cmd {
          display: block;
          background: var(--bg-base);
          border: 1px solid var(--border);
          border-radius: var(--radius-sm);
          padding: 8px 12px;
          font-family: "SF Mono", "Fira Code", "Consolas", monospace;
          font-size: 12px;
          color: var(--text);
          user-select: text;
          -webkit-user-select: text;
        }
        .pp-link {
          font-size: 12px;
          color: var(--primary);
          text-decoration: none;
          transition: filter 0.12s;
        }
        .pp-link:hover { filter: brightness(1.2); }
      `}</style>
    </div>
  );
}
