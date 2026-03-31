//! Plugin sandboxing and capability-based access control.
//!
//! Provides types for configuring the security sandbox that plugins
//! run within, including filesystem, network, and process restrictions.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::path::PathBuf;

/// The overall security policy for a plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxPolicy {
    /// No restrictions (full host access). Use only for trusted plugins.
    Unrestricted,
    /// Standard restrictions with configurable capabilities.
    Standard,
    /// Maximum restrictions (minimal access). Default for untrusted plugins.
    Strict,
    /// Custom policy defined by specific capabilities.
    Custom,
}

impl Default for SandboxPolicy {
    fn default() -> Self {
        SandboxPolicy::Strict
    }
}

impl fmt::Display for SandboxPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxPolicy::Unrestricted => write!(f, "unrestricted"),
            SandboxPolicy::Standard => write!(f, "standard"),
            SandboxPolicy::Strict => write!(f, "strict"),
            SandboxPolicy::Custom => write!(f, "custom"),
        }
    }
}

/// A specific capability that can be granted to a sandboxed plugin.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum SandboxCapability {
    /// Read files within specific paths.
    FileRead {
        /// Allowed path prefixes.
        paths: Vec<PathBuf>,
    },
    /// Write files within specific paths.
    FileWrite {
        /// Allowed path prefixes.
        paths: Vec<PathBuf>,
    },
    /// Make network requests to specific hosts.
    NetworkAccess {
        /// Allowed host patterns (e.g., "api.example.com", "*.github.com").
        hosts: Vec<String>,
        /// Allowed ports. Empty means all ports.
        ports: Vec<u16>,
    },
    /// Spawn child processes.
    ProcessSpawn {
        /// Allowed executable names.
        executables: Vec<String>,
    },
    /// Access environment variables.
    EnvAccess {
        /// Specific variable names. Empty means all variables.
        variables: Vec<String>,
    },
    /// Access to the clipboard.
    ClipboardAccess,
    /// Access to system notifications.
    NotificationAccess,
    /// Access to the host's temporary directory.
    TempDirAccess,
}

impl fmt::Display for SandboxCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SandboxCapability::FileRead { paths } => {
                write!(f, "file-read({})", paths.len())
            }
            SandboxCapability::FileWrite { paths } => {
                write!(f, "file-write({})", paths.len())
            }
            SandboxCapability::NetworkAccess { hosts, .. } => {
                write!(f, "network({})", hosts.join(", "))
            }
            SandboxCapability::ProcessSpawn { executables } => {
                write!(f, "process({})", executables.join(", "))
            }
            SandboxCapability::EnvAccess { variables } => {
                if variables.is_empty() {
                    write!(f, "env(all)")
                } else {
                    write!(f, "env({})", variables.join(", "))
                }
            }
            SandboxCapability::ClipboardAccess => write!(f, "clipboard"),
            SandboxCapability::NotificationAccess => write!(f, "notifications"),
            SandboxCapability::TempDirAccess => write!(f, "temp-dir"),
        }
    }
}

/// Complete sandbox configuration for a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// The base security policy.
    pub policy: SandboxPolicy,
    /// Granted capabilities.
    pub capabilities: Vec<SandboxCapability>,
    /// Memory limit in bytes (0 = unlimited).
    pub memory_limit_bytes: u64,
    /// CPU time limit in seconds (0 = unlimited).
    pub cpu_time_limit_secs: u64,
    /// Maximum number of open file descriptors.
    pub max_file_descriptors: u32,
    /// Maximum number of concurrent threads.
    pub max_threads: u32,
    /// Whether to allow access to the host's stdin/stdout.
    pub allow_stdio: bool,
    /// Directories mapped into the sandbox.
    pub mount_points: Vec<MountPoint>,
}

/// A directory mount point in the sandbox.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MountPoint {
    /// Path on the host filesystem.
    pub host_path: PathBuf,
    /// Path inside the sandbox.
    pub guest_path: PathBuf,
    /// Whether the mount is read-only.
    pub read_only: bool,
}

impl MountPoint {
    /// Create a read-only mount point.
    pub fn read_only(host: impl Into<PathBuf>, guest: impl Into<PathBuf>) -> Self {
        Self {
            host_path: host.into(),
            guest_path: guest.into(),
            read_only: true,
        }
    }

    /// Create a read-write mount point.
    pub fn read_write(host: impl Into<PathBuf>, guest: impl Into<PathBuf>) -> Self {
        Self {
            host_path: host.into(),
            guest_path: guest.into(),
            read_only: false,
        }
    }
}

impl fmt::Display for MountPoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mode = if self.read_only { "ro" } else { "rw" };
        write!(
            f,
            "{} -> {} ({})",
            self.host_path.display(),
            self.guest_path.display(),
            mode
        )
    }
}

impl SandboxConfig {
    /// Create a strict sandbox configuration.
    pub fn strict() -> Self {
        Self {
            policy: SandboxPolicy::Strict,
            capabilities: Vec::new(),
            memory_limit_bytes: 256 * 1024 * 1024, // 256 MB
            cpu_time_limit_secs: 30,
            max_file_descriptors: 64,
            max_threads: 4,
            allow_stdio: false,
            mount_points: Vec::new(),
        }
    }

    /// Create a standard sandbox configuration.
    pub fn standard() -> Self {
        Self {
            policy: SandboxPolicy::Standard,
            capabilities: vec![
                SandboxCapability::TempDirAccess,
                SandboxCapability::NotificationAccess,
            ],
            memory_limit_bytes: 512 * 1024 * 1024, // 512 MB
            cpu_time_limit_secs: 120,
            max_file_descriptors: 256,
            max_threads: 16,
            allow_stdio: true,
            mount_points: Vec::new(),
        }
    }

