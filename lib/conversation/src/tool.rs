//! Tool registry for conversation mode.
//!
//! Tools are operations available during conversation, including:
//! - Integration operations (email, calendar, etc.)
//! - Workflow invocation
//! - Search and retrieval

use crate::error::ToolError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;

/// Definition of a tool available during conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool name.
    pub name: String,
    /// Human-readable description.
    pub description: String,
    /// JSON schema for input parameters.
    pub input_schema: JsonValue,
    /// Whether this tool requires confirmation before execution.
    pub requires_confirmation: bool,
    /// Tool category.
    pub category: ToolCategory,
}

/// Categories of tools.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolCategory {
    /// Integration operations (email, calendar).
    Integration,
    /// Workflow invocation.
    Workflow,
    /// Search and retrieval.
    Search,
    /// System operations.
    System,
    /// Custom user-defined tools.
    Custom,
}

impl ToolDefinition {
    /// Creates a new tool definition.
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({}),
            requires_confirmation: false,
            category: ToolCategory::Custom,
        }
    }

    /// Sets the input schema.
    #[must_use]
    pub fn with_input_schema(mut self, schema: JsonValue) -> Self {
        self.input_schema = schema;
        self
    }

    /// Marks this tool as requiring confirmation.
    #[must_use]
    pub fn requires_confirmation(mut self) -> Self {
        self.requires_confirmation = true;
        self
    }

    /// Sets the category.
    #[must_use]
    pub fn with_category(mut self, category: ToolCategory) -> Self {
        self.category = category;
        self
    }
}

/// Result of a tool invocation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Whether the invocation succeeded.
    pub success: bool,
    /// Result data (if successful).
    pub data: Option<JsonValue>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Execution metadata.
    pub metadata: ToolResultMetadata,
}

/// Metadata about tool execution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolResultMetadata {
    /// Execution time in milliseconds.
    pub execution_time_ms: u64,
    /// Whether the tool modified external state.
    pub has_side_effects: bool,
}

impl ToolResult {
    /// Creates a successful result.
    #[must_use]
    pub fn success(data: JsonValue) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            metadata: ToolResultMetadata::default(),
        }
    }

    /// Creates a failed result.
    #[must_use]
    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error.into()),
            metadata: ToolResultMetadata::default(),
        }
    }

    /// Adds metadata.
    #[must_use]
    pub fn with_metadata(mut self, metadata: ToolResultMetadata) -> Self {
        self.metadata = metadata;
        self
    }
}

/// Trait for tool execution.
pub trait Tool: Send + Sync {
    /// Returns the tool definition.
    fn definition(&self) -> ToolDefinition;

    /// Executes the tool with the given input.
    fn execute(
        &self,
        input: JsonValue,
    ) -> impl std::future::Future<Output = Result<ToolResult, ToolError>> + Send;
}

/// Registry of available tools.
#[derive(Debug, Default)]
pub struct ToolRegistry {
    definitions: HashMap<String, ToolDefinition>,
}

impl ToolRegistry {
    /// Creates a new empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            definitions: HashMap::new(),
        }
    }

    /// Registers a tool definition.
    pub fn register(&mut self, definition: ToolDefinition) {
        self.definitions.insert(definition.name.clone(), definition);
    }

    /// Gets a tool definition by name.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.definitions.get(name)
    }

    /// Returns all registered tool definitions.
    pub fn all(&self) -> impl Iterator<Item = &ToolDefinition> {
        self.definitions.values()
    }

    /// Returns tool definitions by category.
    pub fn by_category(&self, category: ToolCategory) -> impl Iterator<Item = &ToolDefinition> {
        self.definitions.values().filter(move |d| d.category == category)
    }

    /// Returns the number of registered tools.
    #[must_use]
    pub fn len(&self) -> usize {
        self.definitions.len()
    }

    /// Returns whether the registry is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.definitions.is_empty()
    }

    /// Converts definitions to the format expected by LLM APIs.
    #[must_use]
    pub fn to_llm_format(&self) -> Vec<JsonValue> {
        self.definitions
            .values()
            .map(|def| {
                serde_json::json!({
                    "name": def.name,
                    "description": def.description,
                    "parameters": def.input_schema
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definition_builder() {
        let tool = ToolDefinition::new("search_emails", "Search through emails")
            .with_input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string" }
                }
            }))
            .with_category(ToolCategory::Integration)
            .requires_confirmation();

        assert_eq!(tool.name, "search_emails");
        assert!(tool.requires_confirmation);
        assert_eq!(tool.category, ToolCategory::Integration);
    }

    #[test]
    fn tool_result_success() {
        let result = ToolResult::success(serde_json::json!({"emails": []}));
        assert!(result.success);
        assert!(result.data.is_some());
    }

    #[test]
    fn tool_result_failure() {
        let result = ToolResult::failure("Connection failed");
        assert!(!result.success);
        assert_eq!(result.error, Some("Connection failed".to_string()));
    }

    #[test]
    fn tool_registry_operations() {
        let mut registry = ToolRegistry::new();

        registry.register(
            ToolDefinition::new("tool1", "First tool")
                .with_category(ToolCategory::Integration)
        );
        registry.register(
            ToolDefinition::new("tool2", "Second tool")
                .with_category(ToolCategory::Search)
        );

        assert_eq!(registry.len(), 2);
        assert!(registry.get("tool1").is_some());
        assert!(registry.get("nonexistent").is_none());

        let integration_tools: Vec<_> = registry.by_category(ToolCategory::Integration).collect();
        assert_eq!(integration_tools.len(), 1);
    }

    #[test]
    fn tool_registry_llm_format() {
        let mut registry = ToolRegistry::new();
        registry.register(
            ToolDefinition::new("calculate", "Do math")
                .with_input_schema(serde_json::json!({
                    "type": "object",
                    "properties": {
                        "expression": { "type": "string" }
                    }
                }))
        );

        let llm_format = registry.to_llm_format();
        assert_eq!(llm_format.len(), 1);
        assert_eq!(llm_format[0]["name"], "calculate");
    }
}
