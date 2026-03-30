import { createSignal, Show, onMount, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";

interface Props {
  threadId: string;
}

export function BrowserPanel(props: Props) {
  const { store, setStore } = appStore;
  const [url, setUrl] = createSignal(store.threadBrowserUrls[props.threadId] || "https://en.wikipedia.org");
  const [loadedUrl, setLoadedUrl] = createSignal(url());
  const [inspecting, setInspecting] = createSignal(false);
  const [pasteMode, setPasteMode] = createSignal(false);
  const [pasteHtml, setPasteHtml] = createSignal("");
  let iframeRef: HTMLIFrameElement | undefined;

  function navigate() {
    let u = url().trim();
    if (!u) return;
    if (!u.match(/^https?:\/\//)) u = "https://" + u;
    setUrl(u);
    setLoadedUrl(u);
    setStore("threadBrowserUrls", props.threadId, u);
  }

  function close() {
    setStore("threadBrowserOpen", props.threadId, false);
  }

  // Try to inject inspector into iframe (works for same-origin only)
  function toggleInspect() {
    const next = !inspecting();
    setInspecting(next);
    if (next) {
      injectInspector();
    } else {
      removeInspector();
    }
  }

  function injectInspector() {
    try {
      const doc = iframeRef?.contentDocument;
      if (!doc) {
        // Cross-origin — fall back to paste mode
        setPasteMode(true);
        setInspecting(false);
        return;
      }
      if (doc.getElementById("cf-inspector")) return;

      const script = doc.createElement("script");
      script.id = "cf-inspector";
      script.textContent = `
        (function() {
          var ov = document.createElement('div');
          ov.id = 'cf-ov';
          ov.style.cssText = 'position:fixed;pointer-events:none;z-index:999999;border:2px solid #6b7cff;background:rgba(107,124,255,0.06);display:none;transition:all .08s;';
          document.body.appendChild(ov);
          var last = null;
          function onM(e) {
            var el = document.elementFromPoint(e.clientX, e.clientY);
            if (!el || el === ov) return;
            last = el;
            var r = el.getBoundingClientRect();
            ov.style.display = 'block';
            ov.style.left = r.left + 'px';
            ov.style.top = r.top + 'px';
            ov.style.width = r.width + 'px';
            ov.style.height = r.height + 'px';
          }
          function getStyles(el) {
            var cs = getComputedStyle(el);
            var d = document.createElement(el.tagName);
            document.body.appendChild(d);
            var ds = getComputedStyle(d);
            var out = {};
            var keep = ['color','background','background-color','font-size','font-weight','font-family',
              'padding','margin','border','border-radius','display','flex-direction','align-items',
              'justify-content','gap','width','height','max-width','position','box-shadow','text-align',
              'line-height','letter-spacing','overflow'];
            for (var i = 0; i < keep.length; i++) {
              var v = cs.getPropertyValue(keep[i]);
              var dv = ds.getPropertyValue(keep[i]);
              if (v && v !== dv && v !== 'none' && v !== 'normal' && v !== 'auto' && v !== '0px')
                out[keep[i]] = v;
            }
            d.remove();
            return out;
          }
          function onC(e) {
            e.preventDefault(); e.stopPropagation();
            if (!last) return;
            var html = last.outerHTML;
            if (html.length > 3000) html = html.substring(0, 3000) + '...';
            var css = JSON.stringify(getStyles(last), null, 2);
            window.parent.postMessage({ type: 'cf-extract', html: html, css: css }, '*');
          }
          document.addEventListener('mousemove', onM, true);
          document.addEventListener('click', onC, true);
          window.__cfClean = function() {
            document.removeEventListener('mousemove', onM, true);
            document.removeEventListener('click', onC, true);
            ov.remove();
          };
        })();
      `;
      doc.head.appendChild(script);
    } catch {
      // Cross-origin — show paste fallback
      setPasteMode(true);
      setInspecting(false);
    }
  }

  function removeInspector() {
    try {
      const doc = iframeRef?.contentDocument;
      if (!doc) return;
      if ((doc.defaultView as any)?.__cfClean) (doc.defaultView as any).__cfClean();
      doc.getElementById("cf-inspector")?.remove();
    } catch {}
  }

  // Listen for extraction messages from the iframe
  function onMessage(e: MessageEvent) {
    if (e.data?.type === "cf-extract") {
      addExtraction(e.data.html, e.data.css);
      setInspecting(false);
      removeInspector();
    }
  }

  onMount(() => {
    window.addEventListener("message", onMessage);
    onCleanup(() => window.removeEventListener("message", onMessage));
  });

  function addExtraction(html: string, css: string) {
    const formatted = `Extracted from ${loadedUrl()}:\n\n\`\`\`html\n${html}\n\`\`\`\n\n\`\`\`css\n${css}\n\`\`\``;
    // Add as attachment-style context (prepend to composer)
    const current = store.composerText;
    setStore("composerText", current ? formatted + "\n\n" + current : formatted);
  }

  function extractFromPaste() {
    const html = pasteHtml().trim();
    if (!html) return;
    addExtraction(html, "/* Paste mode — styles not available */");
    setPasteHtml("");
    setPasteMode(false);
  }

  return (
    <div class="bp">
      <div class="bp-bar">
        <input
          class="bp-url"
          value={url()}
          onInput={(e) => setUrl(e.currentTarget.value)}
          onKeyDown={(e) => { if (e.key === "Enter") navigate(); }}
          placeholder="URL..."
        />
        <button class="bp-go" onClick={navigate}>Go</button>
        <button
          class="bp-btn"
          classList={{ active: inspecting() }}
          onClick={toggleInspect}
          title="Inspect element — click to extract HTML/CSS"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z"/>
          </svg>
        </button>
        <button
          class="bp-btn"
          classList={{ active: pasteMode() }}
          onClick={() => setPasteMode(!pasteMode())}
          title="Paste HTML manually"
        >
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M16 4h2a2 2 0 012 2v14a2 2 0 01-2 2H6a2 2 0 01-2-2V6a2 2 0 012-2h2"/>
            <rect x="8" y="2" width="8" height="4" rx="1"/>
          </svg>
        </button>
        <button class="bp-btn" onClick={close} title="Close">
          <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
        </button>
      </div>

      <Show when={inspecting()}>
        <div class="bp-hint">Click an element in the page to extract its HTML and CSS</div>
      </Show>

      <Show when={pasteMode()}>
        <div class="bp-paste">
          <textarea
            class="bp-paste-area"
            value={pasteHtml()}
            onInput={(e) => setPasteHtml(e.currentTarget.value)}
            placeholder="Paste outerHTML from DevTools here..."
          />
          <button class="bp-paste-btn" onClick={extractFromPaste} disabled={!pasteHtml().trim()}>
            Extract to composer
          </button>
        </div>
      </Show>

      <Show when={!pasteMode()}>
        <iframe
          ref={iframeRef}
          class="bp-frame"
          src={loadedUrl()}
          sandbox="allow-same-origin allow-scripts allow-forms allow-popups allow-popups-to-escape-sandbox"
        />
      </Show>
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
    }
    .bp-btn {
      width:24px;height:24px;border-radius:var(--radius-sm);
      display:flex;align-items:center;justify-content:center;
      color:var(--text-tertiary);transition:background .1s,color .1s;flex-shrink:0;
    }
    .bp-btn:hover { background:var(--bg-accent);color:var(--text-secondary); }
    .bp-btn.active { color:var(--primary);background:rgba(107,124,255,0.15); }
    .bp-url {
      flex:1;min-width:0;height:24px;padding:0 8px;font-size:12px;
      font-family:var(--font-mono);background:var(--bg-muted);
      border:1px solid var(--border);border-radius:var(--radius-sm);
      color:var(--text);outline:none;
    }
    .bp-url:focus { border-color:var(--primary); }
    .bp-go {
      height:24px;padding:0 10px;font-size:11px;font-weight:600;
      background:var(--primary);color:#fff;border-radius:var(--radius-sm);flex-shrink:0;
    }
    .bp-go:hover { filter:brightness(1.15); }
    .bp-frame { flex:1; min-height:0; border:none; background:#fff; }
    .bp-hint {
      padding:4px 10px;font-size:10px;font-weight:500;
      color:var(--primary);background:var(--bg-muted);
      border-bottom:1px solid var(--border);text-align:center;flex-shrink:0;
    }
    .bp-paste {
      flex:1;display:flex;flex-direction:column;padding:10px;gap:8px;
      background:var(--bg-base);min-height:0;
    }
    .bp-paste-area {
      flex:1;min-height:60px;padding:8px;font-size:12px;
      font-family:var(--font-mono);background:var(--bg-surface);
      border:1px solid var(--border);border-radius:var(--radius-sm);
      color:var(--text);resize:none;outline:none;
    }
    .bp-paste-area:focus { border-color:var(--primary); }
    .bp-paste-area::placeholder { color:var(--text-tertiary); }
    .bp-paste-btn {
      align-self:flex-end;height:28px;padding:0 14px;font-size:11px;font-weight:600;
      background:var(--primary);color:#fff;border-radius:var(--radius-sm);
    }
    .bp-paste-btn:hover { filter:brightness(1.15); }
    .bp-paste-btn:disabled { opacity:0.4;cursor:not-allowed; }
  `;
  document.head.appendChild(s);
}
