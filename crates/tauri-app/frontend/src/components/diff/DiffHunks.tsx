/* Renders a file's hunks: muted mono hunk headers, line-numbered old/new gutter,
 * +/- tinted rows with per-line syntax highlighting, and a system-pill marker
 * when the file exceeds the render budget. */

import { For, Match, Switch, createMemo } from "solid-js";
import type { FileDiff } from "../../types";
import { buildRows, langForPath, type DiffRow } from "./diff-utils";

export function DiffHunks(props: { file: FileDiff }) {
  const rows = createMemo(() => buildRows(props.file, langForPath(props.file.path)));

  return (
    <div class="dh">
      <For each={rows()}>{(row) => <Row row={row} />}</For>
    </div>
  );
}

function Row(props: { row: DiffRow }) {
  return (
    <Switch>
      <Match when={props.row.t === "header" ? props.row : null}>
        {(r) => <div class="dh-hunk-header">{r().header}</div>}
      </Match>
      <Match when={props.row.t === "trunc" ? props.row : null}>
        {(r) => (
          <div class="dh-trunc">
            <span class="dh-trunc-pill">… {r().remaining} more line{r().remaining === 1 ? "" : "s"} hidden</span>
          </div>
        )}
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
