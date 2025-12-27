//! Coordinate primitive.
//!
//! The Coordinate primitive is an LLM-driven execution loop where the model:
//! 1. Evaluates context and goal
//! 2. Decides what operations to run
//! 3. Executes operations
//! 4. Evaluates results
//! 5. Decides: done, or more operations needed?
//! 6. Repeats until done or max iterations reached

use crate::llm_call::LlmInvocationId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ulid::Ulid;

/// Unique identifier for a coordination session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CoordinateSessionId(Ulid);

impl CoordinateSessionId {
    /// Creates a new session ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }
}

impl Default for CoordinateSessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for CoordinateSessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "coord_{}", self.0)
    }
}

/// Configuration for the Coordinate primitive.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateConfig {
    /// The goal to achieve.
    pub goal: String,
    /// Maximum iterations before failing.
    pub max_iterations: u32,
    /// Available tools/operations.
    pub available_tools: Vec<ToolDefinition>,
    /// Optional system prompt for the coordinator.
    pub system_prompt: Option<String>,
}

impl CoordinateConfig {
    /// Creates a new coordinate configuration.
    #[must_use]
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            goal: goal.into(),
            max_iterations: 10,
            available_tools: Vec::new(),
            system_prompt: None,
        }
    }

    /// Sets the maximum iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.max_iterations = max;
        self
    }

    /// Adds an available tool.
    #[must_use]
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.available_tools.push(tool);
        self
    }

    /// Sets the system prompt.
    #[must_use]
    pub fn with_system_prompt(mut self, prompt: impl Into<String>) -> Self {
        self.system_prompt = Some(prompt.into());
        self
    }
}

/// Definition of a tool available to the coordinator.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique name of the tool.
    pub name: String,
    /// Description of what the tool does.
    pub description: String,
    /// JSON schema for the tool's input parameters.
    pub input_schema: JsonValue,
    /// JSON schema for the tool's output.
    pub output_schema: Option<JsonValue>,
}

impl ToolDefinition {
    /// Creates a new tool definition.
    #[must_use]
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            input_schema: serde_json::json!({}),
            output_schema: None,
        }
    }

    /// Sets the input schema.
    #[must_use]
    pub fn with_input_schema(mut self, schema: JsonValue) -> Self {
        self.input_schema = schema;
        self
    }

    /// Sets the output schema.
    #[must_use]
    pub fn with_output_schema(mut self, schema: JsonValue) -> Self {
        self.output_schema = Some(schema);
        self
    }
}

/// A single step in a coordination session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateStep {
    /// Step number (1-indexed).
    pub step_number: u32,
    /// The LLM invocation that decided this step.
    pub decision_invocation: LlmInvocationId,
    /// Actions taken in this step.
    pub actions: Vec<ActionExecution>,
    /// The LLM's reasoning for this step.
    pub reasoning: String,
    /// Whether the coordinator decided to continue.
    pub should_continue: bool,
    /// Timestamp of this step.
    pub timestamp: DateTime<Utc>,
}

/// Record of a single action execution within a step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionExecution {
    /// The tool that was invoked.
    pub tool_name: String,
    /// Input provided to the tool.
    pub input: JsonValue,
    /// Output from the tool (if successful).
    pub output: Option<JsonValue>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Latency in milliseconds.
    pub latency_ms: u64,
}

impl ActionExecution {
    /// Creates a successful action execution.
    #[must_use]
    pub fn success(tool_name: impl Into<String>, input: JsonValue, output: JsonValue, latency_ms: u64) -> Self {
        Self {
            tool_name: tool_name.into(),
            input,
            output: Some(output),
            error: None,
            latency_ms,
        }
    }

    /// Creates a failed action execution.
    #[must_use]
    pub fn failure(tool_name: impl Into<String>, input: JsonValue, error: impl Into<String>, latency_ms: u64) -> Self {
        Self {
            tool_name: tool_name.into(),
            input,
            output: None,
            error: Some(error.into()),
            latency_ms,
        }
    }

    /// Returns whether this action succeeded.
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.error.is_none()
    }
}

/// The result of a coordination session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoordinateResult {
    /// Session identifier.
    pub session_id: CoordinateSessionId,
    /// The goal that was pursued.
    pub goal: String,
    /// Whether the goal was achieved.
    pub success: bool,
    /// Final result (if successful).
    pub result: Option<JsonValue>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// All steps taken during coordination.
    pub steps: Vec<CoordinateStep>,
    /// Total number of iterations.
    pub iteration_count: u32,
    /// When the session started.
    pub started_at: DateTime<Utc>,
    /// When the session ended.
    pub finished_at: DateTime<Utc>,
}

