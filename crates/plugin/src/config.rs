//! Plugin configuration with typed settings, validation, and migration.
//!
//! Provides types for defining and managing plugin configuration with
//! schema validation, default values, and version migration support.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// A typed configuration value.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ConfigValue {
    /// A string value.
    String(String),
    /// An integer value.
    Integer(i64),
    /// A floating-point value.
    Float(f64),
    /// A boolean value.
    Boolean(bool),
    /// A list of values.
    List(Vec<ConfigValue>),
    /// A nested map of values.
    Map(HashMap<String, ConfigValue>),
    /// A null/unset value.
    Null,
}

impl ConfigValue {
    /// Get as a string, if it is one.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            ConfigValue::String(s) => Some(s),
            _ => None,
        }
    }

    /// Get as an integer, if it is one.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            ConfigValue::Integer(n) => Some(*n),
            _ => None,
        }
    }

    /// Get as a float, if it is one (also converts integers).
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            ConfigValue::Float(f) => Some(*f),
            ConfigValue::Integer(n) => Some(*n as f64),
            _ => None,
        }
    }

    /// Get as a boolean, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            ConfigValue::Boolean(b) => Some(*b),
            _ => None,
        }
    }

    /// Get as a list, if it is one.
    pub fn as_list(&self) -> Option<&[ConfigValue]> {
        match self {
            ConfigValue::List(l) => Some(l),
            _ => None,
        }
    }

    /// Get as a map, if it is one.
    pub fn as_map(&self) -> Option<&HashMap<String, ConfigValue>> {
        match self {
            ConfigValue::Map(m) => Some(m),
            _ => None,
        }
    }

    /// Check if this is a null value.
    pub fn is_null(&self) -> bool {
        matches!(self, ConfigValue::Null)
    }

    /// Return the type name as a string.
    pub fn type_name(&self) -> &'static str {
        match self {
            ConfigValue::String(_) => "string",
            ConfigValue::Integer(_) => "integer",
            ConfigValue::Float(_) => "float",
            ConfigValue::Boolean(_) => "boolean",
            ConfigValue::List(_) => "list",
            ConfigValue::Map(_) => "map",
            ConfigValue::Null => "null",
        }
    }
}

impl fmt::Display for ConfigValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigValue::String(s) => write!(f, "\"{s}\""),
            ConfigValue::Integer(n) => write!(f, "{n}"),
            ConfigValue::Float(v) => write!(f, "{v}"),
            ConfigValue::Boolean(b) => write!(f, "{b}"),
            ConfigValue::List(l) => write!(f, "[{} items]", l.len()),
            ConfigValue::Map(m) => write!(f, "{{{} keys}}", m.len()),
            ConfigValue::Null => write!(f, "null"),
        }
    }
}

/// The expected type of a configuration field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConfigType {
    /// A string.
    String,
    /// An integer.
    Integer,
    /// A float.
    Float,
    /// A boolean.
    Boolean,
    /// A list.
    List,
    /// A map.
    Map,
}

impl ConfigType {
    /// Check if a value matches this type.
    pub fn matches(&self, value: &ConfigValue) -> bool {
        matches!(
            (self, value),
            (ConfigType::String, ConfigValue::String(_))
                | (ConfigType::Integer, ConfigValue::Integer(_))
                | (ConfigType::Float, ConfigValue::Float(_))
                | (ConfigType::Float, ConfigValue::Integer(_))
                | (ConfigType::Boolean, ConfigValue::Boolean(_))
                | (ConfigType::List, ConfigValue::List(_))
                | (ConfigType::Map, ConfigValue::Map(_))
        )
    }
}

/// A validation rule for a configuration field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ValidationRule {
    /// Value must not be empty (for strings and lists).
    NonEmpty,
    /// Integer must be within a range.
    IntRange { min: i64, max: i64 },
    /// Float must be within a range.
    FloatRange { min: f64, max: f64 },
    /// String must match a pattern.
    Pattern { regex: String },
    /// String must be one of the given options.
    OneOf { options: Vec<String> },
    /// String max length.
    MaxLength { max: usize },
}

impl ValidationRule {
    /// Validate a config value against this rule.
    pub fn validate(&self, value: &ConfigValue) -> Result<(), String> {
        match self {
            ValidationRule::NonEmpty => match value {
                ConfigValue::String(s) if s.is_empty() => {
                    Err("value must not be empty".to_string())
                }
                ConfigValue::List(l) if l.is_empty() => {
                    Err("list must not be empty".to_string())
                }
                _ => Ok(()),
            },
            ValidationRule::IntRange { min, max } => {
                if let Some(n) = value.as_i64() {
                    if n < *min || n > *max {
                        Err(format!("value {n} is outside range [{min}, {max}]"))
                    } else {
                        Ok(())
                    }
                } else {
                    Err("expected integer value".to_string())
                }
            }
            ValidationRule::FloatRange { min, max } => {
                if let Some(f) = value.as_f64() {
                    if f < *min || f > *max {
                        Err(format!("value {f} is outside range [{min}, {max}]"))
                    } else {
                        Ok(())
                    }
                } else {
                    Err("expected numeric value".to_string())
                }
            }
            ValidationRule::OneOf { options } => {
                if let Some(s) = value.as_str() {
                    if options.contains(&s.to_string()) {
                        Ok(())
                    } else {
                        Err(format!("value must be one of: {}", options.join(", ")))
                    }
                } else {
                    Err("expected string value".to_string())
                }
            }
            ValidationRule::MaxLength { max } => {
                if let Some(s) = value.as_str() {
                    if s.len() > *max {
                        Err(format!("string length {} exceeds max {max}", s.len()))
                    } else {
                        Ok(())
                    }
                } else {
                    Ok(())
                }
            }
            ValidationRule::Pattern { regex: _ } => {
                // Pattern validation would require a regex crate; skip for now.
                Ok(())
            }
        }
    }
}

