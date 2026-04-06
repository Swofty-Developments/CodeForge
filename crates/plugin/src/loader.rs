//! Plugin loader abstractions for different plugin formats.
//!
//! Provides traits and types for loading plugins from WASM modules,
//! native shared libraries, or script files, with lifecycle management.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Default per-plugin memory limit (256 MiB).
const DEFAULT_PLUGIN_MEMORY_BYTES: u64 = 256 * 1024 * 1024;

/// The format/type of a plugin binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PluginFormat {
    /// WebAssembly module.
    Wasm,
    /// Native shared library (.dylib, .so, .dll).
    Native,
    /// JavaScript/TypeScript script.
    JavaScript,
    /// Python script.
    Python,
    /// Lua script.
    Lua,
}

impl PluginFormat {
    /// Return the file extension for this format.
    pub fn extension(&self) -> &'static str {
        match self {
            PluginFormat::Wasm => "wasm",
            PluginFormat::Native => {
                if cfg!(target_os = "macos") {
                    "dylib"
                } else if cfg!(target_os = "windows") {
                    "dll"
                } else {
                    "so"
                }
            }
            PluginFormat::JavaScript => "js",
            PluginFormat::Python => "py",
            PluginFormat::Lua => "lua",
        }
    }

    /// Detect the format from a file extension.
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_lowercase().as_str() {
            "wasm" => Some(PluginFormat::Wasm),
            "dylib" | "so" | "dll" => Some(PluginFormat::Native),
            "js" | "ts" | "mjs" => Some(PluginFormat::JavaScript),
            "py" => Some(PluginFormat::Python),
            "lua" => Some(PluginFormat::Lua),
            _ => None,
        }
    }

    /// Detect the format from a file path.
    pub fn from_path(path: &Path) -> Option<Self> {
        path.extension()
            .and_then(|ext| ext.to_str())
            .and_then(Self::from_extension)
    }

    /// Whether this format runs in a sandboxed environment.
    pub fn is_sandboxed(&self) -> bool {
        matches!(self, PluginFormat::Wasm)
    }

    /// Whether this format requires an interpreter.
    pub fn requires_interpreter(&self) -> bool {
        matches!(
            self,
            PluginFormat::JavaScript | PluginFormat::Python | PluginFormat::Lua
        )
    }
}

impl fmt::Display for PluginFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginFormat::Wasm => write!(f, "WASM"),
            PluginFormat::Native => write!(f, "Native"),
            PluginFormat::JavaScript => write!(f, "JavaScript"),
            PluginFormat::Python => write!(f, "Python"),
            PluginFormat::Lua => write!(f, "Lua"),
        }
    }
}

/// The lifecycle state of a loaded plugin.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PluginLifecycleState {
    /// Plugin binary has been found but not loaded.
    Discovered,
    /// Plugin is being loaded into memory.
    Loading,
    /// Plugin is loaded and being initialized.
    Initializing,
    /// Plugin is fully initialized and ready.
    Ready,
    /// Plugin is running / processing.
    Running,
    /// Plugin is being stopped.
    Stopping,
    /// Plugin has been stopped.
    Stopped,
    /// Plugin encountered an error during loading or execution.
    Error,
    /// Plugin has been unloaded from memory.
    Unloaded,
}

impl fmt::Display for PluginLifecycleState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PluginLifecycleState::Discovered => write!(f, "discovered"),
            PluginLifecycleState::Loading => write!(f, "loading"),
            PluginLifecycleState::Initializing => write!(f, "initializing"),
            PluginLifecycleState::Ready => write!(f, "ready"),
            PluginLifecycleState::Running => write!(f, "running"),
            PluginLifecycleState::Stopping => write!(f, "stopping"),
            PluginLifecycleState::Stopped => write!(f, "stopped"),
            PluginLifecycleState::Error => write!(f, "error"),
            PluginLifecycleState::Unloaded => write!(f, "unloaded"),
        }
    }
}

/// The result of attempting to load a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadResult {
    /// The plugin identifier.
    pub plugin_id: String,
    /// Whether the load succeeded.
    pub success: bool,
    /// The format that was loaded.
    pub format: PluginFormat,
    /// Error message if loading failed.
    pub error: Option<String>,
    /// Time taken to load in milliseconds.
    pub load_time_ms: u64,
    /// Memory usage after loading in bytes.
    pub memory_bytes: Option<u64>,
    /// Exported functions/symbols discovered.
    pub exports: Vec<String>,
}

impl LoadResult {
    /// Create a successful load result.
    pub fn success(plugin_id: impl Into<String>, format: PluginFormat) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            success: true,
            format,
            error: None,
            load_time_ms: 0,
            memory_bytes: None,
            exports: Vec::new(),
        }
    }

    /// Create a failed load result.
    pub fn failure(
        plugin_id: impl Into<String>,
        format: PluginFormat,
        error: impl Into<String>,
    ) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            success: false,
            format,
            error: Some(error.into()),
            load_time_ms: 0,
            memory_bytes: None,
            exports: Vec::new(),
        }
    }

    /// Set the load time.
    pub fn with_load_time(mut self, ms: u64) -> Self {
        self.load_time_ms = ms;
        self
    }

    /// Set discovered exports.
    pub fn with_exports(mut self, exports: Vec<String>) -> Self {
        self.exports = exports;
        self
    }
}

