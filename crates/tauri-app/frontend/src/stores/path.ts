/** Path helpers shared by the store slices. Worktree paths (serialized PathBuf)
 *  and RepoState.path are both absolute; normalize trailing slashes so context
 *  lookups match regardless of how a path was spelled. */

export function normPath(p: string): string {
  const trimmed = p.replace(/\/+$/, "");
  return trimmed.length > 0 ? trimmed : p;
}

export function samePath(a: string, b: string): boolean {
  return normPath(a) === normPath(b);
}
