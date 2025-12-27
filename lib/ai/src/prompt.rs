//! Prompt template registry.
//!
//! Manages versioned prompt templates for AI operations.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use ulid::Ulid;

/// Unique identifier for a prompt template.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PromptTemplateId(Ulid);

impl PromptTemplateId {
    /// Creates a new template ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for PromptTemplateId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for PromptTemplateId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "prompt_{}", self.0)
    }
}

/// A versioned prompt template.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptTemplate {
    /// Unique identifier.
    pub id: PromptTemplateId,
    /// Template name (used for lookup).
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Template content with placeholders.
    pub content: String,
    /// Optional system prompt template.
    pub system_prompt: Option<String>,
    /// Description of what this template is for.
    pub description: Option<String>,
    /// Variable definitions (name -> description).
    pub variables: HashMap<String, VariableDefinition>,
    /// When this template was created.
    pub created_at: DateTime<Utc>,
    /// When this template was last updated.
    pub updated_at: DateTime<Utc>,
}

/// Definition of a template variable.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VariableDefinition {
    /// Description of what this variable is for.
    pub description: String,
    /// Whether this variable is required.
    pub required: bool,
    /// Default value if not provided.
    pub default: Option<JsonValue>,
    /// JSON schema for validation.
    pub schema: Option<JsonValue>,
}

impl VariableDefinition {
    /// Creates a required variable definition.
    #[must_use]
    pub fn required(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            required: true,
            default: None,
            schema: None,
        }
    }

    /// Creates an optional variable definition.
    #[must_use]
    pub fn optional(description: impl Into<String>) -> Self {
        Self {
            description: description.into(),
            required: false,
            default: None,
            schema: None,
        }
    }

    /// Sets a default value.
    #[must_use]
    pub fn with_default(mut self, default: JsonValue) -> Self {
        self.default = Some(default);
        self
    }
}

impl PromptTemplate {
    /// Creates a new prompt template.
    #[must_use]
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            id: PromptTemplateId::new(),
            name: name.into(),
            version: "0.1.0".to_string(),
            content: content.into(),
            system_prompt: None,
            description: None,
            variables: HashMap::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Sets the system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, system: impl Into<String>) -> Self {
        self.system_prompt = Some(system.into());
        self
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Adds a variable definition.
    #[must_use]
    pub fn with_variable(
        mut self,
        name: impl Into<String>,
        definition: VariableDefinition,
    ) -> Self {
        self.variables.insert(name.into(), definition);
        self
    }

    /// Renders the template with the given variables.
    ///
    /// Variables are substituted using `{{variable_name}}` syntax.
    #[must_use]
    pub fn render(&self, variables: &HashMap<String, JsonValue>) -> String {
        let mut result = self.content.clone();

        for (name, value) in variables {
            let placeholder = format!("{{{{{}}}}}", name);
            let replacement = match value {
                JsonValue::String(s) => s.clone(),
                other => other.to_string(),
            };
            result = result.replace(&placeholder, &replacement);
        }

        // Apply defaults for missing variables
        for (name, def) in &self.variables {
            let placeholder = format!("{{{{{}}}}}", name);
            if let Some(default) = &def.default
                && result.contains(&placeholder)
            {
                let replacement = match default {
                    JsonValue::String(s) => s.clone(),
                    other => other.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        result
    }

    /// Renders the system prompt with the given variables.
    #[must_use]
    pub fn render_system_prompt(&self, variables: &HashMap<String, JsonValue>) -> Option<String> {
        self.system_prompt.as_ref().map(|template| {
            let mut result = template.clone();
            for (name, value) in variables {
                let placeholder = format!("{{{{{}}}}}", name);
                let replacement = match value {
                    JsonValue::String(s) => s.clone(),
                    other => other.to_string(),
                };
                result = result.replace(&placeholder, &replacement);
            }
            result
        })
    }

    /// Validates that all required variables are provided.
    pub fn validate_variables(
        &self,
        variables: &HashMap<String, JsonValue>,
    ) -> Result<(), Vec<String>> {
        let missing: Vec<String> = self
            .variables
            .iter()
            .filter(|(_, def)| def.required && def.default.is_none())
            .filter(|(name, _)| !variables.contains_key(*name))
            .map(|(name, _)| name.clone())
            .collect();

        if missing.is_empty() {
            Ok(())
        } else {
            Err(missing)
        }
    }
}

/// Registry of prompt templates.
#[derive(Debug, Clone, Default)]
pub struct PromptRegistry {
    templates: HashMap<String, PromptTemplate>,
}

impl PromptRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            templates: HashMap::new(),
        }
    }

    /// Registers a template.
    pub fn register(&mut self, template: PromptTemplate) {
        self.templates.insert(template.name.clone(), template);
    }

    /// Gets a template by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&PromptTemplate> {
        self.templates.get(name)
    }

    /// Returns all registered templates.
    pub fn all(&self) -> impl Iterator<Item = &PromptTemplate> {
        self.templates.values()
    }

    /// Returns the number of registered templates.
    #[must_use]
    pub fn len(&self) -> usize {
        self.templates.len()
    }

    /// Returns whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.templates.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_rendering() {
        let template = PromptTemplate::new(
            "classify_email",
            "Classify the following email into one of: {{categories}}\n\nEmail: {{email}}",
        );

        let mut vars = HashMap::new();
        vars.insert(
            "categories".to_string(),
            serde_json::json!("spam, important, other"),
        );
        vars.insert(
            "email".to_string(),
            serde_json::json!("Hello, this is a test email."),
        );

        let rendered = template.render(&vars);
        assert!(rendered.contains("spam, important, other"));
        assert!(rendered.contains("Hello, this is a test email."));
    }

    #[test]
    fn template_with_defaults() {
        let template = PromptTemplate::new("greeting", "Hello, {{name}}! Your role is {{role}}.")
            .with_variable("name", VariableDefinition::required("User's name"))
            .with_variable(
                "role",
                VariableDefinition::optional("User's role")
                    .with_default(serde_json::json!("guest")),
            );

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), serde_json::json!("Alice"));

        let rendered = template.render(&vars);
        assert_eq!(rendered, "Hello, Alice! Your role is guest.");
    }

    #[test]
    fn template_validation() {
        let template = PromptTemplate::new("test", "{{required_var}} {{optional_var}}")
            .with_variable("required_var", VariableDefinition::required("Required"))
            .with_variable("optional_var", VariableDefinition::optional("Optional"));

        let empty_vars = HashMap::new();
        let result = template.validate_variables(&empty_vars);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), vec!["required_var"]);

        let mut valid_vars = HashMap::new();
        valid_vars.insert("required_var".to_string(), serde_json::json!("value"));
        let result = template.validate_variables(&valid_vars);
        assert!(result.is_ok());
    }

    #[test]
    fn registry_operations() {
        let mut registry = PromptRegistry::new();
        assert!(registry.is_empty());

        registry.register(PromptTemplate::new("template1", "Content 1"));
        registry.register(PromptTemplate::new("template2", "Content 2"));

        assert_eq!(registry.len(), 2);
        assert!(registry.get("template1").is_some());
        assert!(registry.get("nonexistent").is_none());
    }
}
