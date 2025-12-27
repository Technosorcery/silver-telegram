//! Error types for the workflow crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `GraphError`: Low-level graph operations (nodes, ports, edges)
//! - `ExecutionError`: Workflow execution failures
//! - `WorkflowError`: High-level workflow operations (wraps lower errors via context)

use crate::node::NodeId;
use silver_telegram_core::WorkflowId;
use std::fmt;

/// Errors from graph operations.
///
/// These errors contain only information available at the graph layer.
/// Workflow-level context (like workflow_id) should be added by the caller
/// using `.context()`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraphError {
    /// Node with the given ID was not found in the graph.
    NodeNotFound { node_id: NodeId },
    /// Source port not found on node.
    SourcePortNotFound { node_id: NodeId, port_name: String },
    /// Target port not found on node.
    TargetPortNotFound { node_id: NodeId, port_name: String },
    /// Port schemas are incompatible.
    IncompatibleSchemas {
        source_node: NodeId,
        source_port: String,
        target_node: NodeId,
        target_port: String,
    },
    /// A required input port has no incoming edge.
    RequiredInputMissing { node_id: NodeId, port_name: String },
    /// Graph contains cycles.
    CycleDetected,
}

impl fmt::Display for GraphError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeNotFound { node_id } => {
                write!(f, "node not found: {node_id}")
            }
            Self::SourcePortNotFound { node_id, port_name } => {
                write!(f, "source port '{port_name}' not found on node {node_id}")
            }
            Self::TargetPortNotFound { node_id, port_name } => {
                write!(f, "target port '{port_name}' not found on node {node_id}")
            }
            Self::IncompatibleSchemas {
                source_node,
                source_port,
                target_node,
                target_port,
            } => {
                write!(
                    f,
                    "incompatible schemas: {source_node}:{source_port} -> {target_node}:{target_port}"
                )
            }
            Self::RequiredInputMissing { node_id, port_name } => {
                write!(
                    f,
                    "required input port '{port_name}' on node {node_id} has no incoming edge"
                )
            }
            Self::CycleDetected => write!(f, "graph contains cycles"),
        }
    }
}

impl std::error::Error for GraphError {}

/// Errors during workflow execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExecutionError {
    /// Node execution failed.
    NodeFailed { node_id: NodeId, reason: String },
    /// Node execution timed out.
    NodeTimeout { node_id: NodeId },
    /// Required input data was not provided.
    MissingInput { node_id: NodeId, port_name: String },
    /// Output schema validation failed.
    OutputValidationFailed { node_id: NodeId, reason: String },
    /// Execution was cancelled.
    Cancelled,
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NodeFailed { node_id, reason } => {
                write!(f, "node {node_id} failed: {reason}")
            }
            Self::NodeTimeout { node_id } => {
                write!(f, "node {node_id} timed out")
            }
            Self::MissingInput { node_id, port_name } => {
                write!(f, "missing input '{port_name}' for node {node_id}")
            }
            Self::OutputValidationFailed { node_id, reason } => {
                write!(f, "output validation failed for node {node_id}: {reason}")
            }
            Self::Cancelled => write!(f, "execution cancelled"),
        }
    }
}

impl std::error::Error for ExecutionError {}

/// High-level workflow errors.
///
/// Use these to add workflow context when wrapping lower-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// Workflow not found.
    NotFound { workflow_id: WorkflowId },
    /// Invalid state transition.
    InvalidStateTransition { from: String, to: String },
    /// Error in graph operation (use as context wrapper).
    GraphOperation { workflow_id: WorkflowId },
    /// Error during execution (use as context wrapper).
    Execution { workflow_id: WorkflowId },
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { workflow_id } => {
                write!(f, "workflow not found: {workflow_id}")
            }
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid state transition from {from} to {to}")
            }
            Self::GraphOperation { workflow_id } => {
                write!(f, "graph operation failed for workflow {workflow_id}")
            }
            Self::Execution { workflow_id } => {
                write!(f, "execution failed for workflow {workflow_id}")
            }
        }
    }
}

impl std::error::Error for WorkflowError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn graph_error_display() {
        let node_id = NodeId::new();
        let err = GraphError::NodeNotFound { node_id };
        assert!(err.to_string().contains("node not found"));
    }

    #[test]
    fn graph_error_source_port_not_found() {
        let node_id = NodeId::new();
        let err = GraphError::SourcePortNotFound {
            node_id,
            port_name: "output".to_string(),
        };
        assert!(err.to_string().contains("source port 'output' not found"));
    }

    #[test]
    fn execution_error_display() {
        let node_id = NodeId::new();
        let err = ExecutionError::NodeFailed {
            node_id,
            reason: "timeout".to_string(),
        };
        assert!(err.to_string().contains("failed"));
        assert!(err.to_string().contains("timeout"));
    }

    #[test]
    fn workflow_error_display() {
        let workflow_id = WorkflowId::new();
        let err = WorkflowError::NotFound { workflow_id };
        assert!(err.to_string().contains("workflow not found"));
    }
}
