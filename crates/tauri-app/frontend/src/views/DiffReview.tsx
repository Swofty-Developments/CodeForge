/* Diff review view — pending changes grouped by feature, not by file. Sticky
 * header with a refresh control + total +/- counts; one accordion per feature
 * group with the unmapped bucket muted and always last. First group auto-open. */

import { For, Show, createMemo, createSignal } from "solid-js";
import { appStore } from "../stores/app-store";
import { DiffFeatureGroup } from "../components/diff/DiffFeatureGroup";
import { injectDiffStyles, orderedGroups, totalCounts } from "../components/diff/diff-utils";

export function DiffReview() {
  injectDiffStyles();
  const { store } = appStore;
  const [refreshing, setRefreshing] = createSignal(false);

  const groups = createMemo(() => orderedGroups(store.diff?.groups ?? []));
  const totals = createMemo(() => totalCounts(store.diff?.groups ?? []));

  async function refresh() {
    if (refreshing()) return;
    setRefreshing(true);
    try {
      await appStore.refreshDiff();
    } finally {
      setRefreshing(false);
    }
  }

  return (
    <div class="drv">
      <div class="drv-header">
        <span class="drv-title">Changes</span>
        <div class="drv-totals">
          <span class="drv-total-add">+{totals().additions}</span>
          <span class="drv-total-del">−{totals().deletions}</span>
        </div>
        <span class="drv-spacer" />
        <button
          class="drv-refresh"
          classList={{ "drv-refresh--busy": refreshing() }}
          onClick={() => void refresh()}
          title="Refresh diff"
        >
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round">
            <path d="M23 4v6h-6M1 20v-6h6" />
            <path d="M3.51 9a9 9 0 0 1 14.85-3.36L23 10M1 14l4.64 4.36A9 9 0 0 0 20.49 15" />
          </svg>
          Refresh
        </button>
      </div>

      <Show
        when={groups().length > 0}
        fallback={
          <div class="drv-empty-wrap">
            <div class="drv-empty">
              <div class="drv-empty-title">Working tree clean.</div>
              <div class="drv-empty-sub">Changes show up here grouped by feature the moment the working tree moves.</div>
            </div>
          </div>
        }
      >
        <div class="drv-list">
          <For each={groups()}>
            {(group, i) => <DiffFeatureGroup group={group} defaultOpen={i() === 0} />}
          </For>
        </div>
      </Show>
    </div>
  );
}
