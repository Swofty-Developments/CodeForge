//! Plugin manifest types for declaring plugin metadata and capabilities.

use serde::{Deserialize, Serialize};

/// Manifest describing a plugin's identity, capabilities, and requirements.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    /// Unique plugin identifier (reverse domain style, e.g., `com.example.my-plugin`).
    pub id: String,
    /// Human-readable plugin name.
    pub name: String,
    /// Semantic version string.
    pub version: String,
    /// Short description of what the plugin does.
    pub description: String,
    /// Plugin author information.
    pub author: PluginAuthor,
    /// Capabilities this plugin provides.
    pub capabilities: Vec<PluginCapability>,
    /// Permissions this plugin requires.
    pub permissions: Vec<PluginPermission>,
    /// Minimum CodeForge version required.
    pub min_app_version: Option<String>,
    /// Entry point for the plugin (relative path to main script or binary).
    pub entry_point: String,
    /// Plugin icon path (relative to plugin root).
    pub icon: Option<String>,
    /// Homepage or repository URL.
    pub homepage: Option<String>,
    /// License identifier (SPDX).
    pub license: Option<String>,
    /// Tags for discoverability.
    pub tags: Vec<String>,
}

/// Plugin author information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginAuthor {
    /// Author's display name.
    pub name: String,
    /// Author's email address.
    pub email: Option<String>,
    /// Author's website or profile URL.
    pub url: Option<String>,
}

/// A capability that a plugin can provide.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginCapability {
    /// The plugin provides custom tools.
    Tools,
    /// The plugin provides slash commands / skills.
    Skills,
    /// The plugin provides lifecycle hooks.
    Hooks,
    /// The plugin provides a custom UI panel.
    UiPanel,
    /// The plugin provides an MCP server.
    McpServer,
    /// The plugin provides context providers (file indexing, etc.).
    ContextProvider,
    /// The plugin provides custom themes.
    Theme,
}

/// A permission that a plugin requires to function.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PluginPermission {
    /// Read files from the workspace.
    ReadFiles,
    /// Write files to the workspace.
    WriteFiles,
    /// Execute shell commands.
    ExecuteCommands,
    /// Access the network.
    Network,
    /// Access git operations.
    Git,
    /// Access the database.
    Database,
    /// Send notifications to the user.
    Notifications,
    /// Access environment variables.
    Environment,
}

impl PluginManifest {
    /// Validate that the manifest has all required fields populated.
    pub fn validate(&self) -> Result<(), ManifestError> {
        if self.id.is_empty() {
            return Err(ManifestError::MissingField("id"));
        }
        if self.name.is_empty() {
            return Err(ManifestError::MissingField("name"));
        }
        if self.version.is_empty() {
            return Err(ManifestError::MissingField("version"));
        }
        if self.entry_point.is_empty() {
            return Err(ManifestError::MissingField("entry_point"));
        }
        Ok(())
    }

    /// Returns `true` if the plugin has the given capability.
    pub fn has_capability(&self, cap: &PluginCapability) -> bool {
        self.capabilities.contains(cap)
    }

    /// Returns `true` if the plugin requires the given permission.
    pub fn requires_permission(&self, perm: &PluginPermission) -> bool {
        self.permissions.contains(perm)
    }
}

/// Errors in plugin manifest validation.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    /// A required field is missing or empty.
    #[error("missing required field: {0}")]
    MissingField(&'static str),

    /// The manifest JSON could not be parsed.
    #[error("invalid manifest format: {0}")]
    InvalidFormat(String),
}
