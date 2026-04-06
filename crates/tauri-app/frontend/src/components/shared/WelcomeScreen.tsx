import { createSignal, onMount, onCleanup } from "solid-js";

interface WelcomeScreenProps {
  onDismiss: () => void;
}

export function WelcomeScreen(props: WelcomeScreenProps) {
  const [phase, setPhase] = createSignal<"enter" | "visible" | "exit">("enter");
  const tagline = "forge your code";
  const [typedCount, setTypedCount] = createSignal(0);
  let typingTimer: number | undefined;
  let dismissTimer: number | undefined;

  onMount(() => {
    setTimeout(() => setPhase("visible"), 100);
    setTimeout(() => {
      let i = 0;
      typingTimer = window.setInterval(() => {
        i++;
        setTypedCount(i);
        if (i >= tagline.length) clearInterval(typingTimer);
      }, 55);
    }, 900);
    dismissTimer = window.setTimeout(dismiss, 3000);
  });

  onCleanup(() => {
    clearInterval(typingTimer);
    clearTimeout(dismissTimer);
  });

  function dismiss() {
    if (phase() === "exit") return;
    clearTimeout(dismissTimer);
    setPhase("exit");
    setTimeout(() => props.onDismiss(), 450);
  }

  return (
    <div class={`ws ${phase()}`} onClick={dismiss}>
      {/* Radial heat glow */}
      <div class="ws-glow" />

      {/* Geometric grid lines */}
      <div class="ws-grid" />

      <div class="ws-content">
        {/* Anvil icon mark */}
        <div class="ws-mark">
          <svg width="56" height="56" viewBox="0 0 56 56" fill="none">
            <path d="M12 10L6 28L12 46" stroke="url(#wg1)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M44 10L50 28L44 46" stroke="url(#wg1)" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round" />
            <path d="M33 8L23 48" stroke="url(#wg2)" stroke-width="2.5" stroke-linecap="round" />
            <defs>
              <linearGradient id="wg1" x1="6" y1="10" x2="50" y2="46" gradientUnits="userSpaceOnUse">
                <stop stop-color="#5568d9" />
                <stop offset="1" stop-color="#b47aff" />
              </linearGradient>
              <linearGradient id="wg2" x1="23" y1="48" x2="33" y2="8" gradientUnits="userSpaceOnUse">
                <stop stop-color="#6b7cff" />
                <stop offset="1" stop-color="#f07ab4" />
              </linearGradient>
            </defs>
          </svg>
        </div>

        <h1 class="ws-title">
          <span class="ws-t-code">Code</span><span class="ws-t-forge">Forge</span>
        </h1>

        <div class="ws-tagline">
          <span class="ws-typed">{tagline.slice(0, typedCount())}</span>
          <span class="ws-caret" classList={{ blink: typedCount() >= tagline.length }}>|</span>
        </div>

      </div>

      <style>{`
        .ws {
          position: fixed;
          inset: 0;
          z-index: 10000;
          display: flex;
          align-items: center;
          justify-content: center;
          background: var(--bg-base);
          cursor: pointer;
          overflow: hidden;
          opacity: 0;
          transition: opacity 0.4s ease, transform 0.45s cubic-bezier(0.4, 0, 0.2, 1);
        }
        .ws.visible, .ws.enter { opacity: 1; transform: scale(1); }
        .ws.visible { opacity: 1; }
        .ws.exit { opacity: 0; transform: scale(1.02); pointer-events: none; }

        .ws-glow {
          position: absolute;
          width: 600px;
          height: 600px;
          top: 50%;
          left: 50%;
          transform: translate(-50%, -50%);
          border-radius: 50%;
          background: radial-gradient(circle, rgba(107,124,255,0.12) 0%, rgba(180,122,255,0.06) 40%, transparent 70%);
          animation: ws-pulse 4s ease-in-out infinite alternate;
        }
        @keyframes ws-pulse {
          from { opacity: 0.6; transform: translate(-50%, -50%) scale(0.95); }
          to { opacity: 1; transform: translate(-50%, -50%) scale(1.1); }
        }

        .ws-grid {
          position: absolute;
          inset: 0;
          background-image:
            linear-gradient(rgba(255,255,255,0.018) 1px, transparent 1px),
            linear-gradient(90deg, rgba(255,255,255,0.018) 1px, transparent 1px);
          background-size: 80px 80px;
          mask-image: radial-gradient(ellipse 50% 50% at 50% 50%, black 10%, transparent 70%);
          -webkit-mask-image: radial-gradient(ellipse 50% 50% at 50% 50%, black 10%, transparent 70%);
        }

        .ws-content {
          position: relative;
          z-index: 1;
          display: flex;
          flex-direction: column;
          align-items: center;
          gap: 16px;
          opacity: 0;
          animation: ws-content-in 0.7s cubic-bezier(0.16, 1, 0.3, 1) 0.15s forwards;
        }
        @keyframes ws-content-in {
          from { opacity: 0; transform: translateY(16px); }
          to { opacity: 1; transform: translateY(0); }
        }

        .ws-mark {
          opacity: 0;
          animation: ws-mark-in 0.5s cubic-bezier(0.16, 1, 0.3, 1) 0.3s forwards;
        }
        @keyframes ws-mark-in {
          from { opacity: 0; transform: scale(0.7) rotate(-8deg); }
          to { opacity: 1; transform: scale(1) rotate(0deg); }
        }

        .ws-title {
          font-family: var(--font-body);
          font-size: 48px;
          font-weight: 700;
          letter-spacing: -2px;
          line-height: 1;
          opacity: 0;
          animation: ws-title-in 0.6s cubic-bezier(0.16, 1, 0.3, 1) 0.4s forwards;
        }
        @keyframes ws-title-in {
          from { opacity: 0; filter: blur(6px); transform: translateX(-8px); }
          to { opacity: 1; filter: blur(0); transform: translateX(0); }
        }

        .ws-t-code { color: var(--text); }
        .ws-t-forge { color: var(--primary); }

        .ws-tagline {
          font-family: var(--font-mono);
          font-size: 14px;
          color: var(--text-tertiary);
          letter-spacing: 0.5px;
          min-height: 22px;
        }
        .ws-typed { color: var(--text-secondary); }
        .ws-caret { color: var(--primary); font-weight: 300; margin-left: 1px; }
        .ws-caret.blink { animation: caret-b 0.7s step-end infinite; }
        @keyframes caret-b { 0%,100% { opacity: 1; } 50% { opacity: 0; } }

        .ws-sub {
          font-size: 11px;
          color: var(--text-tertiary);
          opacity: 0;
          animation: ws-hint-in 0.4s ease 2s forwards;
          letter-spacing: 0.03em;
        }
        @keyframes ws-hint-in { to { opacity: 0.6; } }
      `}</style>
    </div>
  );
}
