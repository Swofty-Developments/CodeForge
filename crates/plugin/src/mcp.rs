//! MCP (Model Context Protocol) server configuration and connection types.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Configuration for an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    /// Unique name for this server configuration.
    pub name: String,
    /// How to connect to the MCP server.
    pub connection: McpConnection,
    /// Whether this server is currently enabled.
    pub enabled: bool,
    /// Environment variables to set when launching the server.
    pub env: Vec<EnvVar>,
    /// Optional timeout in seconds for server operations.
    pub timeout_secs: Option<u64>,
    /// Capabilities to request from the server.
    pub capabilities: Vec<McpCapability>,
}

/// How to connect to an MCP server.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum McpConnection {
    /// Launch the server as a child process, communicating via stdin/stdout.
    Stdio {
        /// The command to execute.
        command: String,
        /// Command arguments.
        args: Vec<String>,
        /// Working directory for the process.
        cwd: Option<PathBuf>,
    },
    /// Connect to a running server via HTTP.
    Http {
        /// The server URL.
        url: String,
        /// Optional authentication token.
        auth_token: Option<String>,
    },
    /// Connect to a running server via Server-Sent Events.
    Sse {
        /// The SSE endpoint URL.
        url: String,
        /// Optional authentication token.
        auth_token: Option<String>,
    },
}

impl McpConnection {
    /// Create a stdio connection.
    pub fn stdio(command: impl Into<String>, args: Vec<String>) -> Self {
        Self::Stdio {
            command: command.into(),
            args,
            cwd: None,
        }
    }

    /// Create an HTTP connection.
    pub fn http(url: impl Into<String>) -> Self {
        Self::Http {
            url: url.into(),
            auth_token: None,
        }
    }

    /// Create an SSE connection.
    pub fn sse(url: impl Into<String>) -> Self {
        Self::Sse {
            url: url.into(),
            auth_token: None,
        }
    }

    /// Returns the connection type as a string.
    pub fn connection_type(&self) -> &'static str {
        match self {
            Self::Stdio { .. } => "stdio",
            Self::Http { .. } => "http",
            Self::Sse { .. } => "sse",
        }
    }
}

/// Capabilities that an MCP server can provide.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum McpCapability {
    /// The server provides tools.
    Tools,
    /// The server provides resource access.
    Resources,
    /// The server provides prompt templates.
    Prompts,
    /// The server supports sampling (generating completions).
    Sampling,
    /// The server supports logging.
    Logging,
}

/// An environment variable key-value pair.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnvVar {
    /// The variable name.
    pub key: String,
    /// The variable value.
    pub value: String,
}

impl EnvVar {
    /// Create a new environment variable.
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

impl McpServerConfig {
    /// Create a new MCP server config with a stdio connection.
    pub fn new_stdio(
        name: impl Into<String>,
        command: impl Into<String>,
        args: Vec<String>,
    ) -> Self {
        Self {
            name: name.into(),
            connection: McpConnection::stdio(command, args),
            enabled: true,
            env: Vec::new(),
            timeout_secs: Some(30),
            capabilities: Vec::new(),
        }
    }

    /// Returns `true` if this server has the given capability.
    pub fn has_capability(&self, cap: &McpCapability) -> bool {
        self.capabilities.contains(cap)
    }
}
