//! AI model and provider type definitions.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

/// Available AI models.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Model {
    /// Claude Opus — most capable model.
    Opus,
    /// Claude Sonnet — balanced performance and cost.
    Sonnet,
    /// Claude Haiku — fastest and most affordable.
    Haiku,
    /// A custom or third-party model identifier.
    Custom(String),
}

impl fmt::Display for Model {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Opus => write!(f, "opus"),
            Self::Sonnet => write!(f, "sonnet"),
            Self::Haiku => write!(f, "haiku"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for Model {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "opus" => Self::Opus,
            "sonnet" => Self::Sonnet,
            "haiku" => Self::Haiku,
            other => Self::Custom(other.to_string()),
        })
    }
}

impl Model {
    /// Returns the maximum context window size for this model in tokens.
    pub fn max_context_tokens(&self) -> u64 {
        match self {
            Self::Opus => 1_000_000,
            Self::Sonnet => 200_000,
            Self::Haiku => 200_000,
            Self::Custom(_) => 200_000,
        }
    }

    /// Returns `true` if this is a first-party Anthropic model.
    pub fn is_anthropic(&self) -> bool {
        matches!(self, Self::Opus | Self::Sonnet | Self::Haiku)
    }
}

/// AI provider backend.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Provider {
    /// Anthropic's API (direct).
    Anthropic,
    /// Amazon Bedrock.
    Bedrock,
    /// Google Vertex AI.
    Vertex,
    /// A custom or self-hosted provider.
    Custom(String),
}

impl fmt::Display for Provider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Anthropic => write!(f, "anthropic"),
            Self::Bedrock => write!(f, "bedrock"),
            Self::Vertex => write!(f, "vertex"),
            Self::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl FromStr for Provider {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "anthropic" => Self::Anthropic,
            "bedrock" => Self::Bedrock,
            "vertex" => Self::Vertex,
            other => Self::Custom(other.to_string()),
        })
    }
}

/// Permission mode controlling how tool use is approved.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PermissionMode {
    /// All tool uses require explicit user approval.
    Strict,
    /// Safe tools are auto-approved; destructive tools require approval.
    Normal,
    /// All tool uses are auto-approved without prompting.
    Permissive,
}

impl fmt::Display for PermissionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Strict => write!(f, "strict"),
            Self::Normal => write!(f, "normal"),
            Self::Permissive => write!(f, "permissive"),
        }
    }
}

impl FromStr for PermissionMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "strict" => Ok(Self::Strict),
            "normal" => Ok(Self::Normal),
            "permissive" => Ok(Self::Permissive),
            other => Err(format!("unknown permission mode: {other}")),
        }
    }
}
