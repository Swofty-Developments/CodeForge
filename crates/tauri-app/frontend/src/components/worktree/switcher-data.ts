/** Pure derivation helpers for the WorktreeSwitcher. The four sections are
 *  disjoint by construction — a branch covered by a worktree row (checkedOutAt
 *  set) never re-appears as a branch row, and an open context's worktree never
 *  re-appears under "Worktrees". */

import type { BranchInfo, RepoContext, Worktree } from "../../types";
import { samePath } from "../../stores/path";

export function basename(p: string): string {
  const parts = p.replace(/\/+$/, "").split("/");
  return parts[parts.length - 1] || p;
}

export interface SwitcherSections {
  /** Open contexts (the tabs), base first. */
  open: RepoContext[];
  /** Worktrees on disk that are NOT open as contexts. */
  onDisk: Worktree[];
  /** Local branches not checked out in any worktree. */
  local: BranchInfo[];
  /** Remote-tracking branches whose short name has no local branch. */
  remote: BranchInfo[];
}

const contains = (haystack: string | null | undefined, q: string): boolean =>
  q.length === 0 || (haystack ?? "").toLowerCase().includes(q);

export function buildSections(
  contexts: RepoContext[],
  worktrees: Worktree[],
  branches: BranchInfo[],
  query: string,
): SwitcherSections {
  const q = query.trim().toLowerCase();

  const open = [...contexts]
    .sort((a, b) => (a.isBase === b.isBase ? 0 : a.isBase ? -1 : 1))
    .filter((c) => contains(c.state.branch ?? c.state.name, q) || contains(basename(c.state.path), q));

  const onDisk = worktrees
    .filter((w) => !contexts.some((c) => samePath(c.state.path, String(w.path))))
    .filter((w) => contains(w.branch ?? w.name, q) || contains(basename(String(w.path)), q));

  const local = branches.filter(
    (b) => b.remote === null && b.checkedOutAt === null && contains(b.name, q),
  );

  const localNames = new Set(branches.filter((b) => b.remote === null).map((b) => b.name));
  const remote = branches.filter(
    (b) => b.remote !== null && !localNames.has(b.name) && contains(`${b.remote}/${b.name}`, q),
  );

  return { open, onDisk, local, remote };
}

/** Whether `query` exactly names an existing branch (suppresses the create row). */
export function hasExactBranch(
  query: string,
  contexts: RepoContext[],
  worktrees: Worktree[],
  branches: BranchInfo[],
): boolean {
  const n = query.trim();
  if (!n) return false;
  return (
    branches.some((b) => b.name === n) ||
    worktrees.some((w) => w.branch === n) ||
    contexts.some((c) => c.state.branch === n)
  );
}
