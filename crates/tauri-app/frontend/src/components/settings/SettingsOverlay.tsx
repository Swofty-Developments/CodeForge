import { createSignal, onMount, For } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

const PERMISSION_MODES = [
  { value: "auto", label: "Auto", description: "AI decides when to ask (recommended)" },
  { value: "acceptEdits", label: "Accept Edits", description: "Auto-approve file operations" },
  { value: "bypassPermissions", label: "Bypass All", description: "Auto-approve everything" },
  { value: "plan", label: "Plan Mode", description: "Only plans, doesn't execute" },
  { value: "default", label: "Default", description: "Ask for everything (requires TTY, may hang)" },
] as const;

export function SettingsOverlay() {
  const { store, setStore } = appStore;
  const [claudePath, setClaudePath] = createSignal("claude");
  const [codexPath, setCodexPath] = createSignal("codex");
  const [permissionMode, setPermissionMode] = createSignal("bypassPermissions");

  onMount(async () => {
    const cp = await ipc.getSetting("claude_path");
    const cx = await ipc.getSetting("codex_path");
    const pm = await ipc.getSetting("permission_mode");
    if (cp) setClaudePath(cp);
    if (cx) setCodexPath(cx);
    if (pm) setPermissionMode(pm);
  });

  function close() {
    setStore("settingsOpen", false);
  }

  return (
    <div class="overlay" onClick={close}>
      <div class="overlay-panel" onClick={(e) => e.stopPropagation()}>
        <div class="settings-header">
          <h3>Settings</h3>
          <button class="close-btn" onClick={close}>
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round">
              <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
            </svg>
          </button>
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

        <div class="settings-section">
          <label>Permission Mode</label>
          <div class="settings-perm-list">
            <For each={PERMISSION_MODES}>
              {(mode) => (
                <button
                  class="settings-perm-option"
                  classList={{ "settings-perm-option--active": permissionMode() === mode.value }}
                  onClick={() => {
                    setPermissionMode(mode.value);
                    ipc.setSetting("permission_mode", mode.value);
                  }}
                >
                  <span class="settings-perm-label">{mode.label}</span>
                  <span class="settings-perm-desc">{mode.description}</span>
                </button>
              )}
            </For>
          </div>
          <span class="settings-hint">Takes effect on next session.</span>
        </div>

        <div class="settings-section">
          <label>Auto-name Threads</label>
          <div class="settings-toggle-row">
            <span class="settings-toggle-desc">Automatically generate thread names after 3 messages</span>
            <button
              class="settings-toggle"
              classList={{ on: store.autoNamingEnabled }}
              onClick={() => setStore("autoNamingEnabled", !store.autoNamingEnabled)}
            >
              <span class="settings-toggle-knob" />
            </button>
          </div>
        </div>

        <div class="settings-section">
          <label>Desktop Notifications</label>
          <div class="settings-toggle-row">
            <span class="settings-toggle-desc">Notify when background threads complete or encounter errors</span>
            <button
              class="settings-toggle"
              classList={{ on: store.notificationsEnabled }}
              onClick={() => setStore("notificationsEnabled", !store.notificationsEnabled)}
            >
              <span class="settings-toggle-knob" />
            </button>
          </div>
        </div>
      </div>

      <style>{`
        .settings-header {
          display: flex;
          justify-content: space-between;
          align-items: center;
          margin-bottom: 20px;
        }
        .settings-header h3 {
          font-size: 16px;
          font-weight: 600;
          letter-spacing: -0.3px;
        }
        .close-btn {
          color: var(--text-tertiary);
          padding: 5px;
          border-radius: var(--radius-sm);
          transition: background 0.12s, color 0.12s;
          display: flex;
          align-items: center;
        }
        .close-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }
        .settings-section {
          margin-bottom: 16px;
        }
        .settings-section label {
          display: block;
          font-size: 10px;
          font-weight: 600;
          color: var(--text-tertiary);
          margin-bottom: 6px;
          text-transform: uppercase;
          letter-spacing: 0.06em;
        }
        .settings-section input {
          width: 100%;
          font-family: var(--font-mono);
          font-size: 12px;
        }
        .settings-perm-list {
          display: flex;
          flex-direction: column;
          gap: 3px;
          background: var(--bg-muted);
          border-radius: var(--radius-md);
          padding: 3px;
        }
        .settings-perm-option {
          display: flex;
          flex-direction: column;
          gap: 1px;
          padding: 7px 10px;
          border-radius: var(--radius-sm);
          text-align: left;
          transition: background 0.1s;
        }
        .settings-perm-option:hover { background: var(--bg-hover); }
        .settings-perm-option--active {
          background: var(--bg-accent);
          box-shadow: 0 1px 3px rgba(0,0,0,0.15);
        }
        .settings-perm-label {
          font-size: 12px;
          font-weight: 600;
          color: var(--text);
        }
        .settings-perm-option--active .settings-perm-label { color: var(--primary); }
        .settings-perm-desc {
          font-size: 10px;
          color: var(--text-tertiary);
        }
        .settings-hint {
          display: block;
          font-size: 10px;
          color: var(--text-tertiary);
          margin-top: 4px;
        }
        .settings-toggle-row {
          display: flex;
          align-items: center;
          justify-content: space-between;
          gap: 12px;
        }
        .settings-toggle-desc {
          font-size: 12px;
          color: var(--text-secondary);
        }
        .settings-toggle {
          width: 36px;
          height: 20px;
          border-radius: 10px;
          background: var(--bg-accent);
          border: 1px solid var(--border-strong);
          position: relative;
          transition: background 0.2s, border-color 0.2s;
          flex-shrink: 0;
          cursor: pointer;
        }
        .settings-toggle.on {
          background: var(--primary);
          border-color: var(--primary);
        }
        .settings-toggle-knob {
          position: absolute;
          top: 2px;
          left: 2px;
          width: 14px;
          height: 14px;
          border-radius: 50%;
          background: var(--text);
          transition: transform 0.2s;
        }
        .settings-toggle.on .settings-toggle-knob {
          transform: translateX(16px);
        }
      `}</style>
    </div>
  );
}
