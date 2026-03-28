import { createSignal, onMount } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

export function SettingsOverlay() {
  const { setStore } = appStore;
  const [claudePath, setClaudePath] = createSignal("claude");
  const [codexPath, setCodexPath] = createSignal("codex");

  onMount(async () => {
    const cp = await ipc.getSetting("claude_path");
    const cx = await ipc.getSetting("codex_path");
    if (cp) setClaudePath(cp);
    if (cx) setCodexPath(cx);
  });

  function close() {
    setStore("settingsOpen", false);
  }

  return (
    <div class="overlay" onClick={close}>
      <div class="overlay-panel" onClick={(e) => e.stopPropagation()}>
        <div class="settings-header">
          <h3>Settings</h3>
          <button class="close-btn" onClick={close}>&times;</button>
        </div>

        <div class="settings-section">
          <label>Claude Binary</label>
          <input
            value={claudePath()}
            onInput={(e) => setClaudePath(e.currentTarget.value)}
            onBlur={() => ipc.setSetting("claude_path", claudePath())}
          />
        </div>

        <div class="settings-section">
          <label>Codex Binary</label>
          <input
            value={codexPath()}
            onInput={(e) => setCodexPath(e.currentTarget.value)}
            onBlur={() => ipc.setSetting("codex_path", codexPath())}
          />
        </div>
      </div>

      <style>{`
        .settings-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }
        .settings-header h3 { font-size: 18px; font-weight: 500; }
        .close-btn {
          font-size: 20px;
          color: var(--text-secondary);
          padding: 4px 8px;
          border-radius: var(--radius-sm);
          transition: background 0.12s;
        }
        .close-btn:hover { background: var(--bg-accent); }
        .settings-section {
          margin-bottom: 16px;
        }
        .settings-section label {
          display: block;
          font-size: 11px;
          color: var(--text-tertiary);
          margin-bottom: 4px;
          text-transform: uppercase;
          letter-spacing: 0.05em;
        }
        .settings-section input {
          width: 100%;
        }
      `}</style>
    </div>
  );
}
