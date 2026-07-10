/* Presentational rows for the WorktreeSwitcher. All actions and busy state are
 * owned by the switcher and passed in. */

import { Show } from "solid-js";
import type { BranchInfo, RepoContext, Worktree } from "../../types";
import { basename } from "./switcher-data";

function BranchIcon() {
  return (
    <svg class="ws-ico" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M4.5 2.5v7m0 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0-7a1.5 1.5 0 1 1 0 3 1.5 1.5 0 0 1 0-3m7 0a1.5 1.5 0 1 0 0 3 1.5 1.5 0 0 0 0-3m0 3v1.5a3 3 0 0 1-3 3H4.5"
        fill="none"
        stroke="currentColor"
        stroke-width="1.1"
        stroke-linecap="round"
      />
    </svg>
  );
}

function FolderIcon() {
  return (
    <svg class="ws-ico" viewBox="0 0 16 16" aria-hidden="true">
      <path
        d="M2 4.2a1 1 0 0 1 1-1h3l1.2 1.3H13a1 1 0 0 1 1 1V12a1 1 0 0 1-1 1H3a1 1 0 0 1-1-1z"
        fill="none"
        stroke="currentColor"
        stroke-width="1.1"
      />
    </svg>
  );
}

/** Ahead/behind counts + dirty dot — the same visual language as the tab strip. */
function WtStatus(props: { wt: Worktree }) {
  return (
    <>
      <Show when={props.wt.dirty}>
        <span class="ws-dirty" title="uncommitted changes" />
      </Show>
      <Show when={props.wt.ahead > 0}>
        <span class="ws-count ws-ahead" title={`${props.wt.ahead} ahead of base`}>↑{props.wt.ahead}</span>
      </Show>
      <Show when={props.wt.behind > 0}>
        <span class="ws-count ws-behind" title={`${props.wt.behind} behind base`}>↓{props.wt.behind}</span>
      </Show>
    </>
  );
}

/** "Create worktree '<query>' from <current branch>" — shown for a novel name. */
export function CreateRow(props: { name: string; from: string; onPick: () => void }) {
  return (
    <button class="ws-row ws-row--create" onClick={() => props.onPick()}>
      <svg class="ws-ico" viewBox="0 0 24 24" aria-hidden="true">
        <path d="M12 5v14M5 12h14" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" />
      </svg>
      <span class="ws-text">
        Create worktree <span class="ws-mono ws-query">{props.name}</span> from{" "}
        <span class="ws-mono">{props.from}</span>
      </span>
    </button>
  );
}

/** An open context (tab): click switches to it. */
export function ContextRow(props: { ctx: RepoContext; active: boolean; onPick: () => void }) {
  return (
    <button
      class="ws-row"
      classList={{ "ws-row--active": props.active }}
      title={props.ctx.state.path}
      onClick={() => props.onPick()}
    >
      <Show when={!props.ctx.isBase} fallback={<FolderIcon />}>
        <BranchIcon />
      </Show>
      <span class="ws-mono ws-name">{props.ctx.state.branch ?? props.ctx.state.name}</span>
      <Show when={props.ctx.isBase}>
        <span class="ws-badge">base</span>
      </Show>
      <span class="ws-dim">{basename(props.ctx.state.path)}</span>
    </button>
  );
}

/** A worktree on disk (not open): click opens it; trash starts the EXPLICIT
 *  remove flow. Row is a div — a nested <button> inside a <button> is invalid. */
export function WorktreeRow(props: {
  wt: Worktree;
  onPick: () => void;
  onRemove: (e: MouseEvent) => void;
}) {
  return (
    <div class="ws-row" role="button" title={String(props.wt.path)} onClick={() => props.onPick()}>
      <BranchIcon />
      <span class="ws-mono ws-name">{props.wt.branch ?? props.wt.name}</span>
      <WtStatus wt={props.wt} />
      <span class="ws-dim">{basename(String(props.wt.path))}</span>
      <button class="ws-trash" title="Remove worktree from disk…" onClick={(e) => props.onRemove(e)}>
        <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.8">
          <path d="M3 6h18M8 6V4a1 1 0 0 1 1-1h6a1 1 0 0 1 1 1v2m3 0v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6M10 11v6M14 11v6" />
        </svg>
      </button>
    </div>
  );
}

/** A branch without a worktree. remote set = remote-tracking ("origin/…"),
 *  else local. Click checks it out into a new worktree. */
export function BranchRow(props: { branch: BranchInfo; busy: boolean; onPick: () => void }) {
  const full = () =>
    props.branch.remote ? `${props.branch.remote}/${props.branch.name}` : props.branch.name;
  return (
    <button
      class="ws-row"
      disabled={props.busy}
      title={`Check out ${full()} into a new worktree`}
      onClick={() => props.onPick()}
    >
      <BranchIcon />
      <span class="ws-mono ws-name">
        <Show when={props.branch.remote}>
          <span class="ws-remote">{props.branch.remote}/</span>
        </Show>
        {props.branch.name}
      </span>
      <Show when={props.busy}>
        <span class="ws-spin" />
      </Show>
    </button>
  );
}
