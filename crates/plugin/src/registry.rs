//! Plugin registry for managing installed plugins.
//!
//! Provides types for tracking installed plugins, resolving dependencies,
//! checking version constraints, and detecting conflicts.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// The status of a plugin installation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginStatus {
    /// Plugin is installed and active.
    Active,
    /// Plugin is installed but disabled.
    Disabled,
    /// Plugin is installed but has errors.
    Error,
    /// Plugin has an update available.
    UpdateAvailable,
    /// Plugin is being installed.
    Installing,
    /// Plugin is being uninstalled.
    Uninstalling,
}

impl fmt::Display for PluginStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginStatus::Active => write!(f, "active"),
            PluginStatus::Disabled => write!(f, "disabled"),
            PluginStatus::Error => write!(f, "error"),
            PluginStatus::UpdateAvailable => write!(f, "update available"),
            PluginStatus::Installing => write!(f, "installing"),
            PluginStatus::Uninstalling => write!(f, "uninstalling"),
        }
    }
}

/// An installed plugin entry in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledPlugin {
    /// The plugin identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Installed version.
    pub version: String,
    /// Current status.
    pub status: PluginStatus,
    /// Installation path on disk.
    pub install_path: String,
    /// When the plugin was installed.
    pub installed_at: String,
    /// When the plugin was last updated.
    pub updated_at: Option<String>,
    /// Dependencies on other plugins.
    pub dependencies: Vec<PluginDependency>,
    /// Error message if status is Error.
    pub error_message: Option<String>,
    /// Plugin configuration.
    pub config: HashMap<String, serde_json::Value>,
}

impl InstalledPlugin {
    /// Create a new installed plugin entry.
    pub fn new(
        id: impl Into<String>,
        name: impl Into<String>,
        version: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            version: version.into(),
            status: PluginStatus::Active,
            install_path: String::new(),
            installed_at: String::new(),
            updated_at: None,
            dependencies: Vec::new(),
            error_message: None,
            config: HashMap::new(),
        }
    }

    /// Check if the plugin is usable.
    pub fn is_usable(&self) -> bool {
        self.status == PluginStatus::Active
    }

    /// Check if the plugin has unresolved errors.
    pub fn has_error(&self) -> bool {
        self.status == PluginStatus::Error
    }
}

impl fmt::Display for InstalledPlugin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} v{} ({})", self.name, self.version, self.status)
    }
}

/// A dependency declaration from one plugin to another.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    /// The required plugin ID.
    pub plugin_id: String,
    /// Version constraint (e.g., "^1.0.0", ">=2.0.0").
    pub version_constraint: String,
    /// Whether this dependency is optional.
    pub optional: bool,
}

impl fmt::Display for PluginDependency {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let optional = if self.optional { " (optional)" } else { "" };
        write!(f, "{} {}{}", self.plugin_id, self.version_constraint, optional)
    }
}

/// The plugin registry managing all installed plugins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PluginRegistry {
    /// Installed plugins indexed by ID.
    plugins: HashMap<String, InstalledPlugin>,
}

