//! Agent configuration, capability definitions, and execution loop.
//!
//! Provides types for configuring AI agents with specific capabilities,
//! defining the agent loop (observe-think-act), and managing sub-agent
//! communication.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use uuid::Uuid;

/// Unique identifier for an agent instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AgentId(Uuid);

impl AgentId {
    /// Create a new random agent ID.
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for AgentId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for AgentId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "agent-{}", &self.0.to_string()[..8])
    }
}

/// Capabilities that an agent can be granted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AgentCapability {
    /// Can read files from the filesystem.
    FileRead,
    /// Can write/modify files.
    FileWrite,
    /// Can execute shell commands.
    ShellExec,
    /// Can make network requests.
    NetworkAccess,
    /// Can interact with git repositories.
    GitOperations,
    /// Can search codebases.
    CodeSearch,
    /// Can spawn sub-agents.
    SubAgentSpawn,
    /// Can access browser/web content.
    BrowserAccess,
    /// Can manage MCP servers.
    McpManagement,
    /// Can access databases.
    DatabaseAccess,
}

impl AgentCapability {
    /// Return all available capabilities.
    pub fn all() -> &'static [AgentCapability] {
        &[
            AgentCapability::FileRead,
            AgentCapability::FileWrite,
            AgentCapability::ShellExec,
            AgentCapability::NetworkAccess,
            AgentCapability::GitOperations,
            AgentCapability::CodeSearch,
            AgentCapability::SubAgentSpawn,
            AgentCapability::BrowserAccess,
            AgentCapability::McpManagement,
            AgentCapability::DatabaseAccess,
        ]
    }

    /// Whether this capability requires explicit user approval.
    pub fn requires_approval(&self) -> bool {
        matches!(
            self,
            AgentCapability::FileWrite
                | AgentCapability::ShellExec
                | AgentCapability::NetworkAccess
                | AgentCapability::GitOperations
        )
    }

    /// Return a human-readable description.
    pub fn description(&self) -> &'static str {
        match self {
            AgentCapability::FileRead => "Read files from the filesystem",
            AgentCapability::FileWrite => "Write and modify files",
            AgentCapability::ShellExec => "Execute shell commands",
            AgentCapability::NetworkAccess => "Make network requests",
            AgentCapability::GitOperations => "Perform git operations",
            AgentCapability::CodeSearch => "Search codebases",
            AgentCapability::SubAgentSpawn => "Spawn sub-agents",
            AgentCapability::BrowserAccess => "Access web content",
            AgentCapability::McpManagement => "Manage MCP servers",
            AgentCapability::DatabaseAccess => "Access databases",
        }
    }
}

impl fmt::Display for AgentCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.description())
    }
}

/// Configuration for an agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// The agent's unique ID.
    pub id: AgentId,
    /// Human-readable name for the agent.
    pub name: String,
    /// The model to use for this agent.
    pub model: String,
    /// System prompt for the agent.
    pub system_prompt: String,
    /// Granted capabilities.
    pub capabilities: Vec<AgentCapability>,
    /// Maximum number of steps before the agent is stopped.
    pub max_steps: u32,
    /// Maximum total tokens the agent can consume.
    pub max_tokens: usize,
    /// Temperature for model sampling.
    pub temperature: f64,
    /// Whether the agent can request human input.
    pub interactive: bool,
    /// Custom metadata / labels.
    pub metadata: HashMap<String, String>,
}

impl AgentConfig {
    /// Create a new agent config with defaults.
    pub fn new(name: impl Into<String>, model: impl Into<String>) -> Self {
        Self {
            id: AgentId::new(),
            name: name.into(),
            model: model.into(),
            system_prompt: String::new(),
            capabilities: Vec::new(),
            max_steps: 50,
            max_tokens: 100_000,
            temperature: 0.7,
            interactive: false,
            metadata: HashMap::new(),
        }
    }

    /// Set the system prompt.
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = prompt.into();
        self
    }

    /// Add a capability.
    pub fn with_capability(mut self, cap: AgentCapability) -> Self {
        if !self.capabilities.contains(&cap) {
            self.capabilities.push(cap);
        }
        self
    }

    /// Set all capabilities (grant full access).
    pub fn with_all_capabilities(mut self) -> Self {
        self.capabilities = AgentCapability::all().to_vec();
        self
    }

    /// Set the maximum steps.
    pub fn with_max_steps(mut self, steps: u32) -> Self {
        self.max_steps = steps;
        self
    }

    /// Check if the agent has a specific capability.
    pub fn has_capability(&self, cap: AgentCapability) -> bool {
        self.capabilities.contains(&cap)
    }
}

impl fmt::Display for AgentConfig {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Agent '{}' ({}), model={}, caps={}, max_steps={}",
            self.name,
            self.id,
            self.model,
            self.capabilities.len(),
            self.max_steps
        )
    }
}

