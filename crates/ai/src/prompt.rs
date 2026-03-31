//! Prompt templating, system prompt building, and conversation context assembly.
//!
//! Provides types for constructing prompts with variable interpolation,
//! building system prompts from components, and parsing CLAUDE.md files.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A prompt template with variable placeholders.
///
/// Placeholders use the `{{variable}}` syntax and are replaced at render time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// The raw template string with `{{variable}}` placeholders.
    pub template: String,
    /// Default values for variables.
    pub defaults: HashMap<String, String>,
    /// Description of what this template is for.
    pub description: Option<String>,
}

impl PromptTemplate {
    /// Create a new template from a string.
    pub fn new(template: impl Into<String>) -> Self {
        Self {
            template: template.into(),
            defaults: HashMap::new(),
            description: None,
        }
    }

    /// Set a default value for a variable.
    pub fn with_default(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.defaults.insert(key.into(), value.into());
        self
    }

    /// Set the description.
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }

    /// Render the template by replacing variables with provided values.
    /// Falls back to defaults for missing variables.
    pub fn render(&self, variables: &HashMap<String, String>) -> String {
        let mut result = self.template.clone();
        // Collect all variable references.
        let var_names = self.extract_variables();
        for name in var_names {
            let placeholder = format!("{{{{{name}}}}}");
            let value = variables
                .get(&name)
                .or_else(|| self.defaults.get(&name))
                .cloned()
                .unwrap_or_default();
            result = result.replace(&placeholder, &value);
        }
        result
    }

    /// Render with a single variable.
    pub fn render_with(&self, key: &str, value: &str) -> String {
        let mut vars = HashMap::new();
        vars.insert(key.to_string(), value.to_string());
        self.render(&vars)
    }

    /// Extract all variable names from the template.
    pub fn extract_variables(&self) -> Vec<String> {
        let mut vars = Vec::new();
        let mut remaining = self.template.as_str();
        while let Some(start) = remaining.find("{{") {
            remaining = &remaining[start + 2..];
            if let Some(end) = remaining.find("}}") {
                let name = remaining[..end].trim().to_string();
                if !vars.contains(&name) {
                    vars.push(name);
                }
                remaining = &remaining[end + 2..];
            }
        }
        vars
    }

    /// Check if all required variables have values (from defaults or provided).
    pub fn validate(&self, variables: &HashMap<String, String>) -> Vec<String> {
        self.extract_variables()
            .into_iter()
            .filter(|name| !variables.contains_key(name) && !self.defaults.contains_key(name))
            .collect()
    }
}

impl fmt::Display for PromptTemplate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.template)
    }
}

/// Builder for constructing system prompts from components.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SystemPromptBuilder {
    /// The base role description.
    role: Option<String>,
    /// Capability descriptions.
    capabilities: Vec<String>,
    /// Behavioral guidelines.
    guidelines: Vec<String>,
    /// Context sections (e.g., from CLAUDE.md).
    context_sections: Vec<ContextSection>,
    /// Output format instructions.
    format_instructions: Option<String>,
    /// Safety/constraint instructions.
    constraints: Vec<String>,
}

/// A named section of context to include in the system prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextSection {
    /// The section heading.
    pub heading: String,
    /// The section content.
    pub content: String,
    /// Priority for ordering (lower = higher priority).
    pub priority: u32,
}

impl SystemPromptBuilder {
    /// Create a new system prompt builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the role description (e.g., "You are a code assistant...").
    pub fn role(mut self, role: impl Into<String>) -> Self {
        self.role = Some(role.into());
        self
    }

    /// Add a capability description.
    pub fn capability(mut self, cap: impl Into<String>) -> Self {
        self.capabilities.push(cap.into());
        self
    }

    /// Add a behavioral guideline.
    pub fn guideline(mut self, guideline: impl Into<String>) -> Self {
        self.guidelines.push(guideline.into());
        self
    }

    /// Add a context section.
    pub fn context(mut self, heading: impl Into<String>, content: impl Into<String>) -> Self {
        self.context_sections.push(ContextSection {
            heading: heading.into(),
            content: content.into(),
            priority: self.context_sections.len() as u32,
        });
        self
    }

