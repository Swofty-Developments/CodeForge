/* Timeline-event presentation helpers for the feature-detail recent-activity
 * list: a semantic accent per kind and a one-line summary read from the typed,
 * kind-discriminated payload — exactly one field per kind, no key guessing. */

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

/** One-line summary read from the kind-discriminated typed payload. Each branch
 *  reads exactly the field(s) the backend writes for that kind. */
export function summarize(ev: TimelineEvent): string {
  switch (ev.kind) {
    case "file_edited":
      return ev.payload.path || kindLabel(ev.kind);
    case "command_run":
      return ev.payload.command || kindLabel(ev.kind);
    case "note":
      return ev.payload.text || kindLabel(ev.kind);
    case "session_started":
      return ev.payload.source ? `session started (${ev.payload.source})` : "session started";
    case "session_ended":
      return "session ended";
    case "feature_pinned":
      return ev.payload.pinned ? "pinned" : "unpinned";
    case "feature_edited":
      return `edited ${ev.payload.name}`;
    case "index_started":
      return "indexing started";
    case "index_completed":
      return "error" in ev.payload
        ? `indexing failed: ${ev.payload.error}`
        : `indexed ${ev.payload.features} feature${ev.payload.features === 1 ? "" : "s"}`;
    case "tests_run":
      // The backend does not yet emit a payload for this kind (see contractNotes).
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
