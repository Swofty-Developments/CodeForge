import { createSignal, onCleanup, onMount, Show } from "solid-js";
import { appStore } from "../../stores/app-store";
import { SplitPane } from "./SplitPane";

/**
 * SplitView renders two thread panes side by side with a draggable divider.
 * Features: spring physics on release, snap points, grip texture, depth cues.
 */
export function SplitView() {
  const { store, setSplitTab } = appStore;

  const [splitPercent, setSplitPercent] = createSignal(50);
  const [dragging, setDragging] = createSignal(false);
  const [focusedSplit, setFocusedSplit] = createSignal<"left" | "right">("left");
  const [nearSnap, setNearSnap] = createSignal<number | null>(null);
  const [entered, setEntered] = createSignal(false);
  let containerRef: HTMLDivElement | undefined;
  let splitPercentRef = 50;

  const SNAP_POINTS = [25, 33.33, 50, 66.67, 75];
  const SNAP_THRESHOLD = 2.5;

  function formatSnapLabel(pct: number): string {
    const ratios: Record<number, string> = {
      25: "1 : 3",
      33.33: "1 : 2",
      50: "1 : 1",
      66.67: "2 : 1",
      75: "3 : 1",
    };
    return ratios[pct] ?? `${Math.round(pct)}%`;
  }

  function findSnap(pct: number): number | null {
    for (const sp of SNAP_POINTS) {
      if (Math.abs(pct - sp) < SNAP_THRESHOLD) return sp;
    }
    return null;
  }

  function springAnimate(from: number, to: number, onUpdate: (v: number) => void, onDone: () => void) {
    const tension = 300, damping = 26, mass = 1;
    let velocity = 0, position = from, lastTime = performance.now();
    function step(now: number) {
      const dt = Math.min((now - lastTime) / 1000, 0.032);
      lastTime = now;
      const springForce = -tension * (position - to);
      const dampingForce = -damping * velocity;
      velocity += ((springForce + dampingForce) / mass) * dt;
      position += velocity * dt;
      onUpdate(position);
      if (Math.abs(velocity) < 0.1 && Math.abs(position - to) < 0.05) { onUpdate(to); onDone(); return; }
      requestAnimationFrame(step);
    }
    requestAnimationFrame(step);
  }

  // Animate entrance
  onMount(() => {
    requestAnimationFrame(() => requestAnimationFrame(() => setEntered(true)));
  });

  function handleMouseDown(e: MouseEvent) {
    e.preventDefault();
    if (!containerRef) return;
    const rect = containerRef.getBoundingClientRect();

    setDragging(true);
    containerRef.classList.add("split-dragging");
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";

    const onMove = (ev: MouseEvent) => {
      const x = ev.clientX - rect.left;
      const pct = Math.min(Math.max((x / rect.width) * 100, 15), 85);
      setNearSnap(findSnap(pct));
      containerRef!.style.setProperty("--split-left", `${pct}%`);
      containerRef!.style.setProperty("--split-right", `${100 - pct}%`);
      splitPercentRef = pct;
    };
    const onUp = () => {
      setDragging(false);
      containerRef?.classList.remove("split-dragging");
      document.body.style.cursor = "";
      document.body.style.userSelect = "";
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      setNearSnap(null);

      const snap = findSnap(splitPercentRef);
      const target = snap ?? splitPercentRef;
      if (Math.abs(splitPercentRef - target) < 0.1) { setSplitPercent(target); return; }
      springAnimate(splitPercentRef, target,
        (v) => {
          containerRef?.style.setProperty("--split-left", `${v}%`);
          containerRef?.style.setProperty("--split-right", `${100 - v}%`);
        },
        () => setSplitPercent(target)
      );
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  // Keyboard shortcut: Cmd+\ to toggle split view
  function handleKeyDown(e: KeyboardEvent) {
    if ((e.metaKey || e.ctrlKey) && e.key === "\\") {
      e.preventDefault();
      setSplitTab(null);
    }
  }

  onMount(() => document.addEventListener("keydown", handleKeyDown));
  onCleanup(() => document.removeEventListener("keydown", handleKeyDown));

  return (
    <div
      class="split-view-container"
      ref={containerRef}
      classList={{ "split-entered": entered() }}
      style={{
        "--split-left": `${splitPercent()}%`,
        "--split-right": `${100 - splitPercent()}%`,
      }}
    >
      <div
        class="split-view-pane"
        classList={{ focused: focusedSplit() === "left" }}
        style={{ width: "var(--split-left)" }}
        onFocusIn={() => setFocusedSplit("left")}
        onClick={() => setFocusedSplit("left")}
      >
        <Show when={store.activeTab}>
          <SplitPane threadId={store.activeTab!} />
        </Show>
      </div>

      <div
        class="split-view-divider"
        classList={{ "near-snap": nearSnap() !== null }}
        onMouseDown={handleMouseDown}
      >
        <div class="split-view-divider-line" />
        <div class="split-view-divider-grip">
          <span /><span /><span />
        </div>
      </div>

      <div
        class="split-view-pane"
        classList={{ focused: focusedSplit() === "right" }}
        style={{ width: "var(--split-right)" }}
        onFocusIn={() => setFocusedSplit("right")}
        onClick={() => setFocusedSplit("right")}
      >
        <Show when={store.splitTab}>
          <div class="split-view-pane-wrapper">
            <SplitPane threadId={store.splitTab!} />
            <button
              class="split-view-close"
              onClick={() => setSplitTab(null)}
              title="Close Split View (Cmd+\\)"
            >
              Close Split
            </button>
          </div>
        </Show>
      </div>

      {/* Snap guide */}
      <Show when={nearSnap() !== null}>
        <div
          class="split-snap-guide"
          style={{ left: `${nearSnap()!}%` }}
        >
          <span class="split-snap-label">{formatSnapLabel(nearSnap()!)}</span>
        </div>
      </Show>
    </div>
  );
}

if (!document.getElementById("split-view-styles")) {
  const style = document.createElement("style");
  style.id = "split-view-styles";
  style.textContent = `
    .split-view-container {
      display: flex;
      flex: 1;
      min-height: 0;
      overflow: hidden;
      background: var(--bg-base, #131316);
      position: relative;
    }
    .split-view-container.split-dragging {
      cursor: col-resize;
      user-select: none;
    }
    .split-view-container.split-dragging .split-view-pane {
      pointer-events: none;
      will-change: width;
    }

    /* Direction A: Animated entrance */
    .split-view-pane {
      display: flex;
      flex-direction: column;
      min-width: 0;
      overflow: hidden;
      position: relative;
      opacity: 0;
      transform: scale(0.98);
      transition: opacity 250ms cubic-bezier(0.16, 1, 0.3, 1),
                  transform 250ms cubic-bezier(0.16, 1, 0.3, 1),
                  filter 0.2s ease, box-shadow 0.2s ease;
    }
    .split-entered .split-view-pane {
      opacity: 1;
      transform: scale(1);
    }
    .split-entered .split-view-pane:first-child {
      transition-delay: 0ms;
    }
    .split-entered .split-view-pane:last-of-type {
      transition-delay: 60ms;
    }

    /* Direction C: Depth — focused pane lifts */
    .split-view-pane.focused {
      filter: brightness(1);
      box-shadow: 0 0 24px rgba(0, 0, 0, 0.15);
      z-index: 1;
    }
    .split-view-pane:not(.focused) {
      filter: brightness(0.95);
    }

    .split-view-pane-wrapper {
      display: flex;
      flex-direction: column;
      height: 100%;
      position: relative;
    }

    /* Direction B: Enhanced divider */
    .split-view-divider {
      width: 7px;
      cursor: col-resize;
      flex-shrink: 0;
      display: flex;
      align-items: center;
      justify-content: center;
      position: relative;
      z-index: 2;
      transition: width 0.15s ease;
    }
    .split-view-divider:hover {
      width: 9px;
    }
    .split-view-divider-line {
      width: 1px;
      height: 100%;
      background: rgba(255,255,255,0.07);
      transition: background 0.15s, box-shadow 0.15s;
    }
    .split-view-divider:hover .split-view-divider-line,
    .split-dragging .split-view-divider-line {
      background: var(--primary, #6680f2);
      box-shadow: 0 0 8px rgba(107, 124, 255, 0.2);
    }
    .split-view-divider.near-snap .split-view-divider-line {
      background: #f0b840;
      box-shadow: 0 0 10px rgba(240, 184, 64, 0.3), 0 0 4px rgba(240, 184, 64, 0.2);
    }

    /* Grip dots */
    .split-view-divider-grip {
      position: absolute;
      display: flex;
      flex-direction: column;
      gap: 3px;
      opacity: 0;
      transition: opacity 0.15s;
    }
    .split-view-divider:hover .split-view-divider-grip,
    .split-dragging .split-view-divider-grip {
      opacity: 0.4;
    }
    .split-view-divider-grip span {
      width: 3px;
      height: 3px;
      border-radius: 50%;
      background: rgba(255, 255, 255, 0.5);
    }

    /* Snap guide — amber dashed line with ratio label */
    .split-snap-guide {
      position: absolute;
      top: 0;
      bottom: 0;
      width: 1px;
      pointer-events: none;
      z-index: 3;
      animation: snap-guide-in 120ms ease-out both;
      background: repeating-linear-gradient(
        to bottom,
        #f0b840 0px, #f0b840 4px,
        transparent 4px, transparent 8px
      );
    }
    .split-snap-label {
      position: absolute;
      top: 8px;
      left: 50%;
      transform: translateX(-50%);
      font-size: 9px;
      font-family: var(--font-mono, monospace);
      font-weight: 600;
      color: #f0b840;
      background: var(--bg-card, #151518);
      border: 1px solid rgba(240, 184, 64, 0.25);
      border-radius: 3px;
      padding: 1px 5px;
      white-space: nowrap;
      pointer-events: none;
    }
    @keyframes snap-guide-in {
      from { opacity: 0; transform: scaleY(0.8); }
      to { opacity: 1; transform: scaleY(1); }
    }

    .split-view-close {
      position: absolute;
      top: 6px;
      right: 8px;
      z-index: 5;
      font-size: 11px;
      color: rgba(255,255,255,0.5);
      background: rgba(255,255,255,0.06);
      border: 1px solid rgba(255,255,255,0.1);
      border-radius: 6px;
      padding: 3px 10px;
      cursor: pointer;
      transition: all 0.15s;
    }
    .split-view-close:hover {
      background: rgba(255,255,255,0.1);
      color: rgba(255,255,255,0.8);
      border-color: rgba(255,255,255,0.2);
    }

    /* prefers-reduced-motion */
    @media (prefers-reduced-motion: reduce) {
      .split-view-pane {
        opacity: 1 !important;
        transform: none !important;
        transition: none !important;
      }
      .split-view-divider,
      .split-view-divider-line {
        transition: none !important;
      }
    }
  `;
  document.head.appendChild(style);
}
