/* Composer attachment helpers: basename for chip labels + building the final
 * prompt with an "@<path>" reference header so Claude Code reads the files with
 * its own tools. We only ever pass paths — we never read file contents ourselves. */

export function basename(path: string): string {
  const parts = path.split(/[\\/]/).filter(Boolean);
  return parts.length ? parts[parts.length - 1] : path;
}

/** Prepend an "@<path>" attachment header to the prompt; paths pass through
 *  verbatim so Claude Code's file tools resolve them. Empty => text unchanged. */
export function withAttachments(text: string, paths: readonly string[]): string {
  if (paths.length === 0) return text;
  const refs = paths.map((p) => `@${p}`).join("\n");
  return `[Attached files]\n${refs}\n\n${text}`;
}
