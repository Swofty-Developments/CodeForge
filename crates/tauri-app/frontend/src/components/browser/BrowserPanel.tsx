import { createSignal, onMount, onCleanup, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

interface Props { threadId: string; }

// Viewport dimensions for coordinate mapping
const VP_W = 1280;
const VP_H = 800;

export function BrowserPanel(props: Props) {
  const { store, setStore } = appStore;
  const [urlInput, setUrlInput] = createSignal(store.threadBrowserUrls[props.threadId] || "https://google.com");
  const [currentUrl, setCurrentUrl] = createSignal(urlInput());
  const [inspecting, setInspecting] = createSignal(false);
  const [loading, setLoading] = createSignal(false);
  let extractPending = false;
  let canvasRef: HTMLCanvasElement | undefined;
  let containerRef: HTMLDivElement | undefined;
  let img = new Image();

  // Map canvas coords to viewport coords
  function toVP(e: MouseEvent) {
    if (!canvasRef) return { x: 0, y: 0 };
    const r = canvasRef.getBoundingClientRect();
    return {
      x: (e.clientX - r.left) / r.width * VP_W,
      y: (e.clientY - r.top) / r.height * VP_H,
    };
  }

  onMount(async () => {
    const unlisten = await ipc.listenBrowserEvent((p) => {
      if (p.thread_id !== props.threadId) return;
      switch (p.type) {
        case "frame":
          if (p.data && canvasRef) {
            img.onload = () => {
              const ctx = canvasRef!.getContext("2d");
              if (ctx) {
                canvasRef!.width = img.width;
                canvasRef!.height = img.height;
                ctx.drawImage(img, 0, 0);
              }
            };
            img.src = `data:image/jpeg;base64,${p.data}`;
          }
          break;
        case "navigated":
          if (p.url) {
            setCurrentUrl(p.url);
            setUrlInput(p.url);
            setStore("threadBrowserUrls", props.threadId, p.url);
          }
          setLoading(false);
          break;
        case "extraction":
          if (p.html && extractPending) {
            extractPending = false;
            const content = `<!-- From ${currentUrl()} ${p.selector || ""} -->\n${p.html}\n\n/* Computed styles */\n${p.css || "{}"}`;
            setStore("attachments", (prev) => [...prev, {
              id: crypto.randomUUID(),
              type: "extraction" as const,
              name: p.selector || "Element",
              content,
              language: "html",
            }]);
          }
          setInspecting(false);
          break;
        case "ready":
          navigate();
          break;
      }
    });
    onCleanup(() => unlisten());
    navigate();
  });

  function navigate() {
    let u = urlInput().trim();
    if (!u) return;
    if (!u.match(/^https?:\/\//)) u = "https://" + u;
    setUrlInput(u);
    setStore("threadBrowserUrls", props.threadId, u);
    setLoading(true);
    ipc.browserNavigate(props.threadId, u).catch(() => setLoading(false));
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
    ipc.browserClose(props.threadId).catch(() => {});
  }

  function toggleInspect() {
    const next = !inspecting();
    setInspecting(next);
    if (next) ipc.browserStartInspect(props.threadId);
    else ipc.browserStopInspect(props.threadId);
  }

  function handleClick(e: MouseEvent) {
    const { x, y } = toVP(e);
    if (inspecting()) {
      extractPending = true;
      ipc.browserExtract(props.threadId);
    } else {
      ipc.browserClick(props.threadId, x, y);
    }
  }

  function handleMouseMove(e: MouseEvent) {
    if (!inspecting()) return;
    const { x, y } = toVP(e);
    ipc.browserMouseMove(props.threadId, x, y);
  }

  function handleScroll(e: WheelEvent) {
    e.preventDefault();
    const { x, y } = toVP(e);
    ipc.browserScroll(props.threadId, x, y, e.deltaX, e.deltaY);
  }

  function handleKeyDown(e: KeyboardEvent) {
    e.preventDefault();
    if (e.key.length === 1) {
      ipc.browserTypeText(props.threadId, e.key);
    } else {
      ipc.browserKeyDown(props.threadId, e.key, "");
    }
  }

  function handleKeyUp(e: KeyboardEvent) {
    if (e.key.length > 1) ipc.browserKeyUp(props.threadId, e.key);
  }

  return (
    <div class="bp">
      <div class="bp-bar">
        <button class="bp-nav" onClick={() => ipc.browserBack(props.threadId)} title="Back">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="15 18 9 12 15 6"/></svg>
        </button>
        <button class="bp-nav" onClick={() => ipc.browserForward(props.threadId)} title="Forward">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><polyline points="9 18 15 12 9 6"/></svg>
        </button>
        <button class="bp-nav" onClick={() => { setLoading(true); ipc.browserReload(props.threadId); }} title="Reload">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round"><polyline points="23 4 23 10 17 10"/><path d="M20.49 15a9 9 0 11-2.12-9.36L23 10"/></svg>
        </button>
        <input
          class="bp-url"
          value={urlInput()}
          onInput={(e) => setUrlInput(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") { e.currentTarget.blur(); navigate(); } }}
          onBlur={navigate}
        />
        <button
          class="bp-nav"
          classList={{ active: inspecting() }}
          onClick={toggleInspect}
          title="Inspect element"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z"/>
          </svg>
        </button>
        <button class="bp-nav" onClick={close} title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>
      <Show when={inspecting()}>
        <div class="bp-inspect-bar">Click an element to extract its HTML &amp; CSS</div>
      </Show>
      <div ref={containerRef} class="bp-viewport" tabIndex={0} onKeyDown={handleKeyDown} onKeyUp={handleKeyUp}>
        <canvas
          ref={canvasRef}
          class="bp-canvas"
          classList={{ inspecting: inspecting() }}
          onClick={handleClick}
          onMouseMove={handleMouseMove}
          onWheel={handleScroll}
        />
        <Show when={loading()}>
          <div class="bp-loading">Loading...</div>
        </Show>
      </div>
    </div>
  );
}

if (!document.getElementById("bp-styles")) {
  const s = document.createElement("style");
  s.id = "bp-styles";
  s.textContent = `
    .bp { flex:1; display:flex; flex-direction:column; min-height:0; overflow:hidden; }
    .bp-bar {
      display:flex; align-items:center; gap:2px; padding:4px 6px;
      border-bottom:1px solid var(--border); background:var(--bg-surface); flex-shrink:0;
    }
    .bp-nav {
      width:24px;height:24px;border-radius:var(--radius-sm);
      display:flex;align-items:center;justify-content:center;
      color:var(--text-tertiary);transition:background .1s,color .1s;flex-shrink:0;
    }
    .bp-nav:hover { background:var(--bg-accent);color:var(--text-secondary); }
    .bp-nav.active { color:var(--primary);background:rgba(107,124,255,0.15); }
    .bp-url {
      flex:1;min-width:0;height:24px;padding:0 8px;font-size:12px;
      font-family:var(--font-mono);background:var(--bg-muted);
      border:1px solid var(--border);border-radius:var(--radius-sm);
      color:var(--text);outline:none;
    }
    .bp-url:focus { border-color:var(--primary); }
    .bp-inspect-bar {
      padding:3px 10px;font-size:10px;font-weight:500;color:var(--primary);
      background:rgba(107,124,255,0.06);border-bottom:1px solid var(--border);
      text-align:center;flex-shrink:0;
    }
    .bp-viewport {
      flex:1;min-height:0;position:relative;overflow:hidden;
      background:#0a0a0a;outline:none;
    }
    .bp-canvas {
      width:100%;height:100%;object-fit:contain;cursor:default;
      display:block;
    }
    .bp-canvas.inspecting { cursor:crosshair; }
    .bp-loading {
      position:absolute;top:50%;left:50%;transform:translate(-50%,-50%);
      color:var(--text-tertiary);font-size:12px;
    }
  `;
  document.head.appendChild(s);
}
