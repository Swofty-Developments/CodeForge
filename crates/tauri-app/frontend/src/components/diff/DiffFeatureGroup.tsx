/* One feature accordion in diff review: name, shared indicator, per-group +/-
 * counts and file count. Body (grid 0fr→1fr) holds the file rows, latched so a
 * never-opened group never renders its diffs. Unmapped group reads muted. */

import { For, Show, createSignal } from "solid-js";
import type { FeatureDiffGroup } from "../../types";
import { FileDiffRow } from "./FileDiffRow";
import { groupCounts, isUnmapped } from "./diff-utils";

export function DiffFeatureGroup(props: { group: FeatureDiffGroup; defaultOpen: boolean }) {
  const [open, setOpen] = createSignal(props.defaultOpen);
  const [mounted, setMounted] = createSignal(props.defaultOpen);

  function toggle() {
    if (!open()) setMounted(true);
    setOpen(!open());
  }

  const counts = () => groupCounts(props.group);
  const unmapped = () => isUnmapped(props.group);
  const fileCount = () => props.group.files.length;

  return (
    <div class="dfg" classList={{ "dfg--unmapped": unmapped() }}>
      <button class="dfg-head" aria-expanded={open()} onClick={toggle}>
        <svg
          class="dfg-chevron"
          classList={{ "dfg-chevron--open": open() }}
          width="12"
          height="12"
          viewBox="0 0 24 24"
          fill="none"
          stroke="currentColor"
          stroke-width="2.5"
          stroke-linecap="round"
          stroke-linejoin="round"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        <span class="dfg-name">{props.group.name}</span>
        <Show when={props.group.shared}>
          <span class="dfg-shared tint-purple">shared</span>
        </Show>
        <span class="dfg-spacer" />
        <span class="dfg-counts">
          <Show when={counts().additions > 0}>
            <span class="dfg-add">+{counts().additions}</span>
          </Show>
          <Show when={counts().deletions > 0}>
            <span class="dfg-del">−{counts().deletions}</span>
          </Show>
        </span>
        <span class="dfg-filecount">
          {fileCount()} file{fileCount() === 1 ? "" : "s"}
        </span>
      </button>
      <div class="dfg-body" classList={{ "dfg-body--open": open() }}>
        <div class="dfg-body-inner">
          <Show when={mounted()}>
            <For each={props.group.files}>
              {(file, i) => <FileDiffRow file={file} defaultOpen={props.defaultOpen && i() < 3} />}
            </For>
          </Show>
        </div>
      </div>
    </div>
  );
}
