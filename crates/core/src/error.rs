//! Unified error types for the CodeForge application.

use std::fmt;

/// Primary error type encompassing all failure modes in CodeForge.
#[derive(Debug, thiserror::Error)]
pub enum CodeForgeError {
    /// An error originating from session management.
    #[error("session error: {message}")]
    Session {
        /// Description of the session failure.
        message: String,
    },

    /// A database operation failed.
    #[error("database error: {message}")]
    Database {
        /// Description of the database failure.
        message: String,
    },

    /// A git operation failed.
    #[error("git error: {message}")]
    Git {
        /// Description of the git failure.
        message: String,
    },

    /// A GitHub API interaction failed.
    #[error("github error: {message}")]
    GitHub {
        /// Description of the GitHub failure.
        message: String,
    },

    /// An MCP server or protocol error.
    #[error("mcp error: {message}")]
    Mcp {
        /// Description of the MCP failure.
        message: String,
    },

    /// An I/O operation failed.
    #[error("io error: {source}")]
    Io {
        /// The underlying I/O error.
        #[from]
        source: std::io::Error,
    },

    /// A configuration error.
    #[error("config error: {message}")]
    Config {
        /// Description of the configuration problem.
        message: String,
    },

    /// An error propagated from anyhow.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Convenience type alias for results using [`CodeForgeError`].
pub type Result<T> = std::result::Result<T, CodeForgeError>;

impl CodeForgeError {
    /// Create a session error with the given message.
    pub fn session(msg: impl fmt::Display) -> Self {
        Self::Session {
            message: msg.to_string(),
        }
    }

    /// Create a database error with the given message.
    pub fn database(msg: impl fmt::Display) -> Self {
        Self::Database {
            message: msg.to_string(),
        }
    }

    /// Create a git error with the given message.
    pub fn git(msg: impl fmt::Display) -> Self {
        Self::Git {
            message: msg.to_string(),
        }
    }

    /// Create a GitHub error with the given message.
    pub fn github(msg: impl fmt::Display) -> Self {
        Self::GitHub {
            message: msg.to_string(),
        }
    }

    /// Create an MCP error with the given message.
    pub fn mcp(msg: impl fmt::Display) -> Self {
        Self::Mcp {
            message: msg.to_string(),
        }
    }

    /// Create a config error with the given message.
    pub fn config(msg: impl fmt::Display) -> Self {
        Self::Config {
            message: msg.to_string(),
        }
    }

    /// Returns `true` if this is a retryable error (I/O or transient network issues).
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Io { .. } | Self::GitHub { .. } | Self::Mcp { .. })
    }
}
