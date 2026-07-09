/* Open a repo-relative file in the OS default handler (editor). Calls the Tauri
 * shell plugin command directly so we don't depend on the @tauri-apps/plugin-shell
 * JS wrapper (not in package.json yet — see blockers). Needs the shell plugin +
 * `shell:allow-open` capability on the Rust side. */

import { invoke } from "@tauri-apps/api/core";

function join(repoPath: string, rel: string): string {
  const base = repoPath.replace(/\/+$/, "");
  const tail = rel.replace(/^\/+/, "");
  return `${base}/${tail}`;
}

export async function openInEditor(repoPath: string | undefined, relPath: string): Promise<void> {
  const abs = repoPath ? join(repoPath, relPath) : relPath;
  await invoke("plugin:shell|open", { path: abs, with: null });
}