impl fmt::Display for LoadResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.success {
            write!(
                f,
                "{}: loaded ({}, {}ms, {} exports)",
                self.plugin_id,
                self.format,
                self.load_time_ms,
                self.exports.len()
            )
        } else {
            write!(
                f,
                "{}: failed ({})",
                self.plugin_id,
                self.error.as_deref().unwrap_or("unknown error")
            )
        }
    }
}

/// Configuration for the plugin loader.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoaderConfig {
    /// Directories to search for plugins.
    pub search_paths: Vec<PathBuf>,
    /// Supported plugin formats.
    pub supported_formats: Vec<PluginFormat>,
    /// Timeout for loading a single plugin.
    pub load_timeout: Duration,
    /// Timeout for initializing a plugin.
    pub init_timeout: Duration,
    /// Maximum memory per plugin in bytes.
    pub max_memory_per_plugin: u64,
    /// Whether to validate plugin signatures.
    pub verify_signatures: bool,
    /// Whether to allow native (unsafe) plugins.
    pub allow_native: bool,
}

impl Default for LoaderConfig {
    fn default() -> Self {
        Self {
            search_paths: Vec::new(),
            supported_formats: vec![PluginFormat::Wasm, PluginFormat::JavaScript],
            load_timeout: Duration::from_secs(10),
            init_timeout: Duration::from_secs(5),
            max_memory_per_plugin: DEFAULT_PLUGIN_MEMORY_BYTES,
            verify_signatures: false,
            allow_native: false,
        }
    }
}

/// Information about a discovered plugin binary.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginBinary {
    /// Path to the plugin file.
    pub path: PathBuf,
    /// Detected format.
    pub format: PluginFormat,
    /// File size in bytes.
    pub size_bytes: u64,
    /// SHA-256 hash of the file (if computed).
    pub hash: Option<String>,
    /// Whether the binary is signed.
    pub signed: bool,
}

impl PluginBinary {
    /// Create from a path with auto-detected format.
    pub fn from_path(path: impl Into<PathBuf>) -> Option<Self> {
        let path = path.into();
        let format = PluginFormat::from_path(&path)?;
        Some(Self {
            path,
            format,
            size_bytes: 0,
            hash: None,
            signed: false,
        })
    }
}

impl fmt::Display for PluginBinary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} ({}, {} bytes)",
            self.path.display(),
            self.format,
            self.size_bytes
        )
    }
}

/// Trait for loading plugins from different formats.
pub trait PluginLoader {
    /// The error type for load operations.
    type Error: std::error::Error;

    /// Check if this loader supports the given format.
    fn supports(&self, format: PluginFormat) -> bool;

    /// Discover plugins in the configured search paths.
    fn discover(&self) -> Result<Vec<PluginBinary>, Self::Error>;

    /// Load a plugin from a binary.
    fn load(&self, binary: &PluginBinary) -> Result<LoadResult, Self::Error>;

    /// Initialize a loaded plugin.
    fn init(&self, plugin_id: &str) -> Result<(), Self::Error>;

    /// Start a plugin (begin processing events).
    fn start(&self, plugin_id: &str) -> Result<(), Self::Error>;

    /// Stop a running plugin.
    fn stop(&self, plugin_id: &str) -> Result<(), Self::Error>;

    /// Unload a plugin from memory.
    fn unload(&self, plugin_id: &str) -> Result<(), Self::Error>;

    /// Get the current lifecycle state of a plugin.
    fn state(&self, plugin_id: &str) -> Option<PluginLifecycleState>;
}

/// Summary of loaded plugins.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LoaderSummary {
    /// Total plugins discovered.
    pub discovered: usize,
    /// Successfully loaded.
    pub loaded: usize,
    /// Failed to load.
    pub failed: usize,
    /// Currently running.
    pub running: usize,
    /// Load failures by plugin ID.
    pub failures: HashMap<String, String>,
}

impl fmt::Display for LoaderSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} discovered, {} loaded, {} running, {} failed",
            self.discovered, self.loaded, self.running, self.failed
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_detection() {
        assert_eq!(
            PluginFormat::from_extension("wasm"),
            Some(PluginFormat::Wasm)
        );
        assert_eq!(
            PluginFormat::from_extension("js"),
            Some(PluginFormat::JavaScript)
        );
        assert_eq!(PluginFormat::from_extension("txt"), None);
    }

    #[test]
    fn format_sandboxing() {
        assert!(PluginFormat::Wasm.is_sandboxed());
        assert!(!PluginFormat::Native.is_sandboxed());
    }

    #[test]
    fn load_result_display() {
        let result = LoadResult::success("my-plugin", PluginFormat::Wasm)
            .with_load_time(42)
            .with_exports(vec!["init".to_string(), "run".to_string()]);
        let display = result.to_string();
        assert!(display.contains("my-plugin"));
        assert!(display.contains("42ms"));
    }

    #[test]
    fn binary_from_path() {
        let binary = PluginBinary::from_path("/plugins/test.wasm").unwrap();
        assert_eq!(binary.format, PluginFormat::Wasm);
    }
}
