/* Pure helpers for the timeline view: session lane colors, kind glyphs/labels,
 * kind-specific titles, relative time, and client-side filter matching. */

import type { Actor, EventKind, TimelineEvent, TimelineFilter } from "../../types";

// Lane accent order is part of the spec: green/sky/amber/purple/primary.
const LANE_VARS = ["--green", "--sky", "--amber", "--purple", "--primary"] as const;

/** Stable djb2 hash of the session id onto the accent set; system events are grey. */
export function laneColor(sessionId: string | null): string {
  if (!sessionId) return "var(--text-tertiary)";
  let h = 5381;
  for (let i = 0; i < sessionId.length; i++) {
    h = ((h << 5) + h + sessionId.charCodeAt(i)) >>> 0;
  }
  return `var(${LANE_VARS[h % LANE_VARS.length]})`;
}

export const KIND_LABEL: Record<EventKind, string> = {
  session_started: "Session started",
  session_ended: "Session ended",
  file_edited: "File edited",
  command_run: "Command run",
  tests_run: "Tests run",
  index_started: "Index started",
  index_completed: "Index completed",
  feature_pinned: "Feature pinned",
  feature_edited: "Feature edited",
  doc_updated: "Doc refreshed",
  note: "Note",
};

export const KIND_GLYPH: Record<EventKind, string> = {
  session_started: "▸",
  session_ended: "▪",
  file_edited: "±",
  command_run: "$",
  tests_run: "✓",
  index_started: "◌",
  index_completed: "◆",
  feature_pinned: "⚑",
  feature_edited: "✎",
  doc_updated: "↻",
  note: "❝",
};

/** Compact labels for the kind filter chips. */
export const KIND_SHORT: Record<EventKind, string> = {
  session_started: "start",
  session_ended: "end",
  file_edited: "edit",
  command_run: "cmd",
  tests_run: "tests",
  index_started: "index",
  index_completed: "indexed",
  feature_pinned: "pin",
  feature_edited: "feature",
  doc_updated: "doc",
  note: "note",
};

export const ALL_KINDS = Object.keys(KIND_LABEL) as EventKind[];
export const ALL_ACTORS: Actor[] = ["agent", "human", "system"];

// Payload shapes are backend-defined; read defensively across likely key names.
function field(payload: unknown, keys: string[]): unknown {
  if (typeof payload !== "object" || payload === null) return undefined;
  const rec = payload as Record<string, unknown>;
  for (const k of keys) {
    if (rec[k] !== undefined && rec[k] !== null) return rec[k];
  }
  return undefined;
}

function strField(payload: unknown, keys: string[]): string | null {
  const v = field(payload, keys);
  return typeof v === "string" && v.length > 0 ? v : null;
}

export function basename(path: string): string {
  const parts = path.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? path;
}

export function shortId(id: string | null): string {
  return id ? id.slice(0, 8) : "—";
}

const MAX_TITLE = 96;

function truncate(text: string): string {
  const one = text.replace(/\s+/g, " ").trim();
  return one.length > MAX_TITLE ? `${one.slice(0, MAX_TITLE)}…` : one;
}

export function eventTitle(event: TimelineEvent): string {
  switch (event.kind) {
    case "file_edited": {
      const p = strField(event.payload, ["path", "file_path", "filePath", "file"]);
      return p ? basename(p) : KIND_LABEL.file_edited;
    }
    case "command_run": {
      const cmd = strField(event.payload, ["command", "cmd"]);
      return cmd ? truncate(cmd) : KIND_LABEL.command_run;
    }
    case "session_started":
      return `Session ${shortId(event.sessionId)} started`;
    case "session_ended":
      return `Session ${shortId(event.sessionId)} ended`;
    case "note": {
      const text = strField(event.payload, ["text", "note", "message"]);
      return text ? truncate(text) : KIND_LABEL.note;
    }
    case "index_completed": {
      const v = field(event.payload, ["features", "features_count", "featuresCount", "count"]);
      const n = typeof v === "number" ? v : Array.isArray(v) ? v.length : null;
      return n !== null ? `Indexed ${n} feature${n === 1 ? "" : "s"}` : KIND_LABEL.index_completed;
    }
    case "doc_updated":
      // Typed DocUpdatedPayload: both outcomes are named, never conflated.
      return event.payload.outcome === "failed"
        ? `Doc refresh failed: ${event.payload.slug}`
        : `Doc refreshed: ${event.payload.slug}`;
    default:
      return KIND_LABEL[event.kind];
  }
}

/** Failed living-doc refresh — rows tint this title like other error states. */
export function docRefreshFailed(event: TimelineEvent): boolean {
  return event.kind === "doc_updated" && event.payload.outcome === "failed";
}

export function relativeTime(ts: string, nowMs: number): string {
  const t = Date.parse(ts);
  if (Number.isNaN(t)) return "";
  const s = Math.max(0, Math.floor((nowMs - t) / 1000));
  if (s < 45) return "now";
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h}h`;
  const d = Math.floor(h / 24);
  if (d < 7) return `${d}d`;
  return new Date(t).toLocaleDateString(undefined, { month: "short", day: "numeric" });
}

export function prettyPayload(payload: unknown): string {
  try {
    return JSON.stringify(payload, null, 2) ?? "null";
  } catch {
    return String(payload);
  }
}

// ── Filters ─────────────────────────────────────────────────────────────────

export interface TimelineFilters {
  feature: string | null;
  actor: Actor | null;
  kinds: EventKind[];
}

export function filtersNarrowed(f: TimelineFilters): boolean {
  return f.feature !== null || f.actor !== null || f.kinds.length > 0;
}

export function matchesFilters(event: TimelineEvent, f: TimelineFilters): boolean {
  if (f.feature && !event.featureSlugs.includes(f.feature)) return false;
  if (f.actor && event.actor !== f.actor) return false;
  if (f.kinds.length > 0 && !f.kinds.includes(event.kind)) return false;
  return true;
}

export function toIpcFilter(f: TimelineFilters): TimelineFilter {
  const out: TimelineFilter = {};
  if (f.feature) out.featureSlug = f.feature;
  if (f.actor) out.actor = f.actor;
  if (f.kinds.length > 0) out.kinds = [...f.kinds];
  return out;
}
