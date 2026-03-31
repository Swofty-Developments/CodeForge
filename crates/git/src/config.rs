//! Git configuration reading and writing.
//!
//! Provides types for interacting with git config values at the repository,
//! global, and system levels. Supports reading user identity, remote URLs,
//! hook paths, and custom configuration keys.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;

/// The scope at which a git config value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ConfigScope {
    /// System-wide configuration (`/etc/gitconfig`).
    System,
    /// User-global configuration (`~/.gitconfig`).
    Global,
    /// Repository-local configuration (`.git/config`).
    Local,
    /// Worktree-specific configuration.
    Worktree,
}

impl fmt::Display for ConfigScope {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigScope::System => write!(f, "system"),
            ConfigScope::Global => write!(f, "global"),
            ConfigScope::Local => write!(f, "local"),
            ConfigScope::Worktree => write!(f, "worktree"),
        }
    }
}

/// A single git configuration entry with its source scope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigEntry {
    /// The full key (e.g., "user.name").
    pub key: String,
    /// The value.
    pub value: String,
    /// Where this value was defined.
    pub scope: ConfigScope,
}

impl fmt::Display for ConfigEntry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}={} ({})", self.key, self.value, self.scope)
    }
}

/// Represents the parsed contents of a git configuration file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GitConfig {
    /// Configuration entries indexed by key.
    entries: HashMap<String, Vec<ConfigEntry>>,
}

impl GitConfig {
    /// Create an empty configuration.
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Set a configuration value at the given scope.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>, scope: ConfigScope) {
        let key = key.into();
        let entry = ConfigEntry {
            key: key.clone(),
            value: value.into(),
            scope,
        };
        self.entries.entry(key).or_default().push(entry);
    }

    /// Get the effective value for a key (last-writer-wins across scopes).
    pub fn get(&self, key: &str) -> Option<&str> {
        self.entries
            .get(key)
            .and_then(|entries| entries.last())
            .map(|e| e.value.as_str())
    }

    /// Get all values for a multi-valued key.
    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.entries
            .get(key)
            .map(|entries| entries.iter().map(|e| e.value.as_str()).collect())
            .unwrap_or_default()
    }

    /// Get a boolean config value, interpreting "true", "yes", "on", "1" as true.
    pub fn get_bool(&self, key: &str) -> Option<bool> {
        self.get(key).map(|v| matches!(v.to_lowercase().as_str(), "true" | "yes" | "on" | "1"))
    }

    /// Get an integer config value.
    pub fn get_i64(&self, key: &str) -> Option<i64> {
        self.get(key).and_then(|v| v.parse().ok())
    }

    /// Remove all entries for a key.
    pub fn unset(&mut self, key: &str) {
        self.entries.remove(key);
    }

    /// Return all keys in the configuration.
    pub fn keys(&self) -> Vec<&str> {
        self.entries.keys().map(|k| k.as_str()).collect()
    }

    /// Return all entries.
    pub fn entries(&self) -> Vec<&ConfigEntry> {
        self.entries.values().flatten().collect()
    }

    /// Return the number of unique keys.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Return true if empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the configured user name.
    pub fn user_name(&self) -> Option<&str> {
        self.get("user.name")
    }

    /// Get the configured user email.
    pub fn user_email(&self) -> Option<&str> {
        self.get("user.email")
    }

    /// Get the signing key.
    pub fn signing_key(&self) -> Option<&str> {
        self.get("user.signingkey")
    }

    /// Check if commit signing is enabled.
    pub fn commit_gpgsign(&self) -> bool {
        self.get_bool("commit.gpgsign").unwrap_or(false)
    }

    /// Get the default branch name.
    pub fn default_branch(&self) -> &str {
        self.get("init.defaultBranch").unwrap_or("main")
    }

    /// Get the URL for a remote.
    pub fn remote_url(&self, remote: &str) -> Option<&str> {
        self.get(&format!("remote.{remote}.url"))
    }

    /// Get the fetch refspec for a remote.
    pub fn remote_fetch(&self, remote: &str) -> Option<&str> {
        self.get(&format!("remote.{remote}.fetch"))
    }

    /// Get the configured hook path.
    pub fn hooks_path(&self) -> PathBuf {
        self.get("core.hooksPath")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".git/hooks"))
    }

    /// Check if a specific config section exists.
    pub fn has_section(&self, section: &str) -> bool {
        let prefix = format!("{section}.");
        self.entries.keys().any(|k| k.starts_with(&prefix))
    }

    /// Get all keys within a section (e.g., "remote.origin").
    pub fn section_keys(&self, section: &str) -> Vec<&str> {
        let prefix = format!("{section}.");
        self.entries
            .keys()
            .filter(|k| k.starts_with(&prefix))
            .map(|k| k.as_str())
            .collect()
    }
}

