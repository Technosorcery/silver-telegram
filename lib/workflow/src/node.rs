//! Workflow node types and configurations.
//!
//! Nodes are the building blocks of workflows. Each node has:
//! - A unique ID within the workflow
//! - A category (Trigger, AI Layer, Integration, etc.)
//! - Configuration specific to its type
//! - Input and output ports

use crate::port::{InputPort, OutputPort, PortSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use ulid::Ulid;

/// A unique identifier for a node within a workflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct NodeId(Ulid);

impl NodeId {
    /// Creates a new random node ID.
    #[must_use]
    pub fn new() -> Self {
        Self(Ulid::new())
    }

    /// Creates a node ID from a ULID.
    #[must_use]
    pub const fn from_ulid(ulid: Ulid) -> Self {
        Self(ulid)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for NodeId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "node_{}", self.0)
    }
}

/// The category of a workflow node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeCategory {
    /// Entry points that initiate workflow execution.
    Trigger,
    /// AI-powered operations (LLM Call, Coordinate).
    AiLayer,
    /// Protocol-specific actions (email, calendar, etc.).
    Integration,
    /// Expression-based data manipulation.
    Transform,
    /// Graph structure control (Branch, Loop, Parallel, Join).
    ControlFlow,
    /// Cross-run state management.
    Memory,
    /// Terminal actions (Notify, Log, HTTP Response).
    Output,
}

/// Configuration for trigger nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TriggerNodeConfig {
    /// Cron-style scheduled trigger.
    Schedule {
        /// Cron expression (e.g., "0 7 * * *" for 7am daily).
        cron: String,
        /// Timezone for the schedule.
        timezone: Option<String>,
    },
    /// HTTP webhook trigger.
    Webhook {
        /// The webhook path (e.g., "/hooks/my-workflow").
        path: String,
    },
    /// Integration event trigger.
    IntegrationEvent {
        /// The integration account ID.
        integration_id: String,
        /// The event type to listen for.
        event_type: String,
    },
    /// Manual trigger (user-initiated).
    Manual,
}

/// Configuration for AI layer nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AiLayerNodeConfig {
    /// Single-shot LLM inference.
    LlmCall {
        /// The prompt template name or inline prompt.
        prompt: String,
        /// Optional output schema for structured output.
        output_schema: Option<PortSchema>,
        /// Constraint level for the output.
        constraint_level: ConstraintLevel,
    },
    /// LLM-driven execution loop.
    Coordinate {
        /// The goal to achieve.
        goal: String,
        /// Maximum iterations before failing.
        max_iterations: u32,
        /// Available tools/operations for the coordinator.
        available_tools: Vec<String>,
    },
    /// Classify content into categories.
    Classify {
        /// The categories to choose from.
        categories: Vec<String>,
        /// Constraint level for the output.
        constraint_level: ConstraintLevel,
    },
    /// Extract structured data from content.
    Extract {
        /// The output schema for extracted data.
        output_schema: PortSchema,
    },
    /// Generate text based on context.
    Generate {
        /// Instructions for generation.
        instructions: String,
    },
    /// Summarize content.
    Summarize {
        /// Maximum length constraint.
        max_length: Option<u32>,
    },
    /// Score content against criteria.
    Score {
        /// Scoring criteria.
        criteria: String,
        /// Minimum score (default 0.0).
        min_score: f64,
        /// Maximum score (default 1.0).
        max_score: f64,
    },
    /// Check if item is a duplicate of recent items.
    Deduplicate {
        /// How to compare items.
        comparison_method: String,
    },
    /// Decide between options.
    Decide {
        /// The options to choose from.
        options: Vec<String>,
        /// Criteria for decision.
        criteria: String,
    },
}

/// Constraint level for AI primitive outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConstraintLevel {
    /// Pick from exactly the specified options.
    #[default]
    Constrained,
    /// Pick from options or suggest a new one.
    SemiConstrained,
    /// Determine appropriate output freely.
    Unconstrained,
}

/// Configuration for integration nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct IntegrationNodeConfig {
    /// The integration type (e.g., "email", "calendar").
    pub integration_type: String,
    /// The operation to perform (e.g., "fetch", "send", "list").
    pub operation: String,
    /// Operation-specific parameters.
    pub parameters: JsonValue,
}

/// Configuration for transform nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct TransformNodeConfig {
    /// Expression for data transformation.
    /// Note: Expression language is deferred per ADR-005.
    pub expression: String,
}

/// Configuration for control flow nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ControlFlowNodeConfig {
    /// Conditional branching.
    Branch {
        /// Conditions for each branch (port name -> condition expression).
        conditions: Vec<BranchCondition>,
    },
    /// Fan-out: explode array into individual items.
    FanOut,
    /// Fan-in: collect items back into array.
    FanIn {
        /// The corresponding FanOut node ID.
        fan_out_node: NodeId,
    },
    /// Parallel execution marker (multiple outgoing edges).
    Parallel,
    /// Join parallel branches.
    Join,
}

