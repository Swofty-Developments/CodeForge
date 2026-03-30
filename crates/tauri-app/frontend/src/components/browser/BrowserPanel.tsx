import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

interface Props {
  threadId: string;
}

export function BrowserPanel(props: Props) {
  const { store, setStore } = appStore;

  const [urlInput, setUrlInput] = createSignal(store.threadBrowserUrls[props.threadId] || "https://google.com");
  const [screenshot, setScreenshot] = createSignal<string | null>(null);
  const [loading, setLoading] = createSignal(false);
  let imgRef: HTMLImageElement | undefined;
  let viewportRef: HTMLDivElement | undefined;

  // Viewport dimensions for coordinate mapping
  const VIEWPORT_W = 1200;
  const VIEWPORT_H = 800;

  onMount(async () => {
    const unlisten = await ipc.listenBrowserEvent((payload) => {
      if (payload.thread_id !== props.threadId) return;
      const t = payload.type;
      if (t === "screenshot" && payload.data) {
        setScreenshot(`data:image/png;base64,${payload.data}`);
        setLoading(false);
      } else if (t === "navigated" && payload.url) {
        setUrlInput(payload.url);
        setStore("threadBrowserUrls", props.threadId, payload.url);
      } else if (t === "ready") {
        navigate();
      }
    });
    onCleanup(() => unlisten());

    // Auto-resize the Playwright viewport to match the pane width
    if (viewportRef) {
      const ro = new ResizeObserver((entries) => {
        for (const e of entries) {
          const w = Math.round(e.contentRect.width * 2); // 2x for retina
          const h = Math.round(e.contentRect.height * 2);
          if (w > 100 && h > 100) {
            ipc.browserResize(props.threadId, w, h).catch(() => {});
          }
        }
      });
      ro.observe(viewportRef);
      onCleanup(() => ro.disconnect());
    }

    navigate();
  });

  function navigate() {
    let url = urlInput().trim();
    if (!url) return;
    if (!url.match(/^https?:\/\//)) url = "https://" + url;
    setUrlInput(url);
    setStore("threadBrowserUrls", props.threadId, url);
    setLoading(true);
    ipc.browserNavigate(props.threadId, url).catch(() => setLoading(false));
  }

  function handleClick(e: MouseEvent) {
    if (!imgRef) return;
    const rect = imgRef.getBoundingClientRect();
    const x = ((e.clientX - rect.left) / rect.width) * VIEWPORT_W;
    const y = ((e.clientY - rect.top) / rect.height) * VIEWPORT_H;
    setLoading(true);
    ipc.browserClick(props.threadId, x, y).catch(() => setLoading(false));
  }

  function handleScroll(e: WheelEvent) {
    e.preventDefault();
    ipc.browserScroll(props.threadId, e.deltaY * 2).catch(() => {});
  }

  function handleKeyDown(e: KeyboardEvent) {
    // Only forward when the viewport area is focused
    if (e.target !== viewportRef && e.target !== imgRef) return;
    e.preventDefault();
    if (e.key.length === 1) {
      ipc.browserTypeText(props.threadId, e.key).catch(() => {});
    } else {
      ipc.browserKeypress(props.threadId, e.key).catch(() => {});
    }
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
    ipc.browserClose(props.threadId).catch(() => {});
  }

  return (
    <div class="bp">
      <div class="bp-bar">
        <button class="bp-btn" onClick={() => ipc.browserBack(props.threadId)} title="Back">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="15 18 9 12 15 6"/></svg>
        </button>
        <button class="bp-btn" onClick={() => ipc.browserForward(props.threadId)} title="Forward">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="9 18 15 12 9 6"/></svg>
        </button>
        <button class="bp-btn" onClick={() => { setLoading(true); ipc.browserReload(props.threadId); }} title="Reload">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
        </button>
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
      <div
        ref={viewportRef}
        class="bp-viewport"
        tabIndex={0}
        onKeyDown={handleKeyDown}
      >
        <Show when={screenshot()}>
          <img
            ref={imgRef}
            class="bp-img"
            src={screenshot()!}
            onClick={handleClick}
            onWheel={handleScroll}
            draggable={false}
          />
        </Show>
        <Show when={loading() && !screenshot()}>
          <div class="bp-status">Loading...</div>
        </Show>
        <Show when={!loading() && !screenshot()}>
          <div class="bp-status">Enter a URL and press Go</div>
        </Show>
      </div>
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
      background: var(--bg-card);
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
    .bp-viewport {
      flex: 1; min-height: 0;
      overflow: hidden;
      background: #0a0a0a;
      display: flex;
      align-items: flex-start;
      justify-content: center;
      outline: none;
    }
    .bp-img {
      width: 100%;
      height: 100%;
      object-fit: contain;
      object-position: top left;
      cursor: pointer;
      image-rendering: auto;
    }
    .bp-status {
      color: var(--text-tertiary);
      font-size: 13px;
      padding: 32px;
      text-align: center;
      align-self: center;
    }
  `;
  document.head.appendChild(s);
}