    /// Add a prioritized context section.
    pub fn context_with_priority(
        mut self,
        heading: impl Into<String>,
        content: impl Into<String>,
        priority: u32,
    ) -> Self {
        self.context_sections.push(ContextSection {
            heading: heading.into(),
            content: content.into(),
            priority,
        });
        self
    }

    /// Set output format instructions.
    pub fn format(mut self, instructions: impl Into<String>) -> Self {
        self.format_instructions = Some(instructions.into());
        self
    }

    /// Add a constraint/safety rule.
    pub fn constraint(mut self, constraint: impl Into<String>) -> Self {
        self.constraints.push(constraint.into());
        self
    }

    /// Build the final system prompt string.
    pub fn build(&self) -> String {
        let mut parts = Vec::new();

        if let Some(ref role) = self.role {
            parts.push(role.clone());
        }

        if !self.capabilities.is_empty() {
            let mut section = String::from("Your capabilities:\n");
            for cap in &self.capabilities {
                section.push_str(&format!("- {cap}\n"));
            }
            parts.push(section);
        }

        if !self.guidelines.is_empty() {
            let mut section = String::from("Guidelines:\n");
            for g in &self.guidelines {
                section.push_str(&format!("- {g}\n"));
            }
            parts.push(section);
        }

        // Sort context sections by priority.
        let mut sections = self.context_sections.clone();
        sections.sort_by_key(|s| s.priority);
        for section in &sections {
            parts.push(format!("# {}\n{}", section.heading, section.content));
        }

        if !self.constraints.is_empty() {
            let mut section = String::from("Constraints:\n");
            for c in &self.constraints {
                section.push_str(&format!("- {c}\n"));
            }
            parts.push(section);
        }

        if let Some(ref fmt) = self.format_instructions {
            parts.push(format!("Output format:\n{fmt}"));
        }

        parts.join("\n\n")
    }
}

/// A parsed CLAUDE.md file containing project-level instructions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClaudeMdFile {
    /// The raw content of the file.
    pub raw_content: String,
    /// The file path it was loaded from.
    pub source_path: Option<String>,
    /// Parsed sections by heading.
    pub sections: Vec<ClaudeMdSection>,
}

/// A section within a CLAUDE.md file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaudeMdSection {
    /// The heading text (empty for content before the first heading).
    pub heading: String,
    /// The heading level (1 = #, 2 = ##, etc). 0 for preamble.
    pub level: u8,
    /// The section content.
    pub content: String,
}

impl ClaudeMdFile {
    /// Parse a CLAUDE.md file from its text content.
    pub fn parse(content: &str) -> Self {
        let mut sections = Vec::new();
        let mut current_heading = String::new();
        let mut current_level: u8 = 0;
        let mut current_content = String::new();

        for line in content.lines() {
            if line.starts_with('#') {
                // Save previous section.
                if !current_content.is_empty() || !current_heading.is_empty() {
                    sections.push(ClaudeMdSection {
                        heading: current_heading.clone(),
                        level: current_level,
                        content: current_content.trim().to_string(),
                    });
                }

                // Parse heading level.
                let level = line.chars().take_while(|c| *c == '#').count() as u8;
                let heading = line.trim_start_matches('#').trim().to_string();
                current_heading = heading;
                current_level = level;
                current_content = String::new();
            } else {
                if !current_content.is_empty() || !line.trim().is_empty() {
                    current_content.push_str(line);
                    current_content.push('\n');
                }
            }
        }

        // Save the last section.
        if !current_content.is_empty() || !current_heading.is_empty() {
            sections.push(ClaudeMdSection {
                heading: current_heading,
                level: current_level,
                content: current_content.trim().to_string(),
            });
        }

        Self {
            raw_content: content.to_string(),
            source_path: None,
            sections,
        }
    }

    /// Set the source path.
    pub fn with_source(mut self, path: impl Into<String>) -> Self {
        self.source_path = Some(path.into());
        self
    }

