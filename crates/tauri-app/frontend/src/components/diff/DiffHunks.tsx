/* Renders a file's hunks: muted mono hunk headers, line-numbered old/new gutter,
 * +/- tinted rows with per-line syntax highlighting. Binary files and
 * backend-truncated diffs surface as explicit typed markers (FileDiff.binary /
 * FileDiff.truncated), never silently omitted or re-capped. */

import { For, Match, Show, Switch, createMemo } from "solid-js";
import type { FileDiff } from "../../types";
import { buildRows, langForPath, type DiffRow } from "./diff-utils";

export function DiffHunks(props: { file: FileDiff }) {
  const rows = createMemo(() => (props.file.binary ? [] : buildRows(props.file, langForPath(props.file.path))));

  return (
    <div class="dh">
      <Show when={props.file.binary}>
        <div class="dh-binary">Binary file — not shown</div>
      </Show>
      <For each={rows()}>{(row) => <Row row={row} />}</For>
      <Show when={props.file.truncated}>
        <div class="dh-trunc">
          <span class="dh-trunc-pill">Diff truncated at the server line limit</span>
        </div>
      </Show>
    </div>
  );
}

function Row(props: { row: DiffRow }) {
  return (
    <Switch>
      <Match when={props.row.t === "header" ? props.row : null}>
        {(r) => <div class="dh-hunk-header">{r().header}</div>}
      </Match>
      <Match when={props.row.t === "line" ? props.row : null}>
        {(r) => (
          <div
            class="dh-line"
            classList={{ "dh-line--add": r().origin === "+", "dh-line--remove": r().origin === "-" }}
          >
            <span class="dh-gutter">{r().oldNo ?? ""}</span>
            <span class="dh-gutter">{r().newNo ?? ""}</span>
            <span class="dh-prefix">{r().origin === " " ? "" : r().origin}</span>
            <span class="dh-content" innerHTML={r().html} />
          </div>
        )}
      </Match>
    </Switch>
  );
}
