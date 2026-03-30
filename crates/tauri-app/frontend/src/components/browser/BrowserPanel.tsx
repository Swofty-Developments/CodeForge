import { onMount, onCleanup } from "solid-js";
import { appStore } from "../../stores/app-store";
import * as ipc from "../../ipc";

// Track created webviews
const created = new Set<string>();

interface Props {
  threadId: string;
}

/**
 * This component is JUST a placeholder div. The native webview
 * is positioned over it by Rust. All controls (URL bar, buttons)
 * live in the TabBar component above — NOT here — because the
 * native webview renders on top of all DOM content.
 */
export function BrowserPanel(props: Props) {
  const { store } = appStore;
  let ref: HTMLDivElement | undefined;
  let lastKey = "";

  function sync() {
    if (!ref) return;
    const r = ref.getBoundingClientRect();
    if (r.width < 10 || r.height < 10) return;
    const key = `${Math.round(r.left)},${Math.round(r.top)},${Math.round(r.width)},${Math.round(r.height)}`;
    if (key === lastKey) return;
    lastKey = key;
    ipc.browserSetBounds(props.threadId, Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)).catch(() => {});
  }

  onMount(() => {
    const t = setTimeout(() => {
      if (!ref) return;
      const r = ref.getBoundingClientRect();
      if (r.width < 10 || r.height < 10) return;
      const url = store.threadBrowserUrls[props.threadId] || "https://google.com";

      if (created.has(props.threadId)) {
        // Already exists — just reposition
        ipc.browserSetBounds(props.threadId, Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height)).catch(() => {});
      } else {
        ipc.browserOpen(props.threadId, url, Math.round(r.left), Math.round(r.top), Math.round(r.width), Math.round(r.height))
          .then(() => created.add(props.threadId))
          .catch((e) => console.error("browser_open:", e));
      }
    }, 300);

    const ro = new ResizeObserver(() => sync());
    if (ref) ro.observe(ref);
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

  return <div ref={ref} style="flex:1;min-height:0;min-width:0;" />;
}
