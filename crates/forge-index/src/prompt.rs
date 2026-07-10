//! Prompt builders for the headless-`claude` indexer passes.

use forge_core::{Feature, FileRole};

/// Lines a living doc is clamped to (also stated to the model).
pub(crate) const DOC_MAX_LINES: usize = 60;

pub(crate) fn role_label(role: FileRole) -> &'static str {
    match role {
        FileRole::Core => "core",
        FileRole::Support => "support",
        FileRole::Test => "test",
        FileRole::Config => "config",
        FileRole::Unknown => "unknown",
    }
}

/// Cold-start decomposition prompt → strict JSON feature array.
pub(crate) fn decomposition_prompt() -> String {
    r#"You are indexing a code repository to build a *feature map* — organising the
codebase by product features, not files.

Explore this repository from its root:
- read the entry manifests (package.json, Cargo.toml, pyproject.toml, go.mod, ...)
- skim the directory tree and any README / docs
- open a few representative source files to understand responsibilities

Then decompose the repo into features. A feature is ONE distinct capability or
flow (e.g. "login", "password-reset", "session-list", "diff-review",
"timeline-storage") — finer-grained than a whole app or subsystem, but coarser
than a single file or language-level module.

Scale the number of features to the repo's real surface area. A small library
might have 5-12; a large multi-crate or multi-package repo commonly has 25-60+.
Do NOT force everything into a handful of coarse buckets, and do NOT cap yourself
artificially — list every genuinely-distinct feature you find.

CRITICAL — never emit a catch-all feature for an entire area. A whole frontend is
never one "web" feature; a whole service is never one "api" feature. Decompose
each area into its individual capabilities (the specific pages, user flows,
screens, endpoint groups, jobs, or subsystems) and organise them with `group`
(below), which is what keeps a long list navigable. If you are about to write a
feature whose scope is "a whole crate / a whole app / a whole layer", STOP and
split it into the distinct features inside it.

Return ONLY a JSON array (no prose, no markdown fences) of objects with exactly
these fields:
[
  {
    "slug": "kebab-case-id",
    "name": "Human Readable Name",
    "description": "1-3 sentences on what this feature does.",
    "group": "hierarchy/path",
    "entryPoints": ["repo/relative/path.rs"],
    "files": [ { "path": "repo/relative/path.rs", "role": "core" } ],
    "tags": ["short", "keywords"],
    "confidence": 0.0
  }
]

Rules:
- slug is kebab-case and stable; name is title-case.
- group is a slash-delimited hierarchy path the sidebar nests features by, and it
  should USE MULTIPLE LEVELS (up to 3). Level 1 is the crate/package/app name in
  a multi-crate repo or monorepo (e.g. "forge-index", "web"), or the layer in a
  single-package repo (e.g. "backend"). When an area contains more than ~5
  features, subdivide with a second (and if useful third) level by subsystem or
  surface — e.g. "atomix-web/pages", "atomix-web/trading", "atomix-core/matching",
  "backend/auth/oidc". Never leave 10+ features sitting flat under one group when
  they cluster into obvious sub-areas. Related features MUST share the exact same
  group string, and a parent group's features must genuinely belong to every
  segment of the path. Omit group only when a feature genuinely spans the whole
  repo.
- entryPoints are the 1-3 best "start reading here" files for the feature.
- role is one of: core | support | test | config.
- every path is RELATIVE to the repo root and must exist on disk.
- confidence is your 0..1 certainty the feature is real and correctly scoped.
- output the JSON array and nothing else."#
        .to_string()
}

/// Per-feature living-doc prompt. The model returns markdown; this crate writes it.
pub(crate) fn doc_prompt(feature: &Feature) -> String {
    let entries = if feature.entry_points.is_empty() {
        "(none listed)".to_string()
    } else {
        feature
            .entry_points
            .iter()
            .map(|p| format!("- {}", p.display()))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let files = if feature.files.is_empty() {
        "(none listed)".to_string()
    } else {
        feature
            .files
            .iter()
            .map(|f| format!("- {} ({})", f.path.display(), role_label(f.role)))
            .collect::<Vec<_>>()
            .join("\n")
    };
    format!(
        r#"Write a concise living design doc (Markdown) for ONE feature of this repository.

Feature: {name} ({slug})
Description: {description}
Entry points:
{entries}
Key files:
{files}

Read the listed files (and any obvious neighbours) to ground the doc in the real
code, then write Markdown with these sections:
- **Purpose** — 1-2 sentences.
- **How it works** — 3-6 bullets.
- **Key files** — each key file with a one-line role.
- **Invariants & gotchas** — constraints a maintainer must not break.

Keep it under {max} lines. Output ONLY the Markdown document — no code fence around
the whole thing, no preamble, no closing remarks."#,
        name = feature.name,
        slug = feature.slug,
        description = feature.description,
        max = DOC_MAX_LINES,
    )
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    #[test]
    fn doc_prompt_mentions_feature_and_files() {
        let feature = Feature {
            slug: "auth".into(),
            name: "Auth".into(),
            description: "Login.".into(),
            entry_points: vec![PathBuf::from("src/auth/mod.rs")],
            files: vec![forge_core::FeatureFile {
                path: PathBuf::from("src/auth/session.rs"),
                role: FileRole::Core,
                pinned: false,
            }],
            tags: vec![],
            confidence: 0.8,
            pinned: false,
            color: None,
            group: None,
            updated_at: chrono::Utc::now(),
        };
        let prompt = doc_prompt(&feature);
        assert!(prompt.contains("Auth (auth)"));
        assert!(prompt.contains("src/auth/mod.rs"));
        assert!(prompt.contains("src/auth/session.rs (core)"));
    }
}
