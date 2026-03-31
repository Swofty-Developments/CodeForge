//! Tool definition and approval types for AI tool use.

use serde::{Deserialize, Serialize};

/// Definition of a tool that the AI can invoke.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// The tool name (must be unique within a session).
    pub name: String,
    /// Human-readable description of what the tool does.
    pub description: String,
    /// JSON Schema describing the tool's input parameters.
    pub input_schema: serde_json::Value,
    /// Whether this tool requires explicit user approval.
    pub requires_approval: bool,
    /// Category for grouping related tools.
    pub category: ToolCategory,
}

impl ToolDefinition {
    /// Create a new tool definition.
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({"type": "object"}),
            requires_approval: false,
            category: ToolCategory::Other,
        }
    }

    /// Set the input schema for this tool.
    pub fn with_schema(mut self, schema: serde_json::Value) -> Self {
        self.input_schema = schema;
        self
    }

    /// Mark this tool as requiring user approval.
    pub fn with_approval(mut self) -> Self {
        self.requires_approval = true;
        self
    }

    /// Set the category for this tool.
    pub fn in_category(mut self, category: ToolCategory) -> Self {
        self.category = category;
        self
    }
}

/// Categories for organizing tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// File system operations (read, write, list).
    FileSystem,
    /// Shell / command execution.
    Shell,
    /// Search and code navigation.
    Search,
    /// Git operations.
    Git,
    /// Web / network operations.
    Web,
    /// MCP server tools.
    Mcp,
    /// Uncategorized tools.
    Other,
}

/// The result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool use ID this result corresponds to.
    pub tool_use_id: String,
    /// The output content (typically text).
    pub output: String,
    /// Whether the tool execution encountered an error.
    pub is_error: bool,
    /// How long the tool took to execute, in milliseconds.
    pub duration_ms: Option<u64>,
}

impl ToolResult {
    /// Create a successful tool result.
    pub fn success(tool_use_id: impl Into<String>, output: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            output: output.into(),
            is_error: false,
            duration_ms: None,
        }
    }

    /// Create an error tool result.
    pub fn error(tool_use_id: impl Into<String>, error: impl Into<String>) -> Self {
        Self {
            tool_use_id: tool_use_id.into(),
            output: error.into(),
            is_error: true,
            duration_ms: None,
        }
    }

    /// Set the execution duration.
    pub fn with_duration(mut self, ms: u64) -> Self {
        self.duration_ms = Some(ms);
        self
    }
}

/// The approval decision for a tool use request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolApproval {
    /// The tool use is allowed to proceed.
    Allow,
    /// The tool use is denied.
    Deny {
        /// Reason for denial.
        reason: String,
    },
    /// The user should be prompted for approval.
    AskUser {
        /// A message to display to the user.
        prompt: String,
    },
}

impl ToolApproval {
    /// Create an `Allow` approval.
    pub fn allow() -> Self {
        Self::Allow
    }

    /// Create a `Deny` approval with a reason.
    pub fn deny(reason: impl Into<String>) -> Self {
        Self::Deny {
            reason: reason.into(),
        }
    }

    /// Create an `AskUser` approval with a prompt message.
    pub fn ask(prompt: impl Into<String>) -> Self {
        Self::AskUser {
            prompt: prompt.into(),
        }
    }

    /// Returns `true` if the tool use is allowed.
    pub fn is_allowed(&self) -> bool {
        matches!(self, Self::Allow)
    }
}
