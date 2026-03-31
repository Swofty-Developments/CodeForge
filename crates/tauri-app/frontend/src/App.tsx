import { Show, ErrorBoundary, createSignal, onMount, onCleanup } from "solid-js";
import { appStore } from "./stores/app-store";
import { WelcomeScreen } from "./components/shared/WelcomeScreen";
import { SetupWizard } from "./components/onboarding/SetupWizard";
import { Sidebar } from "./components/sidebar/Sidebar";
import { TabBar } from "./components/tabs/TabBar";
import { ChatArea } from "./components/chat/ChatArea";
import { Composer } from "./components/composer/Composer";
import { SettingsOverlay } from "./components/settings/SettingsOverlay";
import { ContextMenu } from "./components/shared/ContextMenu";
import { ProviderPicker } from "./components/composer/ProviderPicker";
import { CommandPalette } from "./components/shared/CommandPalette";
import { SearchOverlay } from "./components/shared/SearchOverlay";
import { UsageDashboard } from "./components/settings/UsageDashboard";
import { ThemeSelector } from "./components/settings/ThemeSelector";
import { SplitView } from "./components/shared/SplitView";
import { BrowserPanel } from "./components/browser/BrowserPanel";
import { DiffEditor } from "./components/diff/DiffEditor";
import * as ipc from "./ipc";

