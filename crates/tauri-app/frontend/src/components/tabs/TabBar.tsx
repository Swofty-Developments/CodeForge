import { For, Show, createSignal } from "solid-js";
import { DragDropProvider } from "@dnd-kit/solid";
import { useSortable } from "@dnd-kit/solid/sortable";
import { move } from "@dnd-kit/helpers";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

function SortableTab(props: { tabId: string; index: number }) {
  const { store, setStore, closeTab } = appStore;

  const thread = () =>
    store.projects.flatMap((p) => p.threads).find((t) => t.id === props.tabId);

  const isActive = () => store.activeTab === props.tabId;

  const color = () => {
    const t = thread();
    if (!t) return null;
    const project = store.projects.find((p) => p.threads.some((th) => th.id === t.id));
    return project?.color || null;
  };

  const sortable = useSortable({
    get id() { return props.tabId; },
    get index() { return props.index; },
  });

  return (
    <div
      ref={sortable.ref}
      class="tab"
      classList={{
        active: isActive(),
        dragging: sortable.isDragging,
      }}
      style={color() ? { "border-bottom": `2px solid ${color()}` } : {}}
      onClick={() => setStore("activeTab", props.tabId)}
    >
      <span class="tab-label">{thread()?.title || "..."}</span>
      <button
        class="tab-close"
        onClick={(e) => { e.stopPropagation(); closeTab(props.tabId); }}
      >
        &times;
      </button>
    </div>
  );
}

