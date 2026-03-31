//! Plugin system types and MCP integration for CodeForge.
//!
//! Provides the data structures for plugin manifests, lifecycle hooks,
//! MCP server configuration, skill definitions, and marketplace interaction.

pub mod api;
pub mod config;
pub mod hook;
pub mod loader;
pub mod marketplace;
pub mod mcp;
pub mod manifest;
pub mod registry;
pub mod sandbox;
pub mod skill;

pub use hook::{Hook, HookContext, HookHandler, HookResult};
pub use manifest::{PluginCapability, PluginManifest, PluginPermission};
pub use marketplace::{MarketplaceSource, PluginListing};
pub use mcp::{McpCapability, McpConnection, McpServerConfig};
pub use skill::{Skill, SkillExecution};
