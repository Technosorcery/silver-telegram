//! Edge types for workflow graphs.
//!
//! Edges connect ports between nodes. Each edge specifies:
//! - The source port (output from one node)
//! - The target port (input on another node)

use crate::node::NodeId;
use serde::{Deserialize, Serialize};

/// An edge connecting two ports in a workflow graph.
///
/// Edges carry data from a source node's output port to a target node's input port.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Edge {
    /// The name of the output port on the source node.
    pub source_port: String,
    /// The name of the input port on the target node.
    pub target_port: String,
}

impl Edge {
    /// Creates a new edge between ports.
    #[must_use]
    pub fn new(source_port: impl Into<String>, target_port: impl Into<String>) -> Self {
        Self {
            source_port: source_port.into(),
            target_port: target_port.into(),
        }
    }

    /// Creates an edge using default port names ("output" -> "input").
    #[must_use]
    pub fn default_ports() -> Self {
        Self::new("output", "input")
    }
}

impl Default for Edge {
    fn default() -> Self {
        Self::default_ports()
    }
}

/// A complete edge reference including source and target node IDs.
///
/// This is used for external representation and validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EdgeRef {
    /// The source node ID.
    pub source_node: NodeId,
    /// The source port name.
    pub source_port: String,
    /// The target node ID.
    pub target_node: NodeId,
    /// The target port name.
    pub target_port: String,
}

impl EdgeRef {
    /// Creates a new edge reference.
    #[must_use]
    pub fn new(
        source_node: NodeId,
        source_port: impl Into<String>,
        target_node: NodeId,
        target_port: impl Into<String>,
    ) -> Self {
        Self {
            source_node,
            source_port: source_port.into(),
            target_node,
            target_port: target_port.into(),
        }
    }

    /// Creates an edge reference using default port names.
    #[must_use]
    pub fn with_default_ports(source_node: NodeId, target_node: NodeId) -> Self {
        Self::new(source_node, "output", target_node, "input")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_default_ports() {
        let edge = Edge::default_ports();
        assert_eq!(edge.source_port, "output");
        assert_eq!(edge.target_port, "input");
    }

    #[test]
    fn edge_custom_ports() {
        let edge = Edge::new("classification", "content");
        assert_eq!(edge.source_port, "classification");
        assert_eq!(edge.target_port, "content");
    }

    #[test]
    fn edge_ref_creation() {
        let source = NodeId::new();
        let target = NodeId::new();
        let edge_ref = EdgeRef::new(source, "out", target, "in");

        assert_eq!(edge_ref.source_node, source);
        assert_eq!(edge_ref.source_port, "out");
        assert_eq!(edge_ref.target_node, target);
        assert_eq!(edge_ref.target_port, "in");
    }

    #[test]
    fn edge_serde_roundtrip() {
        let edge = Edge::new("result", "data");
        let json = serde_json::to_string(&edge).expect("serialize");
        let parsed: Edge = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(edge, parsed);
    }
}
