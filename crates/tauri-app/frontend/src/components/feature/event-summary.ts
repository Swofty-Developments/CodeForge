/* Timeline-event presentation helpers for the feature-detail recent-activity
 * list: a semantic accent per kind and a best-effort one-line summary from the
 * (untyped) payload. */

import type { EventKind, TimelineEvent } from "../../types";

/** Semantic accent name (drives the row's icon/dot color) per event kind. */
export function kindAccent(kind: EventKind): string {
  switch (kind) {
    case "file_edited":
    case "feature_edited":
      return "primary";
    case "command_run":
      return "sky";
    case "tests_run":
      return "green";
    case "session_started":
    case "session_ended":
      return "purple";
    case "index_started":
    case "index_completed":
      return "amber";
    case "feature_pinned":
      return "amber";
    default:
      return "text-tertiary";
  }
}

/** Short human label for the kind (used when the payload yields nothing). */
export function kindLabel(kind: EventKind): string {
  return kind.replace(/_/g, " ");
}

function str(v: unknown): string | null {
  return typeof v === "string" && v.trim() ? v.trim() : null;
}

/** Best-effort one-line summary from the kind + loosely-typed payload. */
export function summarize(ev: TimelineEvent): string {
  const p = (ev.payload ?? {}) as Record<string, unknown>;
  switch (ev.kind) {
    case "file_edited": {
      const path = str(p.path) ?? str(p.file);
      if (path) return path;
      if (Array.isArray(p.files) && p.files.length) return `${p.files.length} files edited`;
      return "file edited";
    }
    case "command_run":
      return str(p.command) ?? str(p.cmd) ?? "command run";
    case "tests_run": {
      const summary = str(p.summary);
      if (summary) return summary;
      const passed = typeof p.passed === "number" ? p.passed : null;
      const failed = typeof p.failed === "number" ? p.failed : null;
      if (passed !== null || failed !== null) return `${passed ?? 0} passed · ${failed ?? 0} failed`;
      return "tests run";
    }
    case "note":
      return str(p.text) ?? str(p.note) ?? "note";
    case "session_started":
      return str(p.prompt) ?? "session started";
    case "session_ended":
      return str(p.reason) ?? "session ended";
    case "feature_pinned":
      return p.pinned === false ? "unpinned" : "pinned";
    case "feature_edited":
      return str(p.field) ? `edited ${str(p.field)}` : "feature edited";
    case "index_started":
      return "indexing started";
    case "index_completed":
      return str(p.summary) ?? "indexing completed";
    default:
      return kindLabel(ev.kind);
  }
}

/** A tiny inline glyph per kind for the activity row icon. */
export function kindGlyph(kind: EventKind): string {
  switch (kind) {
    case "file_edited":
      return "M4 4h9l4 4v12H4z"; // document
    case "feature_edited":
      return "M4 4h9l4 4v12H4z";
    case "command_run":
      return "M4 6l5 6-5 6M12 18h8"; // terminal prompt
    case "tests_run":
      return "M5 12l4 4 10-10"; // check
    case "session_started":
    case "session_ended":
      return "M8 3v4M16 3v4M4 9h16M5 5h14v14H5z"; // session frame
    case "feature_pinned":
      return "M16 3v2l-1 1v5l3 3v2h-5v6l-1 1-1-1v-6H6v-2l3-3V6L8 5V3h8z"; // pin
    case "index_started":
    case "index_completed":
      return "M4 6h16M4 12h16M4 18h10"; // lines
    default:
      return "M6 12h12"; // dash / note
  }
}
