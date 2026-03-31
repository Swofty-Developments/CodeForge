import { createSignal, Show, onMount } from "solid-js";

export function UpdateChecker() {
  const [updateAvailable, setUpdateAvailable] = createSignal(false);
  const [updateVersion, setUpdateVersion] = createSignal("");
  const [updateNotes, setUpdateNotes] = createSignal("");
  const [downloading, setDownloading] = createSignal(false);
  const [progress, setProgress] = createSignal(0);
  const [dismissed, setDismissed] = createSignal(false);

  onMount(async () => {
    // Delay check so the app loads first
    setTimeout(() => checkForUpdates(), 3000);
  });

  async function checkForUpdates() {
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const update = await check();
      if (update) {
        setUpdateAvailable(true);
        setUpdateVersion(update.version || "");
        setUpdateNotes(update.body || "");
      }
    } catch {
      // Updater not available (dev mode, or no internet)
    }
  }

  async function installUpdate() {
    setDownloading(true);
    setProgress(0);
    try {
      const { check } = await import("@tauri-apps/plugin-updater");
      const { relaunch } = await import("@tauri-apps/plugin-process");

      const update = await check();
      if (!update) return;

      let totalBytes = 0;
      let downloadedBytes = 0;

      await update.downloadAndInstall((event) => {
        if (event.event === "Started" && event.data.contentLength) {
          totalBytes = event.data.contentLength;
        } else if (event.event === "Progress") {
          downloadedBytes += event.data.chunkLength;
          if (totalBytes > 0) {
            setProgress(Math.round((downloadedBytes / totalBytes) * 100));
          }
        } else if (event.event === "Finished") {
          setProgress(100);
        }
      });

      await relaunch();
    } catch (e) {
      console.error("Update failed:", e);
      setDownloading(false);
    }
  }

  return (
    <Show when={updateAvailable() && !dismissed()}>
      <div class="uc-banner">
        <div class="uc-content">
          <div class="uc-icon">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
              <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4" />
              <polyline points="7 10 12 15 17 10" />
              <line x1="12" y1="15" x2="12" y2="3" />
            </svg>
          </div>
          <div class="uc-text">
            <span class="uc-title">
              Update available
              <Show when={updateVersion()}>
                <span class="uc-version">v{updateVersion()}</span>
              </Show>
            </span>
            <Show when={updateNotes() && !downloading()}>
              <span class="uc-notes">{updateNotes().slice(0, 100)}</span>
            </Show>
          </div>
          <Show when={downloading()}>
            <div class="uc-progress">
              <div class="uc-progress-track">
                <div class="uc-progress-fill" style={{ width: `${progress()}%` }} />
              </div>
              <span class="uc-progress-label">{progress()}%</span>
            </div>
          </Show>
          <Show when={!downloading()}>
            <button class="uc-install" onClick={installUpdate}>
              Update now
            </button>
            <button class="uc-dismiss" onClick={() => setDismissed(true)} title="Dismiss">
              <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
                <line x1="18" y1="6" x2="6" y2="18" /><line x1="6" y1="6" x2="18" y2="18" />
              </svg>
            </button>
          </Show>
        </div>
      </div>
    </Show>
  );
}

if (!document.getElementById("update-checker-styles")) {
  const s = document.createElement("style");
  s.id = "update-checker-styles";
  s.textContent = `
    .uc-banner {
      position: fixed;
      bottom: 16px;
      right: 16px;
      z-index: 150;
      animation: fade-slide-up 0.3s ease both;
    }
    .uc-content {
      display: flex;
      align-items: center;
      gap: 10px;
      padding: 10px 14px;
      background: var(--bg-card);
      border: 1px solid rgba(76, 214, 148, 0.25);
      border-radius: var(--radius-md);
      box-shadow: 0 8px 32px rgba(0, 0, 0, 0.4), 0 0 0 1px rgba(76, 214, 148, 0.08);
    }
    .uc-icon {
      color: var(--green);
      flex-shrink: 0;
    }
    .uc-text {
      display: flex;
      flex-direction: column;
      gap: 2px;
      min-width: 0;
    }
    .uc-title {
      font-size: 13px;
      font-weight: 600;
      color: var(--text);
      display: flex;
      align-items: center;
      gap: 6px;
    }
    .uc-version {
      font-size: 11px;
      font-family: var(--font-mono);
      color: var(--green);
      background: rgba(76, 214, 148, 0.1);
      padding: 1px 5px;
      border-radius: var(--radius-pill);
    }
    .uc-notes {
      font-size: 11px;
      color: var(--text-tertiary);
      overflow: hidden;
      text-overflow: ellipsis;
      white-space: nowrap;
      max-width: 200px;
    }
    .uc-install {
      padding: 6px 14px;
      font-size: 12px;
      font-weight: 600;
      background: var(--green);
      color: #fff;
      border-radius: var(--radius-sm);
      transition: all 0.12s;
      white-space: nowrap;
      flex-shrink: 0;
    }
    .uc-install:hover {
      filter: brightness(1.1);
      transform: translateY(-1px);
    }
    .uc-dismiss {
      color: var(--text-tertiary);
      padding: 4px;
      border-radius: var(--radius-sm);
      transition: all 0.1s;
      flex-shrink: 0;
    }
    .uc-dismiss:hover {
      color: var(--text-secondary);
      background: var(--bg-hover);
    }
    .uc-progress {
      display: flex;
      align-items: center;
      gap: 8px;
      flex-shrink: 0;
    }
    .uc-progress-track {
      width: 80px;
      height: 4px;
      background: var(--bg-accent);
      border-radius: 2px;
      overflow: hidden;
    }
    .uc-progress-fill {
      height: 100%;
      background: var(--green);
      border-radius: 2px;
      transition: width 0.2s ease;
    }
    .uc-progress-label {
      font-size: 10px;
      font-family: var(--font-mono);
      color: var(--text-tertiary);
      min-width: 28px;
    }
  `;
  document.head.appendChild(s);
}