impl CoordinateResult {
    /// Creates a successful result.
    #[must_use]
    pub fn success(
        session_id: CoordinateSessionId,
        goal: String,
        result: JsonValue,
        steps: Vec<CoordinateStep>,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            goal,
            success: true,
            result: Some(result),
            error: None,
            iteration_count: steps.len() as u32,
            steps,
            started_at,
            finished_at: Utc::now(),
        }
    }

    /// Creates a failed result.
    #[must_use]
    pub fn failure(
        session_id: CoordinateSessionId,
        goal: String,
        error: String,
        steps: Vec<CoordinateStep>,
        started_at: DateTime<Utc>,
    ) -> Self {
        Self {
            session_id,
            goal,
            success: false,
            result: None,
            error: Some(error),
            iteration_count: steps.len() as u32,
            steps,
            started_at,
            finished_at: Utc::now(),
        }
    }

    /// Returns the total duration of the session.
    #[must_use]
    pub fn duration(&self) -> chrono::Duration {
        self.finished_at - self.started_at
    }
}

/// A Coordinator executes the coordinate primitive.
///
/// This is a builder for setting up and running coordination sessions.
#[derive(Debug, Clone)]
pub struct Coordinator {
    config: CoordinateConfig,
    initial_context: Option<JsonValue>,
}

impl Coordinator {
    /// Creates a new Coordinator with the given goal.
    #[must_use]
    pub fn new(goal: impl Into<String>) -> Self {
        Self {
            config: CoordinateConfig::new(goal),
            initial_context: None,
        }
    }

    /// Creates a Coordinator from a configuration.
    #[must_use]
    pub fn from_config(config: CoordinateConfig) -> Self {
        Self {
            config,
            initial_context: None,
        }
    }

    /// Sets the maximum iterations.
    #[must_use]
    pub fn with_max_iterations(mut self, max: u32) -> Self {
        self.config.max_iterations = max;
        self
    }

    /// Adds an available tool.
    #[must_use]
    pub fn with_tool(mut self, tool: ToolDefinition) -> Self {
        self.config.available_tools.push(tool);
        self
    }

    /// Sets the initial context.
    #[must_use]
    pub fn with_context(mut self, context: JsonValue) -> Self {
        self.initial_context = Some(context);
        self
    }

    /// Returns the configuration.
    #[must_use]
    pub fn config(&self) -> &CoordinateConfig {
        &self.config
    }

    /// Returns the initial context.
    #[must_use]
    pub fn initial_context(&self) -> Option<&JsonValue> {
        self.initial_context.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinate_config_builder() {
        let config = CoordinateConfig::new("Plan a trip to Japan")
            .with_max_iterations(15)
            .with_tool(ToolDefinition::new("search_flights", "Search for flights"))
            .with_tool(ToolDefinition::new("search_hotels", "Search for hotels"));

        assert_eq!(config.goal, "Plan a trip to Japan");
        assert_eq!(config.max_iterations, 15);
        assert_eq!(config.available_tools.len(), 2);
    }

    #[test]
    fn tool_definition_builder() {
        let tool = ToolDefinition::new("calculate", "Perform calculations")
            .with_input_schema(serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": { "type": "string" }
                }
            }))
            .with_output_schema(serde_json::json!({
                "type": "number"
            }));

        assert_eq!(tool.name, "calculate");
        assert!(tool.output_schema.is_some());
    }

    #[test]
    fn action_execution_success() {
        let action = ActionExecution::success(
            "search",
            serde_json::json!({"query": "flights"}),
            serde_json::json!({"results": []}),
            150,
        );

        assert!(action.is_success());
        assert!(action.output.is_some());
        assert!(action.error.is_none());
    }

    #[test]
    fn action_execution_failure() {
        let action = ActionExecution::failure(
            "api_call",
            serde_json::json!({}),
            "Connection timeout",
            5000,
        );

        assert!(!action.is_success());
        assert!(action.output.is_none());
        assert!(action.error.is_some());
    }

    #[test]
    fn coordinate_result_serde() {
        let session_id = CoordinateSessionId::new();
        let started = Utc::now();
        let result = CoordinateResult::success(
            session_id,
            "Test goal".to_string(),
            serde_json::json!({"answer": 42}),
            vec![],
            started,
        );

        let json = serde_json::to_string(&result).expect("serialize");
        let parsed: CoordinateResult = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(result.session_id, parsed.session_id);
        assert!(parsed.success);
    }
}