/// Schema definition for a single configuration field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFieldSchema {
    /// The field key.
    pub key: String,
    /// Human-readable label.
    pub label: String,
    /// Description of what this field controls.
    pub description: String,
    /// The expected type.
    pub value_type: ConfigType,
    /// Default value.
    pub default: ConfigValue,
    /// Whether this field is required.
    pub required: bool,
    /// Validation rules.
    pub rules: Vec<ValidationRule>,
}

/// The complete configuration schema for a plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfigSchema {
    /// Schema version for migration support.
    pub version: u32,
    /// Configuration fields.
    pub fields: Vec<ConfigFieldSchema>,
}

impl PluginConfigSchema {
    /// Create a new schema.
    pub fn new(version: u32) -> Self {
        Self {
            version,
            fields: Vec::new(),
        }
    }

    /// Add a field to the schema.
    pub fn add_field(&mut self, field: ConfigFieldSchema) {
        self.fields.push(field);
    }

    /// Validate a configuration map against this schema.
    pub fn validate(&self, config: &HashMap<String, ConfigValue>) -> Vec<ValidationError> {
        let mut errors = Vec::new();

        for field in &self.fields {
            match config.get(&field.key) {
                Some(value) => {
                    if !field.value_type.matches(value) {
                        errors.push(ValidationError {
                            field: field.key.clone(),
                            message: format!(
                                "expected type {}, got {}",
                                format!("{:?}", field.value_type).to_lowercase(),
                                value.type_name()
                            ),
                        });
                        continue;
                    }
                    for rule in &field.rules {
                        if let Err(msg) = rule.validate(value) {
                            errors.push(ValidationError {
                                field: field.key.clone(),
                                message: msg,
                            });
                        }
                    }
                }
                None => {
                    if field.required {
                        errors.push(ValidationError {
                            field: field.key.clone(),
                            message: "required field is missing".to_string(),
                        });
                    }
                }
            }
        }

        errors
    }

    /// Generate a default configuration from the schema.
    pub fn defaults(&self) -> HashMap<String, ConfigValue> {
        self.fields
            .iter()
            .map(|f| (f.key.clone(), f.default.clone()))
            .collect()
    }

    /// Get a field schema by key.
    pub fn get_field(&self, key: &str) -> Option<&ConfigFieldSchema> {
        self.fields.iter().find(|f| f.key == key)
    }
}

/// A validation error for a specific config field.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationError {
    /// The field that failed validation.
    pub field: String,
    /// The error message.
    pub message: String,
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.field, self.message)
    }
}

/// A migration step between config schema versions.
#[derive(Debug, Clone)]
pub struct ConfigMigration {
    /// Source version.
    pub from_version: u32,
    /// Target version.
    pub to_version: u32,
    /// Description of the migration.
    pub description: String,
    /// Fields to add with default values.
    pub add_fields: HashMap<String, ConfigValue>,
    /// Fields to remove.
    pub remove_fields: Vec<String>,
    /// Fields to rename (old_key -> new_key).
    pub rename_fields: HashMap<String, String>,
}

impl ConfigMigration {
    /// Create a new migration.
    pub fn new(from: u32, to: u32) -> Self {
        Self {
            from_version: from,
            to_version: to,
            description: String::new(),
            add_fields: HashMap::new(),
            remove_fields: Vec::new(),
            rename_fields: HashMap::new(),
        }
    }

    /// Apply the migration to a configuration map.
    pub fn apply(&self, config: &mut HashMap<String, ConfigValue>) {
        // Remove fields.
        for key in &self.remove_fields {
            config.remove(key);
        }
        // Rename fields.
        for (old_key, new_key) in &self.rename_fields {
            if let Some(value) = config.remove(old_key) {
                config.insert(new_key.clone(), value);
            }
        }
        // Add new fields with defaults.
        for (key, value) in &self.add_fields {
            config.entry(key.clone()).or_insert_with(|| value.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_value_types() {
        let v = ConfigValue::String("hello".to_string());
        assert_eq!(v.as_str(), Some("hello"));
        assert_eq!(v.as_i64(), None);
        assert_eq!(v.type_name(), "string");
    }

    #[test]
    fn schema_validation() {
        let mut schema = PluginConfigSchema::new(1);
        schema.add_field(ConfigFieldSchema {
            key: "port".to_string(),
            label: "Port".to_string(),
            description: "Server port".to_string(),
            value_type: ConfigType::Integer,
            default: ConfigValue::Integer(8080),
            required: true,
            rules: vec![ValidationRule::IntRange { min: 1, max: 65535 }],
        });

        let mut config = HashMap::new();
        config.insert("port".to_string(), ConfigValue::Integer(3000));
        let errors = schema.validate(&config);
        assert!(errors.is_empty());

        config.insert("port".to_string(), ConfigValue::Integer(0));
        let errors = schema.validate(&config);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn migration() {
        let mut migration = ConfigMigration::new(1, 2);
        migration.add_fields.insert("new_field".to_string(), ConfigValue::Boolean(true));
        migration.remove_fields.push("old_field".to_string());

        let mut config = HashMap::new();
        config.insert("old_field".to_string(), ConfigValue::String("x".to_string()));
        config.insert("keep".to_string(), ConfigValue::Integer(42));

        migration.apply(&mut config);
        assert!(!config.contains_key("old_field"));
        assert!(config.contains_key("new_field"));
        assert!(config.contains_key("keep"));
    }
}
