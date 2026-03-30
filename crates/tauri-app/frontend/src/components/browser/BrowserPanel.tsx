import { createSignal, onMount, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";
import { Window } from "@tauri-apps/api/window";
import { Webview } from "@tauri-apps/api/webview";

interface Props {
  threadId: string;
}

// Track webview instances across component lifecycle
const webviewInstances: Record<string, Webview> = {};

export function BrowserPanel(props: Props) {
  const { store, setStore } = appStore;
  const [urlInput, setUrlInput] = createSignal(
    store.threadBrowserUrls[props.threadId] || "https://google.com"
  );
  const [ready, setReady] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;

  const label = () => `browser-${props.threadId.replace(/[^a-zA-Z0-9-]/g, "")}`;

  function getContainerBounds() {
    if (!containerRef) return null;
    const rect = containerRef.getBoundingClientRect();
    if (rect.width < 10 || rect.height < 10) return null;
    return { x: Math.round(rect.left), y: Math.round(rect.top), w: Math.round(rect.width), h: Math.round(rect.height) };
  }

  async function updateBounds() {
    const wv = webviewInstances[props.threadId];
    if (!wv) return;
    const b = getContainerBounds();
    if (!b) return;
    try {
      await wv.setPosition(new (await import("@tauri-apps/api/dpi")).LogicalPosition(b.x, b.y));
      await wv.setSize(new (await import("@tauri-apps/api/dpi")).LogicalSize(b.w, b.h));
    } catch {}
  }

  async function createWebview() {
    const existing = webviewInstances[props.threadId];
    if (existing) {
      // Already exists — just reposition and show
      setReady(true);
      await updateBounds();
      return;
    }

    const b = getContainerBounds();
    if (!b) return;

    try {
      const appWindow = await Window.getByLabel("main");
      if (!appWindow) return;

      const url = urlInput().trim() || "https://google.com";
      const wv = new Webview(appWindow, label(), {
        url,
        x: b.x,
        y: b.y,
        width: b.w,
        height: b.h,
      });

      // Wait for it to be created
      await wv.once("tauri://created", () => {});
      webviewInstances[props.threadId] = wv;
      setReady(true);
    } catch (e) {
      console.error("Failed to create webview:", e);
    }
  }

  async function navigate() {
    let url = urlInput().trim();
    if (!url) return;
    if (!url.match(/^https?:\/\//)) url = "https://" + url;
    setUrlInput(url);
    setStore("threadBrowserUrls", props.threadId, url);

    const wv = webviewInstances[props.threadId];
    if (wv) {
      try {
        // Navigate by evaluating JS in the webview
        await wv.eval(`window.location.href = ${JSON.stringify(url)}`);
      } catch {
        // If eval fails, destroy and recreate
        await destroyWebview();
        await createWebview();
      }
    }
  }

  async function destroyWebview() {
    const wv = webviewInstances[props.threadId];
    if (wv) {
      try { await wv.close(); } catch {}
      delete webviewInstances[props.threadId];
    }
    setReady(false);
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
    // Move offscreen instead of destroying (so it persists when switching tabs)
    const wv = webviewInstances[props.threadId];
    if (wv) {
      import("@tauri-apps/api/dpi").then(({ LogicalPosition }) => {
        wv.setPosition(new LogicalPosition(-9999, -9999)).catch(() => {});
      });
    }
  }

  onMount(() => {
    // Wait for layout to settle then create the webview
    setTimeout(() => createWebview(), 150);

    // Track bounds changes
    const ro = new ResizeObserver(() => updateBounds());
    if (containerRef) ro.observe(containerRef);
    window.addEventListener("resize", updateBounds);
    const interval = setInterval(updateBounds, 300);

    onCleanup(() => {
      ro.disconnect();
      window.removeEventListener("resize", updateBounds);
      clearInterval(interval);
      // Move offscreen when unmounting (tab switch)
      const wv = webviewInstances[props.threadId];
      if (wv) {
        import("@tauri-apps/api/dpi").then(({ LogicalPosition }) => {
          wv.setPosition(new LogicalPosition(-9999, -9999)).catch(() => {});
        });
      }
    });
  });

  return (
    <div class="bp">
      <div class="bp-bar">
        <input
          class="bp-url"
          value={urlInput()}
          onInput={(e) => setUrlInput(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") navigate(); }}
          placeholder="URL..."
        />
        <button class="bp-go" onClick={navigate}>Go</button>
        <button class="bp-btn" onClick={close} title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
      <div ref={containerRef} class="bp-container" />
    </div>
  );
}

if (!document.getElementById("bp-styles")) {
  const s = document.createElement("style");
  s.id = "bp-styles";
  s.textContent = `
    .bp {
      flex: 1;
      display: flex;
      flex-direction: column;
      min-height: 0;
      overflow: hidden;
    }
    .bp-bar {
      display: flex;
      align-items: center;
      gap: 3px;
      padding: 5px 6px;
      border-bottom: 1px solid var(--border);
      background: var(--bg-surface);
      flex-shrink: 0;
      z-index: 10;
    }
    .bp-btn {
      width: 24px; height: 24px;
      border-radius: var(--radius-sm);
      display: flex; align-items: center; justify-content: center;
      color: var(--text-tertiary);
      transition: background 0.1s, color 0.1s;
      flex-shrink: 0;
    }
    .bp-btn:hover { background: var(--bg-accent); color: var(--text-secondary); }
    .bp-url {
      flex: 1; min-width: 0; height: 24px;
      padding: 0 8px; font-size: 12px;
      font-family: var(--font-mono);
      background: var(--bg-muted);
      border: 1px solid var(--border);
      border-radius: var(--radius-sm);
      color: var(--text); outline: none;
    }
    .bp-url:focus { border-color: var(--primary); }
    .bp-go {
      height: 24px; padding: 0 10px;
      font-size: 11px; font-weight: 600;
      background: var(--primary); color: #fff;
      border-radius: var(--radius-sm);
      flex-shrink: 0;
    }
    .bp-go:hover { filter: brightness(1.15); }
    .bp-container {
      flex: 1;
      min-height: 0;
    }
  `;
  document.head.appendChild(s);
}
