/* Collapsible section — the shared 0fr→1fr grid-rows collapse with a rotating
 * chevron. Header shows a label, an optional mono count, and optional trailing
 * content (e.g. a section action). */

import { type JSX, createSignal } from "solid-js";

export function CollapsibleSection(props: {
  label: string;
  count?: number;
  defaultOpen?: boolean;
  trailing?: JSX.Element;
  children: JSX.Element;
}) {
  const [open, setOpen] = createSignal(props.defaultOpen ?? true);

  return (
    <section class="fs">
      <div class="fs-head">
        <button class="fs-toggle" onClick={() => setOpen((v) => !v)} aria-expanded={open()}>
          <svg
            class="fs-chevron"
            classList={{ "fs-chevron--open": open() }}
            width="10" height="10" viewBox="0 0 24 24"
            fill="none" stroke="currentColor" stroke-width="2.4"
          >
            <path d="M9 6l6 6-6 6" />
          </svg>
          <span class="section-label fs-label">{props.label}</span>
          {props.count !== undefined && <span class="fs-count">{props.count}</span>}
        </button>
        {props.trailing}
      </div>
      <div class="fs-body" classList={{ "fs-body--open": open() }}>
        <div class="fs-body-inner">{props.children}</div>
      </div>

      <style>{`
        .fs { margin-top: var(--space-5); }
        .fs-head { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); }
        .fs-toggle {
          display: flex; align-items: center; gap: 6px;
          padding: 2px 0; min-width: 0;
          color: var(--text-tertiary);
        }
        .fs-toggle:hover { color: var(--text-secondary); }
        .fs-chevron { flex-shrink: 0; transition: transform 0.18s ease; }
        .fs-chevron--open { transform: rotate(90deg); }
        .fs-label { color: inherit; }
        .fs-count { font-size: 10px; font-family: var(--font-mono); color: var(--text-tertiary); }

        .fs-body {
          display: grid; grid-template-rows: 0fr;
          transition: grid-template-rows 0.22s var(--ease-out);
        }
        .fs-body--open { grid-template-rows: 1fr; }
        .fs-body-inner { overflow: hidden; padding-top: var(--space-2); }

        @media (prefers-reduced-motion: reduce) {
          .fs-body { transition: none; }
        }
      `}</style>
    </section>
  );
}