    /// Create an unrestricted configuration (for trusted plugins).
    pub fn unrestricted() -> Self {
        Self {
            policy: SandboxPolicy::Unrestricted,
            capabilities: Vec::new(),
            memory_limit_bytes: 0,
            cpu_time_limit_secs: 0,
            max_file_descriptors: 0,
            max_threads: 0,
            allow_stdio: true,
            mount_points: Vec::new(),
        }
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: SandboxCapability) -> Self {
        self.capabilities.push(cap);
        self
    }

    /// Add a mount point.
    pub fn with_mount(mut self, mount: MountPoint) -> Self {
        self.mount_points.push(mount);
        self
    }

    /// Check if a specific file read is allowed.
    pub fn can_read_file(&self, path: &std::path::Path) -> bool {
        if self.policy == SandboxPolicy::Unrestricted {
            return true;
        }
        self.capabilities.iter().any(|cap| {
            if let SandboxCapability::FileRead { paths } = cap {
                paths.iter().any(|allowed| path.starts_with(allowed))
            } else {
                false
            }
        })
    }

    /// Check if a specific file write is allowed.
    pub fn can_write_file(&self, path: &std::path::Path) -> bool {
        if self.policy == SandboxPolicy::Unrestricted {
            return true;
        }
        self.capabilities.iter().any(|cap| {
            if let SandboxCapability::FileWrite { paths } = cap {
                paths.iter().any(|allowed| path.starts_with(allowed))
            } else {
                false
            }
        })
    }

    /// Check if network access to a host is allowed.
    pub fn can_access_host(&self, host: &str) -> bool {
        if self.policy == SandboxPolicy::Unrestricted {
            return true;
        }
        self.capabilities.iter().any(|cap| {
            if let SandboxCapability::NetworkAccess { hosts, .. } = cap {
                hosts.iter().any(|pattern| host_matches(host, pattern))
            } else {
                false
            }
        })
    }

    /// Check if spawning an executable is allowed.
    pub fn can_spawn(&self, executable: &str) -> bool {
        if self.policy == SandboxPolicy::Unrestricted {
            return true;
        }
        self.capabilities.iter().any(|cap| {
            if let SandboxCapability::ProcessSpawn { executables } = cap {
                executables.iter().any(|e| e == executable)
            } else {
                false
            }
        })
    }

    /// Validate that the sandbox configuration is consistent.
    pub fn validate(&self) -> Vec<String> {
        let mut issues = Vec::new();
        if self.policy == SandboxPolicy::Strict && !self.capabilities.is_empty() {
            // Strict policy with capabilities is fine but worth noting.
        }
        if self.memory_limit_bytes > 0 && self.memory_limit_bytes < 1024 * 1024 {
            issues.push("Memory limit is less than 1 MB".to_string());
        }
        for mount in &self.mount_points {
            if !mount.read_only && self.policy == SandboxPolicy::Strict {
                issues.push(format!(
                    "Read-write mount {} in strict policy",
                    mount.host_path.display()
                ));
            }
        }
        issues
    }
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self::strict()
    }
}

/// Check if a hostname matches a pattern (supports leading wildcard).
fn host_matches(host: &str, pattern: &str) -> bool {
    if pattern == host {
        return true;
    }
    if let Some(suffix) = pattern.strip_prefix("*.") {
        host.ends_with(suffix) && host.len() > suffix.len()
    } else {
        false
    }
}

/// A request from a sandboxed plugin to access a resource.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessRequest {
    /// The type of access requested.
    pub kind: AccessKind,
    /// The resource being accessed.
    pub resource: String,
    /// The plugin making the request.
    pub plugin_id: String,
    /// Whether the request was granted.
    pub granted: Option<bool>,
}

/// The type of resource access being requested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessKind {
    /// File read.
    FileRead,
    /// File write.
    FileWrite,
    /// Network connection.
    Network,
    /// Process spawn.
    Process,
    /// Environment variable.
    Environment,
}

impl fmt::Display for AccessKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AccessKind::FileRead => write!(f, "file:read"),
            AccessKind::FileWrite => write!(f, "file:write"),
            AccessKind::Network => write!(f, "network"),
            AccessKind::Process => write!(f, "process"),
            AccessKind::Environment => write!(f, "env"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strict_denies_all() {
        let config = SandboxConfig::strict();
        assert!(!config.can_read_file(std::path::Path::new("/etc/passwd")));
        assert!(!config.can_access_host("example.com"));
        assert!(!config.can_spawn("bash"));
    }

    #[test]
    fn file_read_capability() {
        let config = SandboxConfig::strict().with_capability(SandboxCapability::FileRead {
            paths: vec![PathBuf::from("/home/user/project")],
        });
        assert!(config.can_read_file(std::path::Path::new("/home/user/project/src/main.rs")));
        assert!(!config.can_read_file(std::path::Path::new("/etc/passwd")));
    }

    #[test]
    fn host_pattern_matching() {
        assert!(host_matches("api.github.com", "*.github.com"));
        assert!(host_matches("api.github.com", "api.github.com"));
        assert!(!host_matches("evil.com", "*.github.com"));
    }

    #[test]
    fn unrestricted_allows_all() {
        let config = SandboxConfig::unrestricted();
        assert!(config.can_read_file(std::path::Path::new("/anything")));
        assert!(config.can_access_host("anything.com"));
    }
}