/// User identity extracted from git configuration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GitIdentity {
    /// The user's display name.
    pub name: String,
    /// The user's email address.
    pub email: String,
    /// Optional GPG signing key.
    pub signing_key: Option<String>,
}

impl GitIdentity {
    /// Extract identity from a git config.
    pub fn from_config(config: &GitConfig) -> Option<Self> {
        let name = config.user_name()?.to_string();
        let email = config.user_email()?.to_string();
        let signing_key = config.signing_key().map(String::from);
        Some(Self {
            name,
            email,
            signing_key,
        })
    }

    /// Format as a git author string like "Name <email>".
    pub fn to_author_string(&self) -> String {
        format!("{} <{}>", self.name, self.email)
    }
}

impl fmt::Display for GitIdentity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} <{}>", self.name, self.email)
    }
}

/// Parse a git config file from its text content.
pub fn parse_config_file(content: &str, scope: ConfigScope) -> GitConfig {
    let mut config = GitConfig::new();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with(';') {
            continue;
        }
        if line.starts_with('[') {
            if let Some(end) = line.find(']') {
                let section = &line[1..end];
                // Handle subsections like [remote "origin"]
                current_section = section
                    .replace('"', "")
                    .replace(' ', ".")
                    .to_lowercase();
            }
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim().to_lowercase();
            let value = value.trim().to_string();
            let full_key = if current_section.is_empty() {
                key
            } else {
                format!("{}.{}", current_section, key)
            };
            config.set(full_key, value, scope);
        }
    }

    config
}

/// Trait for accessing git configuration in a repository.
pub trait ConfigReader {
    /// The error type for config operations.
    type Error: std::error::Error;

    /// Read the merged configuration for the repository.
    fn read_config(&self) -> Result<GitConfig, Self::Error>;

    /// Read configuration from a specific scope.
    fn read_config_scope(&self, scope: ConfigScope) -> Result<GitConfig, Self::Error>;

    /// Get a single config value by key.
    fn get_config(&self, key: &str) -> Result<Option<String>, Self::Error>;

    /// Set a config value in the given scope.
    fn set_config(&self, key: &str, value: &str, scope: ConfigScope) -> Result<(), Self::Error>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_config() {
        let content = r#"
[user]
    name = Alice
    email = alice@example.com

[remote "origin"]
    url = https://github.com/example/repo.git
    fetch = +refs/heads/*:refs/remotes/origin/*

[core]
    hooksPath = .githooks
"#;
        let config = parse_config_file(content, ConfigScope::Local);
        assert_eq!(config.user_name(), Some("Alice"));
        assert_eq!(config.user_email(), Some("alice@example.com"));
        assert_eq!(
            config.remote_url("origin"),
            Some("https://github.com/example/repo.git")
        );
        assert_eq!(config.hooks_path(), PathBuf::from(".githooks"));
    }

    #[test]
    fn identity_formatting() {
        let id = GitIdentity {
            name: "Bob".to_string(),
            email: "bob@example.com".to_string(),
            signing_key: None,
        };
        assert_eq!(id.to_author_string(), "Bob <bob@example.com>");
    }
}
