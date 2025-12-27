//! Workflow definition types.
//!
//! A workflow is a named, versioned automation that consists of:
//! - Metadata (name, description, version, timestamps)
//! - A directed graph of nodes
//! - Memory configuration (optional)

use crate::graph::WorkflowGraph;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::WorkflowId;

/// Metadata for a workflow definition.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMetadata {
    /// Human-readable name for this workflow.
    pub name: String,
    /// Description of what this workflow does.
    pub description: Option<String>,
    /// Semantic version of this workflow definition.
    pub version: String,
    /// Whether this workflow is enabled.
    pub enabled: bool,
    /// Tags for organization/filtering.
    pub tags: Vec<String>,
    /// When this workflow was created.
    pub created_at: DateTime<Utc>,
    /// When this workflow was last updated.
    pub updated_at: DateTime<Utc>,
}

impl WorkflowMetadata {
    /// Creates new metadata with default values.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            name: name.into(),
            description: None,
            version: "0.1.0".to_string(),
            enabled: true,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    /// Sets the description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the version.
    #[must_use]
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Adds a tag.
    #[must_use]
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

/// Memory configuration for a workflow.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowMemoryConfig {
    /// Whether this workflow uses memory.
    pub enabled: bool,
    /// Maximum size of memory in bytes.
    pub max_size_bytes: u32,
    /// Whether to automatically load memory at start.
    pub auto_load: bool,
}

impl Default for WorkflowMemoryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            max_size_bytes: 64 * 1024, // 64KB default
            auto_load: true,
        }
    }
}

/// A complete workflow definition.
///
/// This is the source of truth for a workflow. Triggers are denormalized
/// from the graph into a separate table for efficient lookup.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Workflow {
    /// Unique identifier for this workflow.
    pub id: WorkflowId,
    /// Workflow metadata.
    pub metadata: WorkflowMetadata,
    /// The workflow graph (nodes and edges).
    pub graph: WorkflowGraph,
    /// Memory configuration.
    pub memory: WorkflowMemoryConfig,
}

impl Workflow {
    /// Creates a new workflow with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: WorkflowId::new(),
            metadata: WorkflowMetadata::new(name),
            graph: WorkflowGraph::new(),
            memory: WorkflowMemoryConfig::default(),
        }
    }

    /// Creates a workflow with a specific ID.
    #[must_use]
    pub fn with_id(id: WorkflowId, name: impl Into<String>) -> Self {
        Self {
            id,
            metadata: WorkflowMetadata::new(name),
            graph: WorkflowGraph::new(),
            memory: WorkflowMemoryConfig::default(),
        }
    }

    /// Returns the workflow name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.metadata.name
    }

    /// Returns whether the workflow is enabled.
    #[must_use]
    pub fn is_enabled(&self) -> bool {
        self.metadata.enabled
    }

    /// Enables the workflow.
    pub fn enable(&mut self) {
        self.metadata.enabled = true;
        self.metadata.updated_at = Utc::now();
    }

    /// Disables the workflow.
    pub fn disable(&mut self) {
        self.metadata.enabled = false;
        self.metadata.updated_at = Utc::now();
    }

    /// Validates the workflow.
    ///
    /// # Errors
    ///
    /// Returns an error if the workflow graph is invalid.
    pub fn validate(&self) -> Result<(), crate::error::GraphError> {
        self.graph.validate()
    }

    /// Marks the workflow as updated (bumps updated_at timestamp).
    pub fn touch(&mut self) {
        self.metadata.updated_at = Utc::now();
    }
}

/// Summary information about a workflow (for listings).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowSummary {
    /// Workflow ID.
    pub id: WorkflowId,
    /// Workflow name.
    pub name: String,
    /// Description, if any.
    pub description: Option<String>,
    /// Whether enabled.
    pub enabled: bool,
    /// Tags.
    pub tags: Vec<String>,
    /// Number of nodes in the graph.
    pub node_count: usize,
    /// Last updated timestamp.
    pub updated_at: DateTime<Utc>,
}

impl From<&Workflow> for WorkflowSummary {
    fn from(workflow: &Workflow) -> Self {
        Self {
            id: workflow.id,
            name: workflow.metadata.name.clone(),
            description: workflow.metadata.description.clone(),
            enabled: workflow.metadata.enabled,
            tags: workflow.metadata.tags.clone(),
            node_count: workflow.graph.node_count(),
            updated_at: workflow.metadata.updated_at,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workflow_creation() {
        let workflow = Workflow::new("Test Workflow");
        assert_eq!(workflow.name(), "Test Workflow");
        assert!(workflow.is_enabled());
        assert_eq!(workflow.graph.node_count(), 0);
    }

    #[test]
    fn workflow_enable_disable() {
        let mut workflow = Workflow::new("Test");

        workflow.disable();
        assert!(!workflow.is_enabled());

        workflow.enable();
        assert!(workflow.is_enabled());
    }

    #[test]
    fn workflow_metadata_builder() {
        let metadata = WorkflowMetadata::new("My Workflow")
            .with_description("Does something useful")
            .with_version("1.0.0")
            .with_tag("daily")
            .with_tag("email");

        assert_eq!(metadata.name, "My Workflow");
        assert_eq!(metadata.description, Some("Does something useful".to_string()));
        assert_eq!(metadata.version, "1.0.0");
        assert_eq!(metadata.tags, vec!["daily", "email"]);
    }

    #[test]
    fn workflow_summary_from_workflow() {
        let workflow = Workflow::new("Summary Test");
        let summary = WorkflowSummary::from(&workflow);

        assert_eq!(summary.id, workflow.id);
        assert_eq!(summary.name, "Summary Test");
        assert_eq!(summary.node_count, 0);
    }

    #[test]
    fn workflow_serde_roundtrip() {
        let workflow = Workflow::new("Serialization Test");
        let json = serde_json::to_string(&workflow).expect("serialize");
        let mut parsed: Workflow = serde_json::from_str(&json).expect("deserialize");
        parsed.graph.rebuild_index_map();

        assert_eq!(workflow.id, parsed.id);
        assert_eq!(workflow.name(), parsed.name());
    }
}