export function App() {
  const { store, setStore } = appStore;
  const [showWelcome, setShowWelcome] = createSignal(true);
  const [showSetup, setShowSetup] = createSignal(false);

  // Check onboarding status on mount
  onMount(async () => {
    try {
      const status = await ipc.checkSetupStatus();
      if (!status.complete) {
        setShowSetup(true);
      }
    } catch {
      // If check fails, don't block
    }
  });

  // Side pane resize state (percentage of main-panel-body width/height for chat)
  const [chatPercent, setChatPercent] = createSignal(60);
  const [draggingDivider, setDraggingDivider] = createSignal(false);
  const [isVertical, setIsVertical] = createSignal(false);
  let bodyRef: HTMLDivElement | undefined;
  let chatPercentRef = 60;

  // Whether active thread is in a git-activated project
  const isGitProject = () => {
    const tab = store.activeTab;
    if (!tab) return false;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    return project ? project.path !== "." : false;
  };

  // Check if active thread is linked to a PR
  const activePrNumber = (): number | null => {
    const tab = store.activeTab;
    if (!tab) return null;
    const project = store.projects.find((p) => p.threads.some((t) => t.id === tab));
    if (!project) return null;
    const prMap = store.projectPrMap[project.id];
    return prMap?.[tab] ?? null;
  };

  // Compute diff cwd reactively based on active thread
  const diffCwd = () => {
    const tab = store.activeTab;
    if (tab) {
      const wt = store.worktrees[tab];
      if (wt?.active) return wt.path;
      const proj = store.projects.find((p) => p.threads.some((t) => t.id === tab));
      if (proj && proj.path !== ".") return proj.path;
    }
    return ".";
  };

  // Check if side pane is open at all (diff only for git projects)
  const hasSidePane = () => {
    const tab = store.activeTab;
    const browserOpen = tab && store.threadBrowserOpen[tab];
    const diffOpen = store.activeTab && store.threadDiffOpen[store.activeTab] && isGitProject();
    return browserOpen || diffOpen;
  };

  // Observe body width to switch vertical/horizontal
  onMount(() => {
    const ro = new ResizeObserver((entries) => {
      for (const entry of entries) {
        setIsVertical(entry.contentRect.width < 700);
      }
    });
    if (bodyRef) ro.observe(bodyRef);
    onCleanup(() => ro.disconnect());
  });

  function handleDividerDown(e: MouseEvent) {
    e.preventDefault();
    if (!bodyRef) return;

    // Cache rect once at start — no layout thrashing during drag
    const rect = bodyRef.getBoundingClientRect();
    const vert = isVertical();

    // Add dragging class directly on DOM — no SolidJS reactivity
    bodyRef.classList.add("dragging");
    document.body.style.cursor = vert ? "row-resize" : "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (ev: MouseEvent) => {
      let pct: number;
      if (vert) {
        pct = ((ev.clientY - rect.top) / rect.height) * 100;
      } else {
        pct = ((ev.clientX - rect.left) / rect.width) * 100;
      }
      pct = Math.min(Math.max(pct, 25), 80);
      bodyRef!.style.setProperty("--chat-pct", `${pct}%`);
      bodyRef!.style.setProperty("--side-pct", `${100 - pct}%`);
      chatPercentRef = pct;
    };
    const onUp = () => {
      bodyRef?.classList.remove("dragging");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      setChatPercent(chatPercentRef);
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      document.body.style.cursor = "";
    };
    document.body.style.cursor = isVertical() ? "row-resize" : "col-resize";
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handleKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    const key = e.key.toLowerCase();
    if (mod && key === "k" && !e.shiftKey) {
      e.preventDefault();
      setStore("commandPaletteOpen", !store.commandPaletteOpen);
    }
    if (mod && e.shiftKey && key === "f") {
      e.preventDefault();
      setStore("searchOpen", !store.searchOpen);
    }
    if (mod && key === "\\") {
      e.preventDefault();
      if (store.splitTab) {
        setStore("splitTab", null);
      } else if (store.activeTab) {
        const other = store.openTabs.find((t) => t !== store.activeTab);
        if (other) setStore("splitTab", other);
      }
    }
    if (mod && e.shiftKey && key === "u") {
      e.preventDefault();
      setStore("usageDashboardOpen", !store.usageDashboardOpen);
    }
    if (mod && e.shiftKey && key === "b") {
      e.preventDefault();
      if (store.activeTab) {
        setStore("threadBrowserOpen", store.activeTab, !store.threadBrowserOpen[store.activeTab]);
      }
    }
    if (mod && e.shiftKey && key === "d") {
      e.preventDefault();
      if (store.activeTab) setStore("threadDiffOpen", store.activeTab, !store.threadDiffOpen[store.activeTab]);
    }
  }

  onMount(() => window.addEventListener("keydown", handleKeyDown));
  onCleanup(() => window.removeEventListener("keydown", handleKeyDown));

  const chatStyle = () => {
    if (!hasSidePane()) return {};
    const pct = `var(--chat-pct, ${chatPercent()}%)`;
    return isVertical()
      ? { height: pct, "min-height": "120px" }
      : { width: pct, "min-width": "200px" };
  };

  const sideStyle = () => {
    if (!hasSidePane()) return {};
    const pct = `var(--side-pct, ${100 - chatPercent()}%)`;
    return isVertical()
      ? { height: pct, "min-height": "120px" }
      : { width: pct, "min-width": "200px" };
  };

  return (
    <ErrorBoundary fallback={(err, reset) => (
      <div class="error-boundary">
        <div class="error-boundary-content">
          <h2 class="error-boundary-title">Something went wrong</h2>
          <p class="error-boundary-message">{err?.message || String(err)}</p>
          <details class="error-boundary-details">
            <summary>Stack trace</summary>
            <pre class="error-boundary-stack">{err?.stack || "No stack trace available"}</pre>
          </details>
          <button class="error-boundary-reload" onClick={() => location.reload()}>Reload</button>
        </div>
      </div>
    )}>
    <>
      <Show when={showSetup()}>
        <SetupWizard onComplete={() => setShowSetup(false)} />
      </Show>
      <Sidebar />
      <Show
        when={store.splitTab}
        fallback={
          <div class="main-panel">
            <TabBar />
            <div
              ref={bodyRef}
              class="main-panel-body"
              classList={{
                vertical: isVertical(),
                /* dragging class added/removed via DOM directly for performance */
              }}
            >
              <div class="main-panel-chat" style={hasSidePane() ? chatStyle() : { flex: "1" }}>
                <ChatArea />
                <Composer />
              </div>

              <Show when={hasSidePane()}>
                <div
                  class="pane-divider"
                  classList={{ vertical: isVertical() }}
                  onMouseDown={handleDividerDown}
                >
                  <div class="pane-divider-line" />
                </div>
                <div class="main-panel-side" style={sideStyle()}>
                  <Show when={store.activeTab && store.threadBrowserOpen[store.activeTab!]}>
                    <BrowserPanel threadId={store.activeTab!} />
                  </Show>
                  <Show when={store.activeTab && store.threadDiffOpen[store.activeTab!] && diffCwd()}>
                    <DiffEditor cwd={diffCwd()} prNumber={activePrNumber()} />
                  </Show>
                </div>
              </Show>
            </div>
          </div>
        }
      >
        <SplitView />
      </Show>

      <Show when={store.settingsOpen}>
        <SettingsOverlay />
      </Show>

      <Show when={store.providerPickerOpen}>
        <ProviderPicker />
      </Show>

      <Show when={store.contextMenu}>
        <ContextMenu />
      </Show>

      <Show when={store.commandPaletteOpen}>
        <CommandPalette />
      </Show>

      <Show when={store.searchOpen}>
        <SearchOverlay />
      </Show>

      <Show when={store.usageDashboardOpen}>
        <UsageDashboard />
      </Show>

      <Show when={store.themeOpen}>
        <ThemeSelector />
      </Show>

      <Show when={showWelcome()}>
        <WelcomeScreen onDismiss={() => setShowWelcome(false)} />
      </Show>

      <style>{`
        .error-boundary {
          position: fixed;
          inset: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--bg-base, #1a1a2e);
          z-index: 9999;
        }
        .error-boundary-content {
          max-width: 520px;
          width: 90%;
          padding: 32px;
          background: var(--bg-card, #22223a);
          border: 1px solid var(--border, #333);
          border-radius: 12px;
          text-align: center;
        }
        .error-boundary-title {
          font-size: 18px;
          font-weight: 600;
          color: var(--red, #f87171);
          margin: 0 0 12px;
        }
        .error-boundary-message {
          font-size: 14px;
          color: var(--text, #e0e0e0);
          margin: 0 0 16px;
          word-break: break-word;
        }
        .error-boundary-details {
          text-align: left;
          margin-bottom: 20px;
        }
        .error-boundary-details summary {
          cursor: pointer;
          font-size: 12px;
          color: var(--text-secondary, #999);
          margin-bottom: 8px;
        }
        .error-boundary-stack {
          font-size: 11px;
          color: var(--text-tertiary, #777);
          background: var(--bg-base, #1a1a2e);
          padding: 12px;
          border-radius: 6px;
          overflow-x: auto;
          max-height: 200px;
          white-space: pre-wrap;
          word-break: break-all;
        }
        .error-boundary-reload {
          padding: 10px 24px;
          background: var(--primary, #6b7cff);
          color: white;
          border: none;
          border-radius: 6px;
          font-size: 13px;
          font-weight: 600;
          cursor: pointer;
        }
        .error-boundary-reload:hover {
          filter: brightness(1.1);
        }
      `}</style>

      <style>{`
        .main-panel {
          flex: 1;
          display: flex;
          flex-direction: column;
          min-width: 0;
          background: var(--bg-base);
        }
        .main-panel-body {
          flex: 1;
          display: flex;
          flex-direction: row;
          min-height: 0;
          overflow: hidden;
        }
        .main-panel-body.vertical {
          flex-direction: column;
        }
        .main-panel-body.dragging {
          user-select: none;
          cursor: col-resize;
        }
        .main-panel-body.dragging.vertical {
          cursor: row-resize;
        }
        .main-panel-body.dragging .main-panel-chat,
        .main-panel-body.dragging .main-panel-side {
          pointer-events: none;
          will-change: width, height;
        }
        .main-panel-chat {
          display: flex;
          flex-direction: column;
          min-width: 0;
          min-height: 0;
          overflow: hidden;
        }

        /* ── Draggable pane divider ── */
        .pane-divider {
          flex-shrink: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: col-resize;
          width: 5px;
          z-index: 2;
        }
        .pane-divider.vertical {
          cursor: row-resize;
          width: auto;
          height: 5px;
        }
        .pane-divider-line {
          background: var(--border);
          transition: background 0.15s;
        }
        .pane-divider:not(.vertical) .pane-divider-line {
          width: 1px;
          height: 100%;
        }
        .pane-divider.vertical .pane-divider-line {
          height: 1px;
          width: 100%;
        }
        .pane-divider:hover .pane-divider-line,
        .main-panel-body.dragging .pane-divider-line {
          background: var(--primary);
        }

        .main-panel-side {
          display: flex;
          flex-direction: column;
          min-height: 0;
          min-width: 0;
          overflow: hidden;
        }
        .main-panel-side > * {
          flex: 1;
          min-height: 0;
          overflow: auto;
        }
        .main-panel-side > * + * {
          border-top: 1px solid var(--border);
        }
      `}</style>
    </>
    </ErrorBoundary>
  );
}
