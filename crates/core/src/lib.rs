//! Core types, traits, and configuration for the CodeForge application.
//!
//! This crate provides the foundational data structures shared across
//! all CodeForge components — configuration, error handling, event types,
//! model definitions, and workspace management.

pub mod config;
pub mod error;
pub mod event;
pub mod model;
pub mod workspace;

pub use config::CodeForgeConfig;
pub use error::{CodeForgeError, Result};
pub use event::{AppEvent, EventMetadata};
pub use model::{Model, PermissionMode, Provider};
pub use workspace::{Workspace, WorkspaceManager};
