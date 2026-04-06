import { Show, ErrorBoundary, createSignal, onMount, onCleanup, createEffect, on } from "solid-js";
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
import { UpdateChecker } from "./components/shared/UpdateChecker";
import { KeyboardHelp } from "./components/shared/KeyboardHelp";
import { Breadcrumb } from "./components/chat/Breadcrumb";
import { StatusBar } from "./components/shared/StatusBar";
import { AnimatedShow } from "./components/shared/AnimatedShow";
import { RemotePollBanner } from "./components/shared/RemotePollBanner";
import * as ipc from "./ipc";

export function App() {
  const { store, setStore, closeTab, selectThread, newThread, sendUserMessage, reopenLastClosedTab } = appStore;
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

  // Poll PR statuses every 60s to detect merges, new CI results, review comments
  onMount(() => {
    // Initial poll after a short delay so the store is loaded
    const initialTimer = setTimeout(() => {
      appStore.pollPrStatuses().catch(() => {});
    }, 5000);

    const pollInterval = setInterval(() => {
      appStore.pollPrStatuses().catch(() => {});
    }, 60_000);

    onCleanup(() => {
      clearTimeout(initialTimer);
      clearInterval(pollInterval);
    });
  });

  // Side pane resize state (percentage of main-panel-body width/height for chat)
  const [chatPercent, setChatPercent] = createSignal(60);
  const [draggingDivider, setDraggingDivider] = createSignal(false);
  const [isVertical, setIsVertical] = createSignal(false);
  const [focusedPane, setFocusedPane] = createSignal<"chat" | "side">("chat");
  const [nearSnap, setNearSnap] = createSignal<number | null>(null);
  let bodyRef: HTMLDivElement | undefined;
  let chatPercentRef = 60;

  // Snap points for divider (percentage values)
  const SNAP_POINTS = [33.33, 50, 60, 66.67, 75];
  const SNAP_THRESHOLD = 2.5; // magnetic pull range in %

  // Spring physics solver for divider release
  function springAnimate(
    from: number,
    to: number,
    onUpdate: (v: number) => void,
    onDone: () => void
  ) {
    const tension = 300;
    const damping = 26;
    const mass = 1;
    let velocity = 0;
    let position = from;
    let raf: number;
    let lastTime = performance.now();

    function step(now: number) {
      const dt = Math.min((now - lastTime) / 1000, 0.032); // cap at ~30fps min
      lastTime = now;

      const displacement = position - to;
      const springForce = -tension * displacement;
      const dampingForce = -damping * velocity;
      const acceleration = (springForce + dampingForce) / mass;

      velocity += acceleration * dt;
      position += velocity * dt;

      onUpdate(position);

      if (Math.abs(velocity) < 0.1 && Math.abs(position - to) < 0.05) {
        onUpdate(to);
        onDone();
        return;
      }
      raf = requestAnimationFrame(step);
    }
    raf = requestAnimationFrame(step);
    return () => cancelAnimationFrame(raf);
  }

  // Find nearest snap point within threshold
  function findSnap(pct: number): number | null {
    for (const sp of SNAP_POINTS) {
      if (Math.abs(pct - sp) < SNAP_THRESHOLD) return sp;
    }
    return null;
  }

  // Format snap point as a human-readable ratio
  function formatSnapLabel(pct: number): string {
    const ratios: Record<number, string> = {
      33.33: "1/3 : 2/3",
      50: "1 : 1",
      60: "3 : 2",
      66.67: "2/3 : 1/3",
      75: "3 : 1",
    };
    return ratios[pct] ?? `${Math.round(pct)}%`;
  }

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

      // Show snap proximity feedback
      const snap = findSnap(pct);
      setNearSnap(snap);

      bodyRef!.style.setProperty("--chat-pct", `${pct}%`);
      bodyRef!.style.setProperty("--side-pct", `${100 - pct}%`);
      chatPercentRef = pct;
    };
    const onUp = () => {
      bodyRef?.classList.remove("dragging");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);

      setNearSnap(null);

      // Spring to nearest snap point if close, otherwise spring settle in place
      const snap = findSnap(chatPercentRef);
      const target = snap ?? chatPercentRef;

      if (Math.abs(chatPercentRef - target) < 0.1) {
        // No spring needed — already at target
        setChatPercent(target);
        return;
      }

      // Animate with spring physics
      bodyRef?.classList.add("spring-settling");
      springAnimate(
        chatPercentRef,
        target,
        (v) => {
          bodyRef?.style.setProperty("--chat-pct", `${v}%`);
          bodyRef?.style.setProperty("--side-pct", `${100 - v}%`);
        },
        () => {
          bodyRef?.classList.remove("spring-settling");
          setChatPercent(target);
        }
      );
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handleKeyDown(e: KeyboardEvent) {
    const mod = e.metaKey || e.ctrlKey;
    const key = e.key.toLowerCase();

    // Escape: close overlays in priority order
    if (key === "escape") {
      if (store.keyboardHelpOpen) { e.preventDefault(); setStore("keyboardHelpOpen", false); return; }
      if (store.commandPaletteOpen) { e.preventDefault(); setStore("commandPaletteOpen", false); return; }
      if (store.searchOpen) { e.preventDefault(); setStore("searchOpen", false); return; }
      if (store.usageDashboardOpen) { e.preventDefault(); setStore("usageDashboardOpen", false); return; }
      if (store.settingsOpen) { e.preventDefault(); setStore("settingsOpen", false); return; }
      if (store.themeOpen) { e.preventDefault(); setStore("themeOpen", false); return; }
      if (store.providerPickerOpen) { e.preventDefault(); setStore("providerPickerOpen", false); return; }
    }

    if (mod && key === "k" && !e.shiftKey) {
      e.preventDefault();
      setStore("commandPaletteOpen", !store.commandPaletteOpen);
    }
    if (mod && e.shiftKey && key === "f") {
      e.preventDefault();
      appStore.openVirtualTab("__search__");
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

    // Cmd+W: Close current tab
    if (mod && key === "w" && !e.shiftKey) {
      e.preventDefault();
      if (store.activeTab) closeTab(store.activeTab);
    }

    // Cmd+T: New thread
    if (mod && key === "t" && !e.shiftKey) {
      e.preventDefault();
      newThread();
    }

    // Cmd+N: New thread (alias)
    if (mod && key === "n" && !e.shiftKey) {
      e.preventDefault();
      newThread();
    }

    // Cmd+Shift+T: Reopen last closed tab
    if (mod && e.shiftKey && key === "t") {
      e.preventDefault();
      reopenLastClosedTab();
    }

    // Cmd+1 through Cmd+9: Switch to tab by position
    if (mod && !e.shiftKey && key >= "1" && key <= "9") {
      e.preventDefault();
      const idx = key === "9" ? store.openTabs.length - 1 : parseInt(key) - 1;
      const tabId = store.openTabs[idx];
      if (tabId) selectThread(tabId);
    }

    // Cmd+Enter: Force send message (even mid-generation)
    if (mod && key === "enter") {
      e.preventDefault();
      sendUserMessage();
    }

    // Cmd+.: Stop/interrupt generation
    if (mod && key === ".") {
      e.preventDefault();
      if (store.activeTab) {
        const status = store.runStates[store.activeTab];
        if (status === "generating" || status === "interrupting") {
          if (status === "interrupting") {
            ipc.stopSession(store.activeTab).catch(() => {});
            setStore("runStates", store.activeTab, "ready");
          } else {
            setStore("runStates", store.activeTab, "interrupting");
            ipc.interruptSession(store.activeTab).catch(() => {});
          }
        }
      }
    }

    // Cmd+,: Settings
    if (mod && key === ",") {
      e.preventDefault();
      setStore("settingsOpen", !store.settingsOpen);
    }

    // Cmd+? (Cmd+Shift+/): Keyboard help
    if (mod && e.shiftKey && (key === "/" || key === "?")) {
      e.preventDefault();
      setStore("keyboardHelpOpen", !store.keyboardHelpOpen);
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
            <RemotePollBanner />
            <Breadcrumb />
            <div
              ref={bodyRef}
              class="main-panel-body"
              classList={{
                vertical: isVertical(),
                /* dragging class added/removed via DOM directly for performance */
              }}
            >
              <div
                class="main-panel-chat"
                classList={{ focused: focusedPane() === "chat" && !!hasSidePane() }}
                style={hasSidePane() ? chatStyle() : { flex: "1" }}
                onFocusIn={() => setFocusedPane("chat")}
                onClick={() => setFocusedPane("chat")}
              >
                <ChatArea />
                <Composer />
                <StatusBar />
              </div>

              <AnimatedShow when={!!hasSidePane()} class="side-pane-animated" duration={220}>
                <div
                  class="pane-divider"
                  classList={{
                    vertical: isVertical(),
                    "near-snap": nearSnap() !== null,
                  }}
                  onMouseDown={handleDividerDown}
                >
                  <div class="pane-divider-line" />
                  <div class="pane-divider-grip">
                    <span /><span /><span />
                  </div>
                </div>
                <div
                  class="main-panel-side"
                  classList={{ focused: focusedPane() === "side" }}
                  style={sideStyle()}
                  onFocusIn={() => setFocusedPane("side")}
                  onClick={() => setFocusedPane("side")}
                >
                  <Show when={store.activeTab && store.threadBrowserOpen[store.activeTab!]}>
                    <BrowserPanel threadId={store.activeTab!} />
                  </Show>
                  <Show when={store.activeTab && store.threadDiffOpen[store.activeTab!] && diffCwd()}>
                    <DiffEditor cwd={diffCwd()} prNumber={activePrNumber()} />
                  </Show>
                </div>

                {/* Snap guide lines — visible during drag when near a snap point */}
                <Show when={nearSnap() !== null}>
                  <div
                    class="snap-guide"
                    style={{
                      [isVertical() ? "top" : "left"]: `${nearSnap()!}%`,
                      opacity: "1",
                    }}
                    classList={{ vertical: isVertical() }}
                  >
                    <span class="snap-guide-label">{formatSnapLabel(nearSnap()!)}</span>
                  </div>
                </Show>
              </AnimatedShow>
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

      <Show when={store.keyboardHelpOpen}>
        <KeyboardHelp />
      </Show>

      <Show when={showWelcome()}>
        <WelcomeScreen onDismiss={() => setShowWelcome(false)} />
      </Show>

      <UpdateChecker />

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
          position: relative;
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

        /* ── Direction C: Spatial depth — chat pane ── */
        .main-panel-chat {
          display: flex;
          flex-direction: column;
          min-width: 0;
          min-height: 0;
          overflow: hidden;
          transition: filter 0.2s ease, box-shadow 0.2s ease;
        }
        .main-panel-chat.focused {
          filter: brightness(1);
          box-shadow: 0 0 24px rgba(0, 0, 0, 0.15);
          z-index: 1;
        }

        /* ── Direction A: Side pane animated entrance/exit ── */
        .side-pane-animated {
          display: contents;
        }
        .side-pane-animated[data-state="entering"] .main-panel-side {
          animation: side-pane-in 220ms cubic-bezier(0.16, 1, 0.3, 1) both;
        }
        .side-pane-animated[data-state="exiting"] .main-panel-side {
          animation: side-pane-out 180ms ease-in both;
        }
        .side-pane-animated[data-state="entering"] .pane-divider {
          animation: fade-in 150ms ease-out both;
        }
        .side-pane-animated[data-state="exiting"] .pane-divider {
          animation: fade-in 100ms ease-in both reverse;
        }

        @keyframes side-pane-in {
          from {
            opacity: 0;
            transform: translateX(12px) scale(0.97);
            filter: blur(2px);
          }
          to {
            opacity: 1;
            transform: translateX(0) scale(1);
            filter: blur(0);
          }
        }
        @keyframes side-pane-out {
          from {
            opacity: 1;
            transform: translateX(0) scale(1);
          }
          to {
            opacity: 0;
            transform: translateX(8px) scale(0.98);
          }
        }
        /* Vertical variant */
        .main-panel-body.vertical .side-pane-animated[data-state="entering"] .main-panel-side {
          animation-name: side-pane-in-vert;
        }
        .main-panel-body.vertical .side-pane-animated[data-state="exiting"] .main-panel-side {
          animation-name: side-pane-out-vert;
        }
        @keyframes side-pane-in-vert {
          from { opacity: 0; transform: translateY(12px) scale(0.97); filter: blur(2px); }
          to { opacity: 1; transform: translateY(0) scale(1); filter: blur(0); }
        }
        @keyframes side-pane-out-vert {
          from { opacity: 1; transform: translateY(0) scale(1); }
          to { opacity: 0; transform: translateY(8px) scale(0.98); }
        }

        /* ── Direction B: Enhanced pane divider ── */
        .pane-divider {
          flex-shrink: 0;
          display: flex;
          align-items: center;
          justify-content: center;
          cursor: col-resize;
          width: 7px;
          z-index: 2;
          position: relative;
          transition: width 0.15s ease;
        }
        .pane-divider:hover {
          width: 9px;
        }
        .pane-divider.vertical {
          cursor: row-resize;
          width: auto;
          height: 7px;
          transition: height 0.15s ease;
        }
        .pane-divider.vertical:hover {
          height: 9px;
        }
        .pane-divider-line {
          background: var(--border);
          transition: background 0.15s, box-shadow 0.15s;
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
          box-shadow: 0 0 8px var(--primary-glow);
        }
        /* Snap proximity glow — amber to distinguish from normal drag blue */
        .pane-divider.near-snap .pane-divider-line {
          background: var(--amber);
          box-shadow: 0 0 10px rgba(240, 184, 64, 0.3), 0 0 4px rgba(240, 184, 64, 0.2);
        }

        /* Grip dots — visible on hover */
        .pane-divider-grip {
          position: absolute;
          display: flex;
          gap: 3px;
          opacity: 0;
          transition: opacity 0.15s;
        }
        .pane-divider:not(.vertical) .pane-divider-grip {
          flex-direction: column;
        }
        .pane-divider:hover .pane-divider-grip,
        .main-panel-body.dragging .pane-divider-grip {
          opacity: 0.4;
        }
        .pane-divider-grip span {
          width: 3px;
          height: 3px;
          border-radius: 50%;
          background: var(--text-tertiary);
        }

        /* ── Direction B: Snap guide line ── */
        .snap-guide {
          position: absolute;
          top: 0;
          bottom: 0;
          width: 1px;
          pointer-events: none;
          z-index: 3;
          opacity: 0;
          transition: opacity 0.12s;
          background: repeating-linear-gradient(
            to bottom,
            var(--amber) 0px,
            var(--amber) 4px,
            transparent 4px,
            transparent 8px
          );
        }
        .snap-guide.vertical {
          top: auto;
          left: 0;
          right: 0;
          width: auto;
          height: 1px;
          background: repeating-linear-gradient(
            to right,
            var(--amber) 0px,
            var(--amber) 4px,
            transparent 4px,
            transparent 8px
          );
        }
        .snap-guide-label {
          position: absolute;
          top: 8px;
          left: 50%;
          transform: translateX(-50%);
          font-size: 9px;
          font-family: var(--font-mono);
          font-weight: 600;
          color: var(--amber);
          background: var(--bg-card);
          border: 1px solid rgba(240, 184, 64, 0.25);
          border-radius: 3px;
          padding: 1px 5px;
          white-space: nowrap;
          pointer-events: none;
        }

        /* ── Direction C: Spatial depth — side pane ── */
        .main-panel-side {
          display: flex;
          flex-direction: column;
          min-height: 0;
          min-width: 0;
          overflow: hidden;
          transition: filter 0.2s ease, box-shadow 0.2s ease;
        }
        .main-panel-side:not(.focused) {
          filter: brightness(0.95);
        }
        .main-panel-side.focused {
          filter: brightness(1);
          box-shadow: 0 0 24px rgba(0, 0, 0, 0.15);
          z-index: 1;
        }
        .main-panel-side > * {
          flex: 1;
          min-height: 0;
          overflow: auto;
        }
        .main-panel-side > * + * {
          border-top: 1px solid var(--border);
        }

        /* When there's no side pane, remove depth effects from chat */
        .main-panel-chat:not(.focused) {
          filter: brightness(0.95);
        }

        /* Spring settling — keep will-change for smoother animation */
        .main-panel-body.spring-settling .main-panel-chat,
        .main-panel-body.spring-settling .main-panel-side {
          will-change: width, height;
        }

        /* ── prefers-reduced-motion ── */
        @media (prefers-reduced-motion: reduce) {
          .side-pane-animated[data-state="entering"] .main-panel-side,
          .side-pane-animated[data-state="exiting"] .main-panel-side,
          .side-pane-animated[data-state="entering"] .pane-divider,
          .side-pane-animated[data-state="exiting"] .pane-divider {
            animation: none !important;
          }
          .main-panel-chat,
          .main-panel-side,
          .pane-divider,
          .pane-divider-line {
            transition: none !important;
          }
        }
      `}</style>
    </>
    </ErrorBoundary>
  );
}
