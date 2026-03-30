import { createSignal, onMount, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

const created = new Set<string>();

interface Props {
  threadId: string;
}

export function BrowserPanel(props: Props) {
  const { store, setStore } = appStore;
  const [urlInput, setUrlInput] = createSignal(
    store.threadBrowserUrls[props.threadId] || "https://google.com"
  );
  let viewportRef: HTMLDivElement | undefined;
  let barRef: HTMLDivElement | undefined;
  let lastKey = "";

  // The native webview is positioned relative to the OS window content area.
  // getBoundingClientRect() gives coords relative to the main webview viewport.
  // There's a small offset (~5px) between these two coordinate systems.
  // We also need to add the address bar height since the webview must sit below it.
  const Y_OFFSET = 25;

  function getBounds() {
    if (!viewportRef) return null;
    const r = viewportRef.getBoundingClientRect();
    if (r.width < 10 || r.height < 10) return null;
    const barH = barRef ? barRef.getBoundingClientRect().height : 0;
    return {
      x: Math.round(r.left),
      y: Math.round(r.top + Y_OFFSET + barH),
      w: Math.round(r.width),
      h: Math.round(r.height - barH),
    };
  }

  function sync() {
    const b = getBounds();
    if (!b || b.h < 10) return;
    const key = `${b.x},${b.y},${b.w},${b.h}`;
    if (key === lastKey) return;
    lastKey = key;
    ipc.browserSetBounds(props.threadId, b.x, b.y, b.w, b.h).catch(() => {});
  }

  onMount(() => {
    const t = setTimeout(() => {
      const b = getBounds();
      if (!b || b.h < 10) return;
      const url = urlInput();

      if (created.has(props.threadId)) {
        ipc.browserSetBounds(props.threadId, b.x, b.y, b.w, b.h).catch(() => {});
      } else {
        ipc.browserOpen(props.threadId, url, b.x, b.y, b.w, b.h)
          .then(() => created.add(props.threadId))
          .catch((e) => console.error("browser_open:", e));
      }
    }, 300);

    const ro = new ResizeObserver(() => sync());
    if (viewportRef) ro.observe(viewportRef);
    window.addEventListener("resize", sync);
    const poll = setInterval(sync, 150);

    onCleanup(() => {
      clearTimeout(t);
      ro.disconnect();
      window.removeEventListener("resize", sync);
      clearInterval(poll);
      ipc.browserHide(props.threadId).catch(() => {});
    });
  });

  function navigate() {
    let u = urlInput().trim();
    if (!u) return;
    if (!u.match(/^https?:\/\//)) u = "https://" + u;
    setUrlInput(u);
    setStore("threadBrowserUrls", props.threadId, u);
    ipc.browserNavigate(props.threadId, u).catch(console.error);
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
    ipc.browserHide(props.threadId).catch(() => {});
  }

  return (
    <div ref={viewportRef} class="bp">
      {/* Address bar — in the DOM, above the native webview */}
      <div ref={barRef} class="bp-bar">
        <input
          class="bp-url"
          value={urlInput()}
          onInput={(e) => setUrlInput(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") { e.currentTarget.blur(); navigate(); } }}
          onBlur={navigate}
          placeholder="URL..."
        />
        <button class="bp-btn" onClick={() => ipc.browserDevtools(props.threadId)} title="DevTools">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/></svg>
        </button>
        <button class="bp-btn" onClick={close} title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
      {/* The native webview covers the remaining space below the bar */}
    </div>
  );
}

if (!document.getElementById("bp-styles")) {
  const s = document.createElement("style");
  s.id = "bp-styles";
  s.textContent = `
    .bp { flex:1; display:flex; flex-direction:column; min-height:0; overflow:hidden; }
    .bp-bar {
      display:flex; align-items:center; gap:3px; padding:5px 6px;
      border-bottom:1px solid var(--border); background:var(--bg-surface); flex-shrink:0;
      position:relative; z-index:10;
    }
    .bp-btn {
      width:24px;height:24px;border-radius:var(--radius-sm);
      display:flex;align-items:center;justify-content:center;
      color:var(--text-tertiary);transition:background .1s,color .1s;flex-shrink:0;
    }
    .bp-btn:hover { background:var(--bg-accent);color:var(--text-secondary); }
    .bp-url {
      flex:1;min-width:0;height:24px;padding:0 8px;font-size:12px;
      font-family:var(--font-mono);background:var(--bg-muted);
      border:1px solid var(--border);border-radius:var(--radius-sm);
      color:var(--text);outline:none;
    }
    .bp-url:focus { border-color:var(--primary); }
  `;
  document.head.appendChild(s);
}