    /// Get a section by heading (case-insensitive).
    pub fn get_section(&self, heading: &str) -> Option<&ClaudeMdSection> {
        let heading_lower = heading.to_lowercase();
        self.sections
            .iter()
            .find(|s| s.heading.to_lowercase() == heading_lower)
    }

    /// Get all top-level sections (heading level 1).
    pub fn top_level_sections(&self) -> Vec<&ClaudeMdSection> {
        self.sections.iter().filter(|s| s.level == 1).collect()
    }

    /// Convert all sections to context sections for the system prompt builder.
    pub fn to_context_sections(&self) -> Vec<ContextSection> {
        self.sections
            .iter()
            .enumerate()
            .map(|(i, s)| ContextSection {
                heading: if s.heading.is_empty() {
                    "Project Instructions".to_string()
                } else {
                    s.heading.clone()
                },
                content: s.content.clone(),
                priority: i as u32,
            })
            .collect()
    }
}

/// Assemble a conversation context from various sources.
#[derive(Debug, Clone, Default)]
pub struct ConversationContext {
    /// System prompt parts to combine.
    parts: Vec<ContextPart>,
}

/// A part of the conversation context with a type label.
#[derive(Debug, Clone)]
pub struct ContextPart {
    /// The type of context.
    pub kind: ContextKind,
    /// The content.
    pub content: String,
    /// Token estimate for budget tracking.
    pub estimated_tokens: usize,
}

/// The type of context being included.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ContextKind {
    /// Base system instructions.
    SystemInstructions,
    /// Project-specific instructions (from CLAUDE.md).
    ProjectInstructions,
    /// Repository structure / file tree.
    RepoStructure,
    /// File contents for context.
    FileContent,
    /// Previous conversation summary.
    ConversationSummary,
    /// Tool definitions.
    ToolDefinitions,
    /// User preferences.
    UserPreferences,
}

impl ConversationContext {
    /// Create a new empty context.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a context part.
    pub fn add(&mut self, kind: ContextKind, content: impl Into<String>, tokens: usize) {
        self.parts.push(ContextPart {
            kind,
            content: content.into(),
            estimated_tokens: tokens,
        });
    }

    /// Return the total estimated tokens.
    pub fn total_tokens(&self) -> usize {
        self.parts.iter().map(|p| p.estimated_tokens).sum()
    }

    /// Return all parts.
    pub fn parts(&self) -> &[ContextPart] {
        &self.parts
    }

    /// Assemble all parts into a single string.
    pub fn assemble(&self) -> String {
        self.parts
            .iter()
            .map(|p| p.content.as_str())
            .collect::<Vec<_>>()
            .join("\n\n")
    }

    /// Truncate to fit within a token budget, removing lowest-priority items first.
    pub fn truncate_to_budget(&mut self, max_tokens: usize) {
        while self.total_tokens() > max_tokens && !self.parts.is_empty() {
            self.parts.pop();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_rendering() {
        let tmpl = PromptTemplate::new("Hello, {{name}}! You are a {{role}}.")
            .with_default("role", "assistant");
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "Alice".to_string());
        let rendered = tmpl.render(&vars);
        assert_eq!(rendered, "Hello, Alice! You are a assistant.");
    }

    #[test]
    fn extract_variables() {
        let tmpl = PromptTemplate::new("{{a}} and {{b}} and {{a}}");
        let vars = tmpl.extract_variables();
        assert_eq!(vars, vec!["a", "b"]);
    }

    #[test]
    fn system_prompt_builder() {
        let prompt = SystemPromptBuilder::new()
            .role("You are a helpful assistant.")
            .capability("Code analysis")
            .guideline("Be concise")
            .build();
        assert!(prompt.contains("helpful assistant"));
        assert!(prompt.contains("Code analysis"));
        assert!(prompt.contains("Be concise"));
    }

    #[test]
    fn claude_md_parsing() {
        let content = "# Project\nThis is a project.\n\n## Rules\n- Rule 1\n- Rule 2\n";
        let parsed = ClaudeMdFile::parse(content);
        assert_eq!(parsed.sections.len(), 2);
        assert_eq!(parsed.sections[0].heading, "Project");
        assert_eq!(parsed.sections[1].heading, "Rules");
    }
}