/// A condition for a branch.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BranchCondition {
    /// The output port name for this branch.
    pub port: String,
    /// The condition expression.
    pub condition: String,
}

/// Configuration for memory nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum MemoryNodeConfig {
    /// Load workflow memory.
    LoadMemory,
    /// Record (update) workflow memory.
    RecordMemory {
        /// Instructions for how AI should maintain memory.
        update_instructions: String,
    },
}

/// Configuration for output nodes.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OutputNodeConfig {
    /// Send a notification.
    Notify {
        /// Notification channel (e.g., "email", "push").
        channel: String,
        /// Template for the notification.
        template: String,
    },
    /// Log to execution history.
    Log {
        /// Log level.
        level: LogLevel,
    },
    /// HTTP response (for webhook-triggered workflows).
    HttpResponse {
        /// Status code to return.
        status_code: u16,
    },
}

/// Log level for log nodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

/// Configuration for a node, varying by category.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum NodeConfig {
    /// Trigger node configuration.
    Trigger(TriggerNodeConfig),
    /// AI layer node configuration.
    AiLayer(AiLayerNodeConfig),
    /// Integration node configuration.
    Integration(IntegrationNodeConfig),
    /// Transform node configuration.
    Transform(TransformNodeConfig),
    /// Control flow node configuration.
    ControlFlow(ControlFlowNodeConfig),
    /// Memory node configuration.
    Memory(MemoryNodeConfig),
    /// Output node configuration.
    Output(OutputNodeConfig),
}

impl NodeConfig {
    /// Returns the category of this node configuration.
    #[must_use]
    pub fn category(&self) -> NodeCategory {
        match self {
            Self::Trigger(_) => NodeCategory::Trigger,
            Self::AiLayer(_) => NodeCategory::AiLayer,
            Self::Integration(_) => NodeCategory::Integration,
            Self::Transform(_) => NodeCategory::Transform,
            Self::ControlFlow(_) => NodeCategory::ControlFlow,
            Self::Memory(_) => NodeCategory::Memory,
            Self::Output(_) => NodeCategory::Output,
        }
    }
}

/// A workflow node.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    /// Unique identifier for this node within the workflow.
    pub id: NodeId,
    /// Human-readable name for this node.
    pub name: String,
    /// Node configuration (determines type and behavior).
    pub config: NodeConfig,
    /// Input ports for this node.
    pub inputs: Vec<InputPort>,
    /// Output ports for this node.
    pub outputs: Vec<OutputPort>,
}

impl Node {
    /// Creates a new node with the given configuration.
    #[must_use]
    pub fn new(name: impl Into<String>, config: NodeConfig) -> Self {
        let (inputs, outputs) = Self::default_ports(&config);
        Self {
            id: NodeId::new(),
            name: name.into(),
            config,
            inputs,
            outputs,
        }
    }

    /// Creates a new node with a specific ID.
    #[must_use]
    pub fn with_id(id: NodeId, name: impl Into<String>, config: NodeConfig) -> Self {
        let (inputs, outputs) = Self::default_ports(&config);
        Self {
            id,
            name: name.into(),
            config,
            inputs,
            outputs,
        }
    }

    /// Returns the category of this node.
    #[must_use]
    pub fn category(&self) -> NodeCategory {
        self.config.category()
    }

    /// Returns the input port with the given name, if any.
    #[must_use]
    pub fn input_port(&self, name: &str) -> Option<&InputPort> {
        self.inputs.iter().find(|p| p.name == name)
    }

    /// Returns the output port with the given name, if any.
    #[must_use]
    pub fn output_port(&self, name: &str) -> Option<&OutputPort> {
        self.outputs.iter().find(|p| p.name == name)
    }

