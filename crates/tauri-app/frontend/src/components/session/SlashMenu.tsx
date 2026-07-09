/* Slash-command popover (FZ-4): a filterable menu of the session's available
 * Claude Code slash commands, shown when the composer input is a single "/token".
 * Presentational — the Composer owns the selection index + keyboard nav. Styled
 * like the command palette (elevated surface, hairline, tight rows). */

import { For, Show } from "solid-js";

/** The sidecar only sends command names (`slashCommands: string[]`). Normalize a
 *  leading slash off, drop blanks, filter by the typed fragment, then rank
 *  prefix matches above substring matches. Names are shown verbatim otherwise. */
export function filterSlashCommands(commands: readonly string[], query: string): string[] {
  const q = query.toLowerCase();
  const names = commands
    .map((c) => (c.startsWith("/") ? c.slice(1) : c))
    .filter((n) => n.length > 0);
  const matched = names.filter((n) => n.toLowerCase().includes(q));
  return matched.sort((a, b) => {
    const ap = a.toLowerCase().startsWith(q) ? 0 : 1;
    const bp = b.toLowerCase().startsWith(q) ? 0 : 1;
    if (ap !== bp) return ap - bp;
    return a.localeCompare(b);
  });
}

export function SlashMenu(props: {
  items: string[];
  selected: number;
  onPick: (name: string) => void;
  onHover: (i: number) => void;
}) {
  return (
    <div class="slash-menu">
      <div class="slash-menu-label">Slash commands</div>
      <div class="slash-menu-list">
        <For each={props.items}>
          {(name, i) => (
            <button
              class="slash-item"
              classList={{ "slash-item--sel": props.selected === i() }}
              onMouseEnter={() => props.onHover(i())}
              // mousedown (not click) so the pick lands before the textarea blurs.
              onMouseDown={(e) => { e.preventDefault(); props.onPick(name); }}
            >
              <span class="slash-item-slash">/</span>
              <span class="slash-item-name">{name}</span>
            </button>
          )}
        </For>
        <Show when={props.items.length === 0}>
          <div class="slash-empty">No matching commands</div>
        </Show>
      </div>
    </div>
  );
}
