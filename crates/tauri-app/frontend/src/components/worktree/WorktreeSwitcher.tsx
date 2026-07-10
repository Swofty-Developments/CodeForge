/* WorktreeSwitcher — the tab-strip "+" / TitleBar "Worktrees…" picker. One
 * filter input over four named sections (Open / Worktrees / Branches / Remote)
 * plus a create row for novel names. Rendered through a Portal to <body> with
 * FIXED positioning: the tab strip is overflow-x:auto and a scroll container
 * clips absolutely-positioned descendants (the NewWorktreePrompt bug). Every
 * state is named — loading, list errors, and add failures all render. */

import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import { Portal } from "solid-js/web";
import * as ipc from "../../ipc";
import { appStore } from "../../stores/app-store";
import { samePath } from "../../stores/path";
import type { BranchInfo, Worktree } from "../../types";
import { buildSections, hasExactBranch } from "./switcher-data";
import { BranchRow, ContextRow, CreateRow, WorktreeRow } from "./SwitcherRows";
import { SWITCHER_CSS } from "./switcher-css";

const POP_WIDTH = 320;

export function WorktreeSwitcher(props: { anchor: () => HTMLElement | undefined }) {
  const { store } = appStore;

  const [query, setQuery] = createSignal("");
  const [branches, setBranches] = createSignal<BranchInfo[] | null>(null);
  const [branchesLoading, setBranchesLoading] = createSignal(false);
  const [branchesError, setBranchesError] = createSignal<string | null>(null);
  const [fetching, setFetching] = createSignal(false);
  /** "<remote>:<name>" of the branch row currently being checked out. */
  const [busyKey, setBusyKey] = createSignal<string | null>(null);
  /** Inline surface for add/fetch failures (also toasted by the store). */
  const [actionError, setActionError] = createSignal<string | null>(null);

  const [pos, setPos] = createSignal({ left: 8, top: 64, maxH: 480 });
  let inputRef: HTMLInputElement | undefined;

  function place(): void {
    const a = props.anchor();
    if (!a) return;
    const r = a.getBoundingClientRect();
    const left = Math.min(Math.max(8, r.left), window.innerWidth - POP_WIDTH - 8);
    const top = r.bottom + 6;
    setPos({ left, top, maxH: Math.max(160, window.innerHeight - top - 12) });
  }

  async function loadBranches(): Promise<void> {
    const repo = store.repo;
    if (!repo) return;
    setBranchesLoading(true);
    setBranchesError(null);
    try {
      setBranches(await ipc.listBranches(repo.path));
    } catch (e) {
      setBranchesError(String(e));
      appStore.pushError(String(e));
    } finally {
      setBranchesLoading(false);
    }
  }

  onMount(() => {
    place();
    window.addEventListener("resize", place);
    queueMicrotask(() => inputRef?.focus());
    void loadBranches();
    void appStore.refreshWorktrees();
  });
  onCleanup(() => window.removeEventListener("resize", place));

  const close = (): void => appStore.setWorktreeSwitcherOpen(false);

  const sections = createMemo(() =>
    buildSections(store.contexts, store.worktrees, branches() ?? [], query()),
  );
  const showCreate = createMemo(() => {
    const n = query().trim();
    return n.length > 0 && !hasExactBranch(n, store.contexts, store.worktrees, branches() ?? []);
  });
  const empty = createMemo(() => {
    const s = sections();
    return (
      !showCreate() && !branchesLoading() && !branchesError() &&
      s.open.length + s.onDisk.length + s.local.length + s.remote.length === 0
    );
  });
  const currentBranch = () => store.repo?.branch ?? store.repo?.name ?? "current branch";

  // ── Row actions ─────────────────────────────────────────────────────────────

  function createFromQuery(): void {
    const n = query().trim();
    if (!n || !showCreate()) return;
    close();
    void appStore.createWorktree(n);
  }

  async function pickBranch(b: BranchInfo): Promise<void> {
    if (busyKey()) return;
    setBusyKey(`${b.remote ?? ""}:${b.name}`);
    setActionError(null);
    const err = await appStore.addWorktreeForBranch(b.name, b.remote);
    setBusyKey(null);
    if (err === null) {
      close();
      return;
    }
    // Named failure (e.g. "already checked out at <path>"): show it and reload
    // so the covering worktree row appears — that row IS the "open it" affordance.
    setActionError(err);
    void loadBranches();
    void appStore.refreshWorktrees();
  }

  async function removeWorktree(w: Worktree, e: MouseEvent): Promise<void> {
    e.stopPropagation();
    const label = w.branch ?? w.name;
    const msg = w.dirty
      ? `Worktree "${label}" has UNCOMMITTED CHANGES.\n\nRemove it from disk anyway? The uncommitted changes will be permanently lost.`
      : `Remove worktree "${label}" from disk?\n\n${w.path}\n\nThe branch is kept — only the checkout directory is removed.`;
    if (!window.confirm(msg)) return;
    await appStore.removeWorktreeExplicit(String(w.path), w.dirty);
    void loadBranches();
  }

  function onKeyDown(e: KeyboardEvent): void {
    if (e.key === "Escape") {
      e.preventDefault();
      close();
    } else if (e.key === "Enter") {
      e.preventDefault();
      createFromQuery();
    }
  }

  async function runFetch(): Promise<void> {
    const repo = store.repo;
    if (!repo || fetching()) return;
    setFetching(true);
    setActionError(null);
    try {
      await ipc.fetchRemotes(repo.path);
      await loadBranches();
      void appStore.refreshWorktrees();
    } catch (e) {
      setActionError(String(e));
      appStore.pushError(String(e));
    } finally {
      setFetching(false);
    }
  }

  return (
    <Portal>
      <div class="ws-backdrop" onClick={close} />
      <div
        class="ws-pop"
        style={{ left: `${pos().left}px`, top: `${pos().top}px`, "max-height": `${pos().maxH}px` }}
        onClick={(e) => e.stopPropagation()}
      >
        <input
          ref={inputRef}
          class="ws-input"
          placeholder="filter or new branch name…"
          value={query()}
          onInput={(e) => setQuery(e.currentTarget.value)}
          onKeyDown={onKeyDown}
        />
        <Show when={actionError()}>
          <div class="ws-error">{actionError()}</div>
        </Show>

        <div class="ws-list">
          <Show when={showCreate()}>
            <CreateRow name={query().trim()} from={currentBranch()} onPick={createFromQuery} />
          </Show>

          <Show when={sections().open.length > 0}>
            <div class="ws-section">Open</div>
            <For each={sections().open}>
              {(c) => (
                <ContextRow
                  ctx={c}
                  active={!!store.activeContextPath && samePath(store.activeContextPath, c.state.path)}
                  showProject={new Set(store.contexts.map((x) => x.state.project ?? x.state.name)).size > 1}
                  onPick={() => { close(); void appStore.switchContext(c.state.path); }}
                />
              )}
            </For>
          </Show>

          <Show when={sections().onDisk.length > 0}>
            <div class="ws-section">Worktrees</div>
            <For each={sections().onDisk}>
              {(w) => (
                <WorktreeRow
                  wt={w}
                  onPick={() => { close(); void appStore.openWorktreeContext(w); }}
                  onRemove={(e) => void removeWorktree(w, e)}
                />
              )}
            </For>
          </Show>

          <Show when={branchesLoading()}>
            <div class="ws-status">loading branches…</div>
          </Show>
          <Show when={branchesError()}>
            <div class="ws-error">branches failed to load: {branchesError()}</div>
          </Show>

          <Show when={!branchesLoading() && sections().local.length > 0}>
            <div class="ws-section">Branches</div>
            <For each={sections().local}>
              {(b) => (
                <BranchRow branch={b} busy={busyKey() === `:${b.name}`} onPick={() => void pickBranch(b)} />
              )}
            </For>
          </Show>

          <Show when={!branchesLoading() && sections().remote.length > 0}>
            <div class="ws-section">Remote</div>
            <For each={sections().remote}>
              {(b) => (
                <BranchRow
                  branch={b}
                  busy={busyKey() === `${b.remote}:${b.name}`}
                  onPick={() => void pickBranch(b)}
                />
              )}
            </For>
          </Show>

          <Show when={empty()}>
            <div class="ws-status">no matches</div>
          </Show>
        </div>

        <div class="ws-footer">
          <button class="ws-fetch" disabled={fetching()} onClick={() => void runFetch()}>
            <Show
              when={fetching()}
              fallback={
                <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                  <path d="M21 12a9 9 0 1 1-2.64-6.36M21 3v6h-6" stroke-linecap="round" stroke-linejoin="round" />
                </svg>
              }
            >
              <span class="ws-spin" />
            </Show>
            Fetch
          </button>
          <span class="ws-hint">type a new name to create a worktree</span>
        </div>
      </div>
      <style>{SWITCHER_CSS}</style>
    </Portal>
  );
}