    /// Generates default ports based on node configuration.
    fn default_ports(config: &NodeConfig) -> (Vec<InputPort>, Vec<OutputPort>) {
        match config {
            NodeConfig::Trigger(_) => {
                // Triggers have no inputs, one output
                (vec![], vec![OutputPort::new("output", PortSchema::any())])
            }
            NodeConfig::AiLayer(ai_config) => match ai_config {
                AiLayerNodeConfig::LlmCall { output_schema, .. } => (
                    vec![InputPort::required("input", PortSchema::any())],
                    vec![OutputPort::new(
                        "output",
                        output_schema.clone().unwrap_or_else(PortSchema::any),
                    )],
                ),
                AiLayerNodeConfig::Coordinate { .. } => (
                    vec![InputPort::required("context", PortSchema::any())],
                    vec![OutputPort::new("result", PortSchema::any())],
                ),
                AiLayerNodeConfig::Classify { .. } => (
                    vec![InputPort::required("content", PortSchema::any())],
                    vec![OutputPort::new(
                        "classification",
                        PortSchema::from_json(serde_json::json!({
                            "type": "object",
                            "properties": {
                                "category": { "type": "string" },
                                "confidence": { "type": "number" }
                            }
                        })),
                    )],
                ),
                AiLayerNodeConfig::Extract { output_schema } => (
                    vec![InputPort::required("content", PortSchema::any())],
                    vec![OutputPort::new("extracted", output_schema.clone())],
                ),
                AiLayerNodeConfig::Generate { .. } => (
                    vec![InputPort::required("context", PortSchema::any())],
                    vec![OutputPort::new("generated", PortSchema::string())],
                ),
                AiLayerNodeConfig::Summarize { .. } => (
                    vec![InputPort::required("content", PortSchema::any())],
                    vec![OutputPort::new("summary", PortSchema::string())],
                ),
                AiLayerNodeConfig::Score { .. } => (
                    vec![InputPort::required("content", PortSchema::any())],
                    vec![OutputPort::new("score", PortSchema::number())],
                ),
                AiLayerNodeConfig::Deduplicate { .. } => (
                    vec![
                        InputPort::required("item", PortSchema::any()),
                        InputPort::required("recent_items", PortSchema::array()),
                    ],
                    vec![OutputPort::new("is_duplicate", PortSchema::boolean())],
                ),
                AiLayerNodeConfig::Decide { .. } => (
                    vec![InputPort::required("context", PortSchema::any())],
                    vec![OutputPort::new("decision", PortSchema::string())],
                ),
            },
            NodeConfig::Integration(_) => (
                vec![InputPort::optional("input", PortSchema::any())],
                vec![OutputPort::new("output", PortSchema::any())],
            ),
            NodeConfig::Transform(_) => (
                vec![InputPort::required("input", PortSchema::any())],
                vec![OutputPort::new("output", PortSchema::any())],
            ),
            NodeConfig::ControlFlow(cf_config) => match cf_config {
                ControlFlowNodeConfig::Branch { conditions } => {
                    let outputs = conditions
                        .iter()
                        .map(|c| OutputPort::new(&c.port, PortSchema::any()))
                        .collect();
                    (
                        vec![InputPort::required("input", PortSchema::any())],
                        outputs,
                    )
                }
                ControlFlowNodeConfig::FanOut => (
                    vec![InputPort::required("items", PortSchema::array())],
                    vec![OutputPort::new("item", PortSchema::any())],
                ),
                ControlFlowNodeConfig::FanIn { .. } => (
                    vec![InputPort::required("item", PortSchema::any())],
                    vec![OutputPort::new("items", PortSchema::array())],
                ),
                ControlFlowNodeConfig::Parallel | ControlFlowNodeConfig::Join => (
                    vec![InputPort::required("input", PortSchema::any())],
                    vec![OutputPort::new("output", PortSchema::any())],
                ),
            },
            NodeConfig::Memory(mem_config) => match mem_config {
                MemoryNodeConfig::LoadMemory => {
                    (vec![], vec![OutputPort::new("memory", PortSchema::any())])
                }
                MemoryNodeConfig::RecordMemory { .. } => (
                    vec![InputPort::required("workflow_output", PortSchema::any())],
                    vec![OutputPort::new("memory", PortSchema::any())],
                ),
            },
            NodeConfig::Output(_) => (
                vec![InputPort::required("input", PortSchema::any())],
                vec![],
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_id_display() {
        let id = NodeId::new();
        let display = id.to_string();
        assert!(display.starts_with("node_"));
    }

    #[test]
    fn trigger_node_has_no_inputs() {
        let node = Node::new(
            "Daily Schedule",
            NodeConfig::Trigger(TriggerNodeConfig::Schedule {
                cron: "0 7 * * *".to_string(),
                timezone: None,
            }),
        );
        assert!(node.inputs.is_empty());
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.outputs[0].name, "output");
    }

    #[test]
    fn classify_node_has_classification_output() {
        let node = Node::new(
            "Email Classifier",
            NodeConfig::AiLayer(AiLayerNodeConfig::Classify {
                categories: vec!["spam".to_string(), "important".to_string()],
                constraint_level: ConstraintLevel::Constrained,
            }),
        );
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 1);
        assert_eq!(node.outputs[0].name, "classification");
    }

    #[test]
    fn branch_node_has_multiple_outputs() {
        let node = Node::new(
            "Router",
            NodeConfig::ControlFlow(ControlFlowNodeConfig::Branch {
                conditions: vec![
                    BranchCondition {
                        port: "high".to_string(),
                        condition: "confidence > 0.8".to_string(),
                    },
                    BranchCondition {
                        port: "low".to_string(),
                        condition: "confidence <= 0.8".to_string(),
                    },
                ],
            }),
        );
        assert_eq!(node.inputs.len(), 1);
        assert_eq!(node.outputs.len(), 2);
        assert_eq!(node.outputs[0].name, "high");
        assert_eq!(node.outputs[1].name, "low");
    }

    #[test]
    fn node_serde_roundtrip() {
        let node = Node::new(
            "Test",
            NodeConfig::AiLayer(AiLayerNodeConfig::Generate {
                instructions: "Write a greeting".to_string(),
            }),
        );
        let json = serde_json::to_string(&node).expect("serialize");
        let parsed: Node = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(node.name, parsed.name);
    }
}
