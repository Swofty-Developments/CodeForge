/* Recursive feature-tree node (FZ-1): a collapsible group header (chevron + name
 * + subtree feature count) over its child groups and feature rows. Nested levels
 * indent via the `.ftg-body-inner` guide rail. Collapse state lives in the
 * sidebar-local `collapse` store; row styling is defined in Sidebar. */

import { For } from "solid-js";
import type { FeatureTreeNode } from "./tree";
import { FeatureRow } from "./FeatureRow";
import { collapsedGroups, toggleGroup } from "./collapse";

export interface TreeCtx {
  activity: (slug: string) => number;
  selected: () => string | null;
  onSelect: (slug: string) => void;
  onTogglePin: (slug: string, pinned: boolean) => void;
}

export function GroupNode(props: { node: FeatureTreeNode; ctx: TreeCtx }) {
  const open = () => !collapsedGroups[props.node.path];

  return (
    <div class="ftg" classList={{ "ftg--ungrouped": props.node.ungrouped }}>
      <button
        class="ftg-header"
        onClick={() => toggleGroup(props.node.path)}
        title={props.node.ungrouped ? "Features with no group" : props.node.path}
      >
        <svg
          class="ftg-chevron"
          classList={{ "ftg-chevron--open": open() }}
          width="9" height="9" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.6"
        >
          <path d="M9 18l6-6-6-6" />
        </svg>
        <span class="ftg-name">{props.node.segment}</span>
        <span class="ftg-count">{props.node.count}</span>
      </button>
      <div class="ftg-body" classList={{ "ftg-body--open": open() }}>
        <div class="ftg-body-inner">
          <For each={props.node.children}>
            {(child) => <GroupNode node={child} ctx={props.ctx} />}
          </For>
          <For each={props.node.features}>
            {(feature, i) => (
              <FeatureRow
                feature={feature}
                active={props.ctx.selected() === feature.slug}
                activity={props.ctx.activity(feature.slug)}
                index={i()}
                onSelect={props.ctx.onSelect}
                onTogglePin={props.ctx.onTogglePin}
              />
            )}
          </For>
        </div>
      </div>
    </div>
  );
}