impl PluginRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            plugins: HashMap::new(),
        }
    }

    /// Register an installed plugin.
    pub fn install(&mut self, plugin: InstalledPlugin) {
        self.plugins.insert(plugin.id.clone(), plugin);
    }

    /// Uninstall a plugin by ID. Returns the removed plugin if found.
    pub fn uninstall(&mut self, id: &str) -> Option<InstalledPlugin> {
        self.plugins.remove(id)
    }

    /// Get a plugin by ID.
    pub fn get(&self, id: &str) -> Option<&InstalledPlugin> {
        self.plugins.get(id)
    }

    /// Get a mutable reference to a plugin.
    pub fn get_mut(&mut self, id: &str) -> Option<&mut InstalledPlugin> {
        self.plugins.get_mut(id)
    }

    /// Check if a plugin is installed.
    pub fn is_installed(&self, id: &str) -> bool {
        self.plugins.contains_key(id)
    }

    /// List all installed plugins.
    pub fn list(&self) -> Vec<&InstalledPlugin> {
        let mut plugins: Vec<_> = self.plugins.values().collect();
        plugins.sort_by_key(|p| &p.name);
        plugins
    }

    /// List plugins filtered by status.
    pub fn list_by_status(&self, status: PluginStatus) -> Vec<&InstalledPlugin> {
        self.plugins
            .values()
            .filter(|p| p.status == status)
            .collect()
    }

    /// Return the number of installed plugins.
    pub fn count(&self) -> usize {
        self.plugins.len()
    }

    /// Return the number of active plugins.
    pub fn active_count(&self) -> usize {
        self.plugins.values().filter(|p| p.is_usable()).count()
    }

    /// Enable a plugin.
    pub fn enable(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.status = PluginStatus::Active;
            true
        } else {
            false
        }
    }

    /// Disable a plugin.
    pub fn disable(&mut self, id: &str) -> bool {
        if let Some(plugin) = self.plugins.get_mut(id) {
            plugin.status = PluginStatus::Disabled;
            true
        } else {
            false
        }
    }

    /// Check for dependency conflicts.
    pub fn find_conflicts(&self) -> Vec<DependencyConflict> {
        let mut conflicts = Vec::new();
        for plugin in self.plugins.values() {
            for dep in &plugin.dependencies {
                if dep.optional {
                    continue;
                }
                if !self.is_installed(&dep.plugin_id) {
                    conflicts.push(DependencyConflict::Missing {
                        required_by: plugin.id.clone(),
                        missing_id: dep.plugin_id.clone(),
                        constraint: dep.version_constraint.clone(),
                    });
                }
            }
        }
        conflicts
    }

    /// Find plugins that depend on the given plugin.
    pub fn dependents(&self, plugin_id: &str) -> Vec<&InstalledPlugin> {
        self.plugins
            .values()
            .filter(|p| p.dependencies.iter().any(|d| d.plugin_id == plugin_id))
            .collect()
    }

    /// Check if it is safe to uninstall a plugin (no active dependents).
    pub fn can_uninstall(&self, plugin_id: &str) -> bool {
        self.dependents(plugin_id)
            .iter()
            .all(|p| p.status == PluginStatus::Disabled)
    }

    /// Serialize the registry to JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize a registry from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// A dependency conflict detected in the registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DependencyConflict {
    /// A required dependency is not installed.
    Missing {
        /// The plugin that requires the dependency.
        required_by: String,
        /// The missing plugin ID.
        missing_id: String,
        /// The version constraint.
        constraint: String,
    },
    /// A dependency version does not satisfy the constraint.
    VersionMismatch {
        /// The plugin that requires the dependency.
        required_by: String,
        /// The dependency plugin ID.
        dependency_id: String,
        /// The required constraint.
        constraint: String,
        /// The installed version.
        installed_version: String,
    },
}

impl fmt::Display for DependencyConflict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DependencyConflict::Missing {
                required_by,
                missing_id,
                constraint,
            } => write!(
                f,
                "{required_by} requires {missing_id} {constraint}, but it is not installed"
            ),
            DependencyConflict::VersionMismatch {
                required_by,
                dependency_id,
                constraint,
                installed_version,
            } => write!(
                f,
                "{required_by} requires {dependency_id} {constraint}, but {installed_version} is installed"
            ),
        }
    }
}

/// Resolution plan for installing/updating plugins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResolutionPlan {
    /// Plugins to install.
    pub to_install: Vec<String>,
    /// Plugins to update.
    pub to_update: Vec<(String, String, String)>, // (id, from_version, to_version)
    /// Plugins to remove.
    pub to_remove: Vec<String>,
    /// Whether the plan is valid (no conflicts).
    pub valid: bool,
    /// Issues preventing resolution.
    pub issues: Vec<String>,
}

impl ResolutionPlan {
    /// Check if there are any changes to make.
    pub fn has_changes(&self) -> bool {
        !self.to_install.is_empty() || !self.to_update.is_empty() || !self.to_remove.is_empty()
    }
}

impl fmt::Display for ResolutionPlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "install: {}, update: {}, remove: {}",
            self.to_install.len(),
            self.to_update.len(),
            self.to_remove.len()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_install_uninstall() {
        let mut registry = PluginRegistry::new();
        let plugin = InstalledPlugin::new("test-plugin", "Test Plugin", "1.0.0");
        registry.install(plugin);
        assert!(registry.is_installed("test-plugin"));
        assert_eq!(registry.count(), 1);

        registry.uninstall("test-plugin");
        assert!(!registry.is_installed("test-plugin"));
    }

    #[test]
    fn enable_disable() {
        let mut registry = PluginRegistry::new();
        registry.install(InstalledPlugin::new("p1", "Plugin 1", "1.0.0"));
        assert_eq!(registry.active_count(), 1);

        registry.disable("p1");
        assert_eq!(registry.active_count(), 0);

        registry.enable("p1");
        assert_eq!(registry.active_count(), 1);
    }

    #[test]
    fn dependency_conflict() {
        let mut registry = PluginRegistry::new();
        let mut plugin = InstalledPlugin::new("dependent", "Dependent", "1.0.0");
        plugin.dependencies.push(PluginDependency {
            plugin_id: "missing-dep".to_string(),
            version_constraint: "^1.0.0".to_string(),
            optional: false,
        });
        registry.install(plugin);

        let conflicts = registry.find_conflicts();
        assert_eq!(conflicts.len(), 1);
    }
}
