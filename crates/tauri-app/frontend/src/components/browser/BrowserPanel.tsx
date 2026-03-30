import { createSignal, onMount, onCleanup, createEffect } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

interface BrowserPanelProps {
  threadId: string;
}

export function BrowserPanel(props: BrowserPanelProps) {
  const { store, setStore } = appStore;
  const currentUrl = () => store.threadBrowserUrls[props.threadId] || "https://example.com";
  const [urlInput, setUrlInput] = createSignal(currentUrl());
  const [started, setStarted] = createSignal(false);
  let viewportRef: HTMLDivElement | undefined;

  // Sync URL input when thread changes
  createEffect(() => {
    setUrlInput(currentUrl());
  });

  // Position the native webview to match our viewport div
  function updateBounds() {
    if (!viewportRef || !started()) return;
    const rect = viewportRef.getBoundingClientRect();
    if (rect.width < 10 || rect.height < 10) return;
    ipc.browserSetBounds(
      props.threadId,
      Math.round(rect.left),
      Math.round(rect.top),
      Math.round(rect.width),
      Math.round(rect.height),
    ).catch(() => {});
  }

  // Keep bounds in sync — ResizeObserver + polling fallback
  onMount(() => {
    const ro = new ResizeObserver(() => updateBounds());
    if (viewportRef) ro.observe(viewportRef);

    window.addEventListener("resize", updateBounds);

    // Poll every 200ms as fallback for layout changes (sidebar resize, pane drag, etc.)
    const interval = setInterval(updateBounds, 200);

    onCleanup(() => {
      ro.disconnect();
      window.removeEventListener("resize", updateBounds);
      clearInterval(interval);
      // Hide webview when panel unmounts (thread switch)
      ipc.browserHide(props.threadId).catch(() => {});
    });
  });

  // Open the native webview on first mount
  onMount(() => {
    requestAnimationFrame(() => {
      if (!viewportRef) return;
      const rect = viewportRef.getBoundingClientRect();
      const url = currentUrl();
      ipc.browserOpen(
        props.threadId,
        url,
        Math.round(rect.left),
        Math.round(rect.top),
        Math.round(rect.width),
        Math.round(rect.height),
      ).then(() => {
        setStarted(true);
      }).catch((e) => {
        console.error("Failed to open browser:", e);
      });
    });
  });

  // When started, keep bounds in sync
  createEffect(() => {
    if (started()) {
      updateBounds();
    }
  });

  function navigate() {
    let url = urlInput().trim();
    if (!url) return;
    if (!url.match(/^https?:\/\//)) url = "https://" + url;
    setStore("threadBrowserUrls", props.threadId, url);
    setUrlInput(url);
    ipc.browserNavigate(props.threadId, url).catch((e) => {
      console.error("Navigate failed:", e);
    });
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
    ipc.browserHide(props.threadId).catch(() => {});
  }

  return (
    <div class="bp">
      <div class="bp-bar">
        <input
          class="bp-url"
          type="text"
          value={urlInput()}
          onInput={(e) => setUrlInput(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") navigate(); }}
          placeholder="Enter URL..."
        />
        <button class="bp-go" onClick={navigate}>Go</button>
        <button class="bp-nav" onClick={close} title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round">
            <line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/>
          </svg>
        </button>
      </div>
      {/* This div reserves space — the native webview is positioned over it */}
      <div ref={viewportRef} class="bp-viewport" />
    </div>
  );
}

if (!document.getElementById("browser-panel-styles")) {
  const s = document.createElement("style");
  s.id = "browser-panel-styles";
  s.textContent = `
    .bp {
      flex: 1;
      display: flex;
      flex-direction: column;
      background: var(--bg-card);
      min-height: 0;
      overflow: hidden;
    }
    .bp-bar {
      display: flex;
      align-items: center;
      gap: 4px;
      padding: 6px 8px;
      border-bottom: 1px solid var(--border);
      background: var(--bg-surface);
      flex-shrink: 0;
    }
    .bp-nav {
      width: 24px;
      height: 24px;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      transition: background 0.1s, color 0.1s;
      flex-shrink: 0;
    }
    .bp-nav:hover { background: var(--bg-accent); color: var(--text-secondary); }
    .bp-url {
      flex: 1;
      min-width: 0;
      height: 26px;
      padding: 0 8px;
      font-size: 12px;
      font-family: var(--font-mono);
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text);
      outline: none;
    }
    .bp-url:focus { border-color: var(--primary); }
    .bp-go {
      height: 26px;
      padding: 0 10px;
      font-size: 11px;
      font-weight: 600;
      background: var(--primary);
      color: #fff;
      border-radius: var(--radius-sm);
      flex-shrink: 0;
      transition: filter 0.1s;
    }
    .bp-go:hover { filter: brightness(1.15); }
    .bp-viewport {
      flex: 1;
      min-height: 0;
    }
  `;
  document.head.appendChild(s);
}