/// The current state of an agent's execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AgentState {
    /// Agent has been created but not started.
    Idle,
    /// Agent is observing/gathering context.
    Observing,
    /// Agent is thinking/planning.
    Thinking,
    /// Agent is executing an action.
    Acting,
    /// Agent is waiting for user input.
    WaitingForInput,
    /// Agent is waiting for a sub-agent.
    WaitingForSubAgent,
    /// Agent has completed its task.
    Completed,
    /// Agent was stopped (by user or limit).
    Stopped,
    /// Agent encountered an error.
    Failed,
}

impl fmt::Display for AgentState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentState::Idle => write!(f, "idle"),
            AgentState::Observing => write!(f, "observing"),
            AgentState::Thinking => write!(f, "thinking"),
            AgentState::Acting => write!(f, "acting"),
            AgentState::WaitingForInput => write!(f, "waiting for input"),
            AgentState::WaitingForSubAgent => write!(f, "waiting for sub-agent"),
            AgentState::Completed => write!(f, "completed"),
            AgentState::Stopped => write!(f, "stopped"),
            AgentState::Failed => write!(f, "failed"),
        }
    }
}

/// A single step in the agent's execution loop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStep {
    /// Step number (0-based).
    pub step_number: u32,
    /// The action the agent decided to take.
    pub action: AgentAction,
    /// The result of the action.
    pub result: Option<ActionResult>,
    /// Tokens consumed by this step.
    pub tokens_used: usize,
    /// Duration of this step in milliseconds.
    pub duration_ms: u64,
}

/// An action that an agent can take.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentAction {
    /// Use a tool.
    ToolUse {
        /// The tool name.
        tool: String,
        /// The tool input.
        input: serde_json::Value,
    },
    /// Send a message to the user.
    Message {
        /// The message content.
        content: String,
    },
    /// Spawn a sub-agent for a delegated task.
    SpawnSubAgent {
        /// The sub-agent's name.
        name: String,
        /// The task to delegate.
        task: String,
    },
    /// Request input from the user.
    RequestInput {
        /// The prompt to show the user.
        prompt: String,
    },
    /// Complete the task.
    Complete {
        /// Summary of what was accomplished.
        summary: String,
    },
}

impl fmt::Display for AgentAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AgentAction::ToolUse { tool, .. } => write!(f, "tool:{tool}"),
            AgentAction::Message { .. } => write!(f, "message"),
            AgentAction::SpawnSubAgent { name, .. } => write!(f, "spawn:{name}"),
            AgentAction::RequestInput { .. } => write!(f, "request-input"),
            AgentAction::Complete { .. } => write!(f, "complete"),
        }
    }
}

/// The result of an agent action.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionResult {
    /// Whether the action succeeded.
    pub success: bool,
    /// The output/response from the action.
    pub output: String,
    /// Error message if the action failed.
    pub error: Option<String>,
}

impl ActionResult {
    /// Create a successful result.
    pub fn success(output: impl Into<String>) -> Self {
        Self {
            success: true,
            output: output.into(),
            error: None,
        }
    }

    /// Create a failed result.
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: String::new(),
            error: Some(error.into()),
        }
    }
}

/// A message in the agent communication protocol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentMessage {
    /// The sender agent ID.
    pub from: AgentId,
    /// The recipient agent ID.
    pub to: AgentId,
    /// The message kind.
    pub kind: AgentMessageKind,
    /// Correlation ID for request/response matching.
    pub correlation_id: Option<String>,
}

/// Types of messages exchanged between agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentMessageKind {
    /// Delegate a task.
    TaskRequest { task: String },
    /// Return a task result.
    TaskResult { result: ActionResult },
    /// Report progress.
    Progress { message: String, percent: Option<f64> },
    /// Request cancellation.
    Cancel,
}

/// Execution summary for an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunSummary {
    /// The agent ID.
    pub agent_id: AgentId,
    /// Total steps executed.
    pub steps: u32,
    /// Total tokens consumed.
    pub total_tokens: usize,
    /// Total duration in milliseconds.
    pub total_duration_ms: u64,
    /// Final state.
    pub final_state: AgentState,
    /// Tools used and their counts.
    pub tool_usage: HashMap<String, u32>,
    /// Number of sub-agents spawned.
    pub sub_agents_spawned: u32,
}

impl fmt::Display for AgentRunSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}: {} steps, {} tokens, {}ms, state={}",
            self.agent_id,
            self.steps,
            self.total_tokens,
            self.total_duration_ms,
            self.final_state
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn agent_config_builder() {
        let config = AgentConfig::new("coder", "claude-sonnet-4")
            .with_capability(AgentCapability::FileRead)
            .with_capability(AgentCapability::FileWrite)
            .with_max_steps(100);
        assert_eq!(config.capabilities.len(), 2);
        assert_eq!(config.max_steps, 100);
        assert!(config.has_capability(AgentCapability::FileRead));
        assert!(!config.has_capability(AgentCapability::ShellExec));
    }

    #[test]
    fn capability_approval() {
        assert!(AgentCapability::ShellExec.requires_approval());
        assert!(!AgentCapability::CodeSearch.requires_approval());
    }

    #[test]
    fn action_result() {
        let success = ActionResult::success("done");
        assert!(success.success);
        let fail = ActionResult::failure("oops");
        assert!(!fail.success);
    }
}
