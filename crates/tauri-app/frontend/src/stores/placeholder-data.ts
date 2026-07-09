/** Placeholder features shown in the shell until the backend is wired.
 *  Remove once open_repo/get_features are implemented. */

import type { Feature } from "../types";

const now = new Date().toISOString();

export const PLACEHOLDER_FEATURES: Feature[] = [
  {
    slug: "feature-index",
    name: "Feature indexing",
    description:
      "Cold-start decomposition of a repository into features via headless Claude, plus incremental re-classification of edited paths.",
    entryPoints: ["crates/forge-index/src/indexer.rs"],
    files: [
      { path: "crates/forge-index/src/lib.rs", role: "core", pinned: false },
      { path: "crates/forge-index/src/indexer.rs", role: "core", pinned: false },
      { path: "crates/forge-core/src/feature.rs", role: "support", pinned: false },
    ],
    tags: ["indexing", "claude"],
    confidence: 0.95,
    pinned: true,
    updatedAt: now,
  },
  {
    slug: "timeline",
    name: "Timeline",
    description: "Append-only per-repo event log fed by Claude Code hooks, queried by feature and streamed live to the UI.",
    entryPoints: ["crates/forge-timeline/src/lib.rs"],
    files: [
      { path: "crates/forge-timeline/src/lib.rs", role: "core", pinned: false },
      { path: "crates/forge-daemon/src/http.rs", role: "support", pinned: false },
    ],
    tags: ["events", "sqlite"],
    confidence: 0.9,
    pinned: false,
    updatedAt: now,
  },
  {
    slug: "diff-review",
    name: "Diff review",
    description: "Pending git changes parsed into hunks and grouped by feature instead of by file.",
    entryPoints: ["crates/forge-git/src/lib.rs"],
    files: [{ path: "crates/forge-git/src/lib.rs", role: "core", pinned: false }],
    tags: ["git"],
    confidence: 0.85,
    pinned: false,
    updatedAt: now,
  },
  {
    slug: "claude-sessions",
    name: "Embedded Claude sessions",
    description: "Real claude processes wrapped by the agent sidecar, streaming NDJSON events into the session pane.",
    entryPoints: ["crates/forge-session/src/claude.rs"],
    files: [
      { path: "crates/forge-session/src/claude.rs", role: "core", pinned: false },
      { path: "crates/tauri-app/agent-sidecar/index.mjs", role: "core", pinned: false },
    ],
    tags: ["sessions", "sidecar"],
    confidence: 0.9,
    pinned: false,
    updatedAt: now,
  },
];
