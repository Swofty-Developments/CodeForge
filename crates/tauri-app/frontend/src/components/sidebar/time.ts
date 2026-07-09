/* Relative-time helpers shared by the sidebar and the feature detail view. */

import { createSignal, onCleanup } from "solid-js";

/** Reactive "current time" that ticks so relative ages stay fresh. */
export function createNow(intervalMs = 30_000): () => number {
  const [now, setNow] = createSignal(Date.now());
  const timer = setInterval(() => setNow(Date.now()), intervalMs);
  onCleanup(() => clearInterval(timer));
  return now;
}

export function formatAgo(ts: string | null, now: number): string {
  if (!ts) return "never";
  const t = Date.parse(ts);
  if (Number.isNaN(t)) return "—";
  const mins = Math.floor((now - t) / 60_000);
  if (mins < 1) return "just now";
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}
