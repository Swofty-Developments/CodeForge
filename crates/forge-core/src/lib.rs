//! forge-core — shared domain types + errors for CodeForge. No IO.
//!
//! # IPC serde contract
//!
//! Every struct in this crate crosses the Tauri IPC boundary (Rust ⇄ SolidJS) and is
//! therefore tagged `#[serde(rename_all = "camelCase")]`:
//! [`Feature`], [`FeatureFile`], [`TimelineEvent`], [`TimelineFilter`], [`FeaturePatch`],
//! [`RepoState`], [`IndexProgress`], [`SessionInfo`], [`StartSessionOpts`],
//! [`DiffByFeature`], [`FeatureDiffGroup`], [`FileDiff`], [`DiffHunk`], [`DiffLine`].
//!
//! Enums ([`FileRole`], [`Actor`], [`EventKind`], [`SessionStatus`]) serialize as
//! `snake_case` strings. These shapes are mirrored 1:1 in `frontend/src/types.ts`.

mod diff;
mod error;
mod feature;
mod session;
mod timeline;
mod worktree;

pub use diff::{DiffByFeature, DiffHunk, DiffLine, FeatureDiffGroup, FileDiff};
pub use error::{Error, Result};
pub use feature::{Feature, FeatureFile, FeaturePatch, FileRole};
pub use session::{IndexProgress, RepoState, SessionInfo, SessionStatus, StartSessionOpts};
pub use timeline::{Actor, EventKind, TimelineEvent, TimelineFilter};
pub use worktree::{MergeResult, Worktree};
