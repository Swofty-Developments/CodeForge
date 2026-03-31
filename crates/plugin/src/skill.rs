//! Skill types for slash-command-style plugin extensions.

use serde::{Deserialize, Serialize};

/// A skill is a named capability that can be triggered by slash commands
/// or pattern matching in user input.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Skill {
    /// Unique skill identifier.
    pub id: String,
    /// Human-readable skill name.
    pub name: String,
    /// Description of what this skill does.
    pub description: String,
    /// The slash command trigger (e.g., `/review-pr`).
    pub trigger: String,
    /// Additional patterns that can trigger this skill.
    pub trigger_patterns: Vec<String>,
    /// The plugin that provides this skill.
    pub plugin_id: String,
    /// How the skill is executed.
    pub execution: SkillExecution,
    /// Whether the skill is currently enabled.
    pub enabled: bool,
    /// Usage examples for display in help.
    pub examples: Vec<SkillExample>,
}

/// How a skill is executed when triggered.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum SkillExecution {
    /// The skill produces a prompt that is sent to the AI.
    Prompt {
        /// The prompt template (may contain `{{input}}` placeholder).
        template: String,
    },
    /// The skill runs a system command.
    Command {
        /// The command to execute.
        command: String,
        /// Command arguments (may contain `{{input}}` placeholder).
        args: Vec<String>,
    },
    /// The skill delegates to an MCP tool.
    McpTool {
        /// The MCP server name.
        server: String,
        /// The tool name on the MCP server.
        tool: String,
    },
}

/// An example of skill usage for documentation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillExample {
    /// The input the user would type.
    pub input: String,
    /// Description of what happens.
    pub description: String,
}

impl Skill {
    /// Create a new prompt-based skill.
    pub fn prompt_skill(
        id: impl Into<String>,
        name: impl Into<String>,
        trigger: impl Into<String>,
        template: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            name: name.into(),
            description: String::new(),
            trigger: trigger.into(),
            trigger_patterns: Vec::new(),
            plugin_id: String::new(),
            execution: SkillExecution::Prompt {
                template: template.into(),
            },
            enabled: true,
            examples: Vec::new(),
        }
    }

    /// Set the description for this skill.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    /// Add an alternative trigger pattern.
    pub fn with_pattern(mut self, pattern: impl Into<String>) -> Self {
        self.trigger_patterns.push(pattern.into());
        self
    }

    /// Add a usage example.
    pub fn with_example(mut self, input: impl Into<String>, desc: impl Into<String>) -> Self {
        self.examples.push(SkillExample {
            input: input.into(),
            description: desc.into(),
        });
        self
    }

    /// Check if the given input matches this skill's trigger or patterns.
    pub fn matches(&self, input: &str) -> bool {
        let lower = input.to_lowercase();
        if lower.starts_with(&self.trigger.to_lowercase()) {
            return true;
        }
        self.trigger_patterns
            .iter()
            .any(|p| lower.contains(&p.to_lowercase()))
    }
}
