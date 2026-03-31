//! Application configuration types and defaults.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::model::{Model, PermissionMode};

/// Top-level configuration for the CodeForge application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeForgeConfig {
    /// Default AI model to use for new sessions.
    pub default_model: Model,
    /// Permission mode governing tool approvals.
    pub permission_mode: PermissionMode,
    /// Path to the SQLite database file.
    pub database_path: PathBuf,
    /// Maximum number of concurrent sessions.
    pub max_concurrent_sessions: usize,
    /// Whether to enable telemetry collection.
    pub telemetry_enabled: bool,
    /// Git author name for automated commits.
    pub git_author_name: Option<String>,
    /// Git author email for automated commits.
    pub git_author_email: Option<String>,
    /// MCP server configurations.
    pub mcp_servers: Vec<McpServerEntry>,
    /// Custom API base URL override.
    pub api_base_url: Option<String>,
    /// Theme preference (light, dark, system).
    pub theme: Theme,
    /// Editor font size in pixels.
    pub font_size: u16,
    /// Whether to auto-save threads on navigation.
    pub auto_save: bool,
    /// Maximum context window tokens before compaction.
    pub max_context_tokens: u64,
    /// Path to custom instructions file.
    pub custom_instructions_path: Option<PathBuf>,
}

/// A configured MCP server entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerEntry {
    /// Display name for the server.
    pub name: String,
    /// Command to launch the server.
    pub command: String,
    /// Arguments passed to the server command.
    pub args: Vec<String>,
    /// Whether this server is enabled.
    pub enabled: bool,
}

/// Application theme preference.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    /// Light color scheme.
    Light,
    /// Dark color scheme.
    Dark,
    /// Follow the operating system setting.
    System,
}

impl Default for CodeForgeConfig {
    fn default() -> Self {
        Self {
            default_model: Model::Sonnet,
            permission_mode: PermissionMode::Normal,
            database_path: PathBuf::from("codeforge.db"),
            max_concurrent_sessions: 4,
            telemetry_enabled: false,
            git_author_name: None,
            git_author_email: None,
            mcp_servers: Vec::new(),
            api_base_url: None,
            theme: Theme::System,
            font_size: 14,
            auto_save: true,
            max_context_tokens: 200_000,
            custom_instructions_path: None,
        }
    }
}

impl CodeForgeConfig {
    /// Load configuration from a JSON file, falling back to defaults for missing fields.
    pub fn from_file(path: &std::path::Path) -> Result<Self, crate::error::CodeForgeError> {
        let contents = std::fs::read_to_string(path)?;
        let config: Self =
            serde_json::from_str(&contents).map_err(|e| crate::error::CodeForgeError::Config {
                message: format!("invalid config JSON: {e}"),
            })?;
        Ok(config)
    }

    /// Serialize the configuration to a pretty-printed JSON string.
    pub fn to_json(&self) -> Result<String, crate::error::CodeForgeError> {
        serde_json::to_string_pretty(self).map_err(|e| crate::error::CodeForgeError::Config {
            message: format!("failed to serialize config: {e}"),
        })
    }
}