export function TabBar() {
  const { store, setStore } = appStore;
  const [showCopied, setShowCopied] = createSignal(false);

  const [urlInput, setUrlInput] = createSignal("");

  const browserOpen = () => {
    const tab = store.activeTab;
    return tab ? !!store.threadBrowserOpen[tab] : false;
  };

  function toggleBrowser() {
    const tab = store.activeTab;
    if (!tab) return;
    const opening = !store.threadBrowserOpen[tab];
    setStore("threadBrowserOpen", tab, opening);
    if (opening) setUrlInput(store.threadBrowserUrls[tab] || "https://google.com");
    if (!opening) ipc.browserHide(tab).catch(() => {});
  }

  function navigateBrowser() {
    const tab = store.activeTab;
    if (!tab) return;
    let u = urlInput().trim();
    if (!u) return;
    if (!u.match(/^https?:\/\//)) u = "https://" + u;
    setUrlInput(u);
    setStore("threadBrowserUrls", tab, u);
    ipc.browserNavigate(tab, u).catch(console.error);
  }

  function closeBrowser() {
    const tab = store.activeTab;
    if (!tab) return;
    setStore("threadBrowserOpen", tab, false);
    ipc.browserHide(tab).catch(() => {});
  }

  function openDevtools() {
    const tab = store.activeTab;
    if (tab) ipc.browserDevtools(tab).catch(console.error);
  }

  function inspectElement() {
    const tab = store.activeTab;
    if (!tab) return;
    ipc.browserEval(tab, `
      (function(){
        if(window.__cfI)return;window.__cfI=true;
        var o=document.createElement('div');
        o.style.cssText='position:fixed;pointer-events:none;z-index:999999;border:2px solid #6b7cff;background:rgba(107,124,255,0.08);display:none;transition:all .06s;';
        document.body.appendChild(o);var last=null;
        function mm(e){var el=document.elementFromPoint(e.clientX,e.clientY);if(!el||el===o)return;last=el;var r=el.getBoundingClientRect();o.style.display='block';o.style.left=r.left+'px';o.style.top=r.top+'px';o.style.width=r.width+'px';o.style.height=r.height+'px';}
        function mc(e){e.preventDefault();e.stopPropagation();if(!last)return;
          var html=last.outerHTML;if(html.length>4000)html=html.substring(0,4000)+'...';
          var cs=getComputedStyle(last),s={},keep=['color','background','background-color','font-size','font-weight','font-family','padding','margin','border','border-radius','display','width','height','position','box-shadow','text-align','line-height','gap','flex-direction','align-items','justify-content'];
          var d=document.createElement(last.tagName);document.body.appendChild(d);var ds=getComputedStyle(d);
          for(var i=0;i<keep.length;i++){var v=cs.getPropertyValue(keep[i]);var dv=ds.getPropertyValue(keep[i]);if(v&&v!==dv&&v!=='none'&&v!=='normal'&&v!=='auto'&&v!=='0px')s[keep[i]]=v;}d.remove();
          document.title='__CF_EXTRACT__'+JSON.stringify({html:html,css:JSON.stringify(s,null,2)});
          document.removeEventListener('mousemove',mm,true);document.removeEventListener('click',mc,true);o.remove();window.__cfI=false;}
        document.addEventListener('mousemove',mm,true);document.addEventListener('click',mc,true);
      })();
    `).catch(console.error);
  }

  function toggleDiff() {
    setStore("diffPanelOpen", !store.diffPanelOpen);
  }

  function exportChat() {
    const tab = store.activeTab;
    if (!tab) return;
    const msgs = store.threadMessages[tab];
    if (!msgs || msgs.length === 0) return;

    const thread = store.projects.flatMap((p) => p.threads).find((t) => t.id === tab);
    const title = thread?.title || "Chat";

    const md = `# ${title}\n\n` + msgs.map((m) => {
      const role = m.role === "user" ? "You" : m.role === "assistant" ? "Assistant" : "System";
      return `**${role}:**\n${m.content}\n`;
    }).join("\n---\n\n");

    navigator.clipboard.writeText(md).then(() => {
      setShowCopied(true);
      setTimeout(() => setShowCopied(false), 1500);
    });
  }

  return (
    <Show when={store.openTabs.length > 0}>
      <DragDropProvider
        onDragEnd={(event) => {
          setStore("openTabs", (tabs) => move(tabs, event));
        }}
      >
        <div class="tab-bar">
          <div class="tab-bar-tabs">
            <For each={store.openTabs}>
              {(tabId, idx) => <SortableTab tabId={tabId} index={idx()} />}
            </For>
          </div>
          <Show when={store.activeTab}>
            <div class="tab-bar-actions">
              <button
                class="tb-action"
                classList={{ active: browserOpen() }}
                onClick={toggleBrowser}
                title="Browser (Cmd+Shift+B)"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <circle cx="12" cy="12" r="10"/><line x1="2" y1="12" x2="22" y2="12"/><path d="M12 2a15.3 15.3 0 014 10 15.3 15.3 0 01-4 10 15.3 15.3 0 01-4-10 15.3 15.3 0 014-10z"/>
                </svg>
              </button>
              <button class="tb-action" onClick={exportChat} title="Export chat" style="position:relative;">
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M21 15v4a2 2 0 01-2 2H5a2 2 0 01-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/>
                </svg>
                {showCopied() && <span class="tb-toast">Copied!</span>}
              </button>
              <button
                class="tb-action"
                classList={{ active: store.diffPanelOpen }}
                onClick={toggleDiff}
                title="Diff view (Cmd+Shift+D)"
              >
                <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <line x1="6" y1="3" x2="6" y2="15"/><circle cx="18" cy="6" r="3"/><circle cx="6" cy="18" r="3"/><path d="M18 9a9 9 0 01-9 9"/>
                </svg>
              </button>
            </div>
          </Show>
          <Show when={browserOpen()}>
            <div class="tb-browser-bar">
              <input
                class="tb-burl"
                value={urlInput()}
                onInput={(e) => setUrlInput(e.currentTarget.value)}
                onKeyDown={(e) => { if (e.key === "Enter") navigateBrowser(); }}
                placeholder="URL..."
              />
              <button class="tb-bgo" onClick={navigateBrowser}>Go</button>
              <button class="tb-action" onClick={inspectElement} title="Select element">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <path d="M12 20h9"/><path d="M16.5 3.5a2.12 2.12 0 013 3L7 19l-4 1 1-4z"/>
                </svg>
              </button>
              <button class="tb-action" onClick={openDevtools} title="DevTools">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
                  <polyline points="16 18 22 12 16 6"/><polyline points="8 6 2 12 8 18"/>
                </svg>
              </button>
              <button class="tb-action" onClick={closeBrowser} title="Close browser">
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
              </button>
            </div>
          </Show>
        </div>
      </DragDropProvider>
    </Show>
  );
}

if (!document.getElementById("tab-bar-styles")) {
  const s = document.createElement("style");
  s.id = "tab-bar-styles";
  s.textContent = `
    .tab-bar {
      display: flex;
      align-items: stretch;
      padding: 6px 8px 0;
      background: var(--bg-muted);
      border-bottom: 1px solid var(--border);
      flex-shrink: 0;
    }
    .tab-bar-tabs {
      display: flex;
      align-items: stretch;
      gap: 1px;
      flex: 1;
      overflow-x: auto;
      overflow-y: hidden;
      min-width: 0;
    }
    .tab-bar-tabs::-webkit-scrollbar { height: 0; display: none; }
    .tab-bar-actions {
      display: flex;
      align-items: center;
      gap: 2px;
      padding: 0 2px 4px;
      flex-shrink: 0;
      margin-left: 4px;
    }
    .tb-url-group {
      display: flex;
      align-items: center;
      gap: 3px;
      padding: 0 4px 4px;
      margin-left: auto;
      min-width: 140px;
      max-width: 280px;
      flex-shrink: 1;
    }
    .tb-url {
      flex: 1;
      min-width: 0;
      height: 22px;
      padding: 0 6px;
      font-size: 11px;
      font-family: var(--font-mono);
      background: var(--bg-accent);
      border: 1px solid var(--border);
      border-radius: 4px;
      color: var(--text);
      outline: none;
    }
    .tb-url:focus { border-color: var(--primary); }
    .tb-url-go {
      height: 22px;
      padding: 0 8px;
      font-size: 10px;
      font-weight: 600;
      background: var(--primary);
      color: #fff;
      border-radius: 4px;
      flex-shrink: 0;
    }
    .tb-url-go:hover { filter: brightness(1.15); }
    .tb-action {
      width: 24px;
      height: 24px;
      border-radius: var(--radius-sm);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--text-tertiary);
      transition: background 0.1s, color 0.1s;
      position: relative;
    }
    .tb-action:hover {
      background: var(--bg-accent);
      color: var(--text-secondary);
    }
    .tb-action.active {
      color: var(--primary);
      background: var(--primary-glow);
    }
    .tb-toast {
      position: absolute;
      top: -24px;
      left: 50%;
      transform: translateX(-50%);
      background: var(--bg-accent);
      color: var(--green);
      font-size: 10px;
      font-weight: 600;
      padding: 2px 7px;
      border-radius: 4px;
      border: 1px solid var(--border);
      white-space: nowrap;
      pointer-events: none;
    }
    .tb-browser-bar {
      display: flex;
      align-items: center;
      gap: 3px;
      padding: 0 4px 4px;
      margin-left: auto;
      min-width: 140px;
      max-width: 320px;
      flex-shrink: 1;
    }
    .tb-burl {
      flex: 1;
      min-width: 0;
      height: 22px;
      padding: 0 6px;
      font-size: 11px;
      font-family: var(--font-mono);
      background: var(--bg-accent);
      border: 1px solid var(--border);
      border-radius: 4px;
      color: var(--text);
      outline: none;
    }
    .tb-burl:focus { border-color: var(--primary); }
    .tb-bgo {
      height: 22px;
      padding: 0 8px;
      font-size: 10px;
      font-weight: 600;
      background: var(--primary);
      color: #fff;
      border-radius: 4px;
      flex-shrink: 0;
    }
    .tb-bgo:hover { filter: brightness(1.15); }
    .tab {
      display: flex;
      align-items: center;
      gap: 6px;
      padding: 6px 12px;
      font-size: 12px;
      font-weight: 500;
      color: var(--text-secondary);
      border-radius: var(--radius-sm) var(--radius-sm) 0 0;
      cursor: grab;
      transition: background 0.12s, color 0.12s, opacity 0.15s;
      white-space: nowrap;
      user-select: none;
      flex-shrink: 0;
      touch-action: none;
    }
    .tab:active { cursor: grabbing; }
    .tab:hover { background: var(--bg-hover); color: var(--text-secondary); }
    .tab.active {
      background: var(--bg-base);
      color: var(--text);
      border: 1px solid var(--border-strong);
      border-bottom: 1px solid var(--bg-base);
      margin-bottom: -1px;
    }
    .tab.dragging { opacity: 0.4; }
    .tab-label {
      overflow: hidden;
      text-overflow: ellipsis;
      max-width: 120px;
    }
    .tab-close {
      color: var(--text-tertiary);
      padding: 2px;
      line-height: 1;
      border-radius: 3px;
      transition: background 0.12s, color 0.12s;
      cursor: pointer;
      display: flex;
      align-items: center;
    }
    .tab-close:hover {
      background: var(--bg-accent);
      color: var(--text);
    }
  `;
  document.head.appendChild(s);
}
