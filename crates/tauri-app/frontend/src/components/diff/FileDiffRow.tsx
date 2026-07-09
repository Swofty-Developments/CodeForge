/* One file inside a feature group: status-letter chip, mono path, +/- counts,
 * collapsible (grid 0fr→1fr) hunk body. Default-open so the diff reads at a
 * glance; the hunk body is latched so it only mounts once expanded. */

import { Show, createSignal } from "solid-js";
import type { FileDiff } from "../../types";
import { DiffHunks } from "./DiffHunks";
import { splitPath, statusMeta } from "./diff-utils";

export function FileDiffRow(props: { file: FileDiff; defaultOpen: boolean }) {
  const [open, setOpen] = createSignal(props.defaultOpen);
  const [mounted, setMounted] = createSignal(props.defaultOpen);

  function toggle() {
    if (!open()) setMounted(true);
    setOpen(!open());
  }

  const meta = () => statusMeta(props.file.status);
  const parts = () => splitPath(props.file.path);

  return (
    <div class="dfr">
      <button class="dfr-head" aria-expanded={open()} onClick={toggle}>
        <span class={`dfr-status dfr-status--${meta().cls}`} title={meta().label}>
          {meta().letter}
        </span>
        <span class="dfr-path">
          <span class="dfr-dir">{parts().dir}</span>
          <span class="dfr-base">{parts().base}</span>
        </span>
        <span class="dfr-counts">
          <Show when={props.file.additions > 0}>
            <span class="dfr-add">+{props.file.additions}</span>
          </Show>
          <Show when={props.file.deletions > 0}>
            <span class="dfr-del">−{props.file.deletions}</span>
          </Show>
        </span>
        <svg
          class="dfr-chevron"
          classList={{ "dfr-chevron--open": open() }}
          width="10"
          height="10"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
      </button>
      <div class="dfr-body" classList={{ "dfr-body--open": open() }}>
        <div class="dfr-body-inner">
          <Show when={mounted()}>
            <DiffHunks file={props.file} />
          </Show>
        </div>
      </div>
    </div>
  );
}
