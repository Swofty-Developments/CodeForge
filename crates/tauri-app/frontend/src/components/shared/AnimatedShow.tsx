import { createSignal, createEffect, on, JSX, Show } from "solid-js";

/**
 * A wrapper around Solid's <Show> that keeps children mounted during exit
 * animations. Children get a `data-state="entering"|"entered"|"exiting"` attribute
 * on their wrapper div, which CSS can target for transitions.
 *
 * Usage:
 *   <AnimatedShow when={open()} class="my-pane" duration={200}>
 *     <MyContent />
 *   </AnimatedShow>
 *
 * CSS:
 *   .my-pane[data-state="entering"] { ... }
 *   .my-pane[data-state="entered"]  { ... }
 *   .my-pane[data-state="exiting"]  { ... }
 */
export function AnimatedShow(props: {
  when: boolean;
  children: JSX.Element;
  class?: string;
  style?: JSX.CSSProperties;
  duration?: number;
}) {
  const duration = () => props.duration ?? 200;
  const [mounted, setMounted] = createSignal(props.when);
  const [state, setState] = createSignal<"entering" | "entered" | "exiting">(
    props.when ? "entered" : "exiting"
  );

  let exitTimer: ReturnType<typeof setTimeout> | null = null;

  createEffect(
    on(
      () => props.when,
      (show) => {
        if (show) {
          // Cancel any pending exit — prevents race when toggling quickly
          if (exitTimer !== null) { clearTimeout(exitTimer); exitTimer = null; }
          setMounted(true);
          // Force a frame so the DOM renders at "entering" before transitioning
          requestAnimationFrame(() => {
            requestAnimationFrame(() => setState("entered"));
          });
          setState("entering");
        } else {
          setState("exiting");
          exitTimer = setTimeout(() => { setMounted(false); exitTimer = null; }, duration());
        }
      }
    )
  );

  // Respect prefers-reduced-motion
  const prefersReduced =
    typeof window !== "undefined" &&
    window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  return (
    <Show when={mounted()}>
      <div
        class={props.class}
        style={props.style}
        data-state={prefersReduced ? "entered" : state()}
      >
        {props.children}
      </div>
    </Show>
  );
}
