/* Slim session header (below the tab strip): the active session's model, its live
 * permission mode, and a per-turn run-state affordance ("generating…"). Read-only
 * context — the interactive mode switch lives in the composer, not here. */

import { Match, Switch } from "solid-js";
import { modeLabel } from "./ModeControl";
import type { SessionUi } from "../../types";

export function SessionHeader(props: { session: SessionUi }) {
  const s = () => props.session;
  return (
    <div class="sp-header">
      <svg class="sp-h-icon" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
        <path d="M4 17l6-6-6-6M12 19h8" />
      </svg>
      <span class="sp-h-model" title="Session model">{s().info.model ?? "default"}</span>
      <span class="sp-h-dot-sep" />
      <span class="sp-h-mode" title="Permission mode">{modeLabel(s().permissionMode)}</span>
      <div class="sp-h-spacer" />
      <Switch>
        <Match when={s().runState === "generating"}>
          <span class="sp-h-state sp-h-state--gen"><span class="sp-h-pulse" />generating…</span>
        </Match>
        <Match when={s().runState === "starting"}>
          <span class="sp-h-state sp-h-state--start"><span class="sp-h-pulse" />starting…</span>
        </Match>
        <Match when={s().runState === "error"}>
          <span class="sp-h-state sp-h-state--err">error</span>
        </Match>
        <Match when={s().runState === "ready"}>
          <span class="sp-h-state sp-h-state--ready">ready</span>
        </Match>
      </Switch>
    </div>
  );
}
