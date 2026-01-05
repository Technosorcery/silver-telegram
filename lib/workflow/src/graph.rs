//! Workflow graph implementation using petgraph.
//!
//! Workflows are directed graphs where:
//! - Nodes are workflow steps with typed ports
//! - Edges connect output ports to input ports
//!
//! The graph structure is stored as JSONB in the database for flexible
//! schema evolution.

use crate::edge::Edge;
use crate::error::GraphError;
use crate::node::{Node, NodeId};
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A workflow graph using petgraph's directed graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowGraph {
    /// The underlying directed graph.
    #[serde(with = "graph_serde")]
    graph: DiGraph<Node, Edge>,
    /// Map from NodeId to petgraph's NodeIndex for O(1) lookup.
    #[serde(skip)]
    node_index_map: HashMap<NodeId, NodeIndex>,
}

impl WorkflowGraph {
    /// Creates a new empty workflow graph.
    #[must_use]
    pub fn new() -> Self {
        Self {
            graph: DiGraph::new(),
            node_index_map: HashMap::new(),
        }
    }

    /// Adds a node to the graph.
    ///
    /// Returns the node ID.
    pub fn add_node(&mut self, node: Node) -> NodeId {
        let node_id = node.id;
        let index = self.graph.add_node(node);
        self.node_index_map.insert(node_id, index);
        node_id
    }

    /// Removes a node from the graph.
    ///
    /// Also removes all edges connected to this node.
    pub fn remove_node(&mut self, node_id: NodeId) -> Option<Node> {
        let index = self.node_index_map.remove(&node_id)?;
        self.graph.remove_node(index)
    }

    /// Returns a reference to a node by its ID.
    #[must_use]
    pub fn get_node(&self, node_id: NodeId) -> Option<&Node> {
        let index = self.node_index_map.get(&node_id)?;
        self.graph.node_weight(*index)
    }

    /// Returns a mutable reference to a node by its ID.
    pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node> {
        let index = self.node_index_map.get(&node_id)?;
        self.graph.node_weight_mut(*index)
    }

    /// Adds an edge between two nodes.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - Source or target node doesn't exist
    /// - Source port doesn't exist on source node
    /// - Target port doesn't exist on target node
    /// - Port schemas are incompatible
    pub fn add_edge(
        &mut self,
        source_id: NodeId,
        target_id: NodeId,
        edge: Edge,
    ) -> Result<(), GraphError> {
        let source_index = self
            .node_index_map
            .get(&source_id)
            .ok_or(GraphError::NodeNotFound { node_id: source_id })?;

        let target_index = self
            .node_index_map
            .get(&target_id)
            .ok_or(GraphError::NodeNotFound { node_id: target_id })?;

        // Validate ports exist and schemas are compatible
        let source_node = self.graph.node_weight(*source_index).unwrap();
        let target_node = self.graph.node_weight(*target_index).unwrap();

        let source_port = source_node.output_port(&edge.source_port).ok_or_else(|| {
            GraphError::SourcePortNotFound {
                node_id: source_id,
                port_name: edge.source_port.clone(),
            }
        })?;

        let target_port = target_node.input_port(&edge.target_port).ok_or_else(|| {
            GraphError::TargetPortNotFound {
                node_id: target_id,
                port_name: edge.target_port.clone(),
            }
        })?;

        // Check schema compatibility
        if !source_port.schema.is_compatible_with(&target_port.schema) {
            return Err(GraphError::IncompatibleSchemas {
                source_node: source_id,
                source_port: edge.source_port.clone(),
                target_node: target_id,
                target_port: edge.target_port.clone(),
            });
        }

        self.graph.add_edge(*source_index, *target_index, edge);
        Ok(())
    }

    /// Returns all nodes in the graph.
    pub fn nodes(&self) -> impl Iterator<Item = &Node> {
        self.graph.node_weights()
    }

    /// Returns the number of nodes in the graph.
    #[must_use]
    pub fn node_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns the number of edges in the graph.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.graph.edge_count()
    }

    /// Returns nodes that have no incoming edges (entry points).
    pub fn entry_nodes(&self) -> Vec<&Node> {
        self.graph
            .node_indices()
            .filter(|&idx| self.graph.edges_directed(idx, Direction::Incoming).count() == 0)
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Returns nodes that have no outgoing edges (terminal nodes).
    pub fn terminal_nodes(&self) -> Vec<&Node> {
        self.graph
            .node_indices()
            .filter(|&idx| self.graph.edges_directed(idx, Direction::Outgoing).count() == 0)
            .filter_map(|idx| self.graph.node_weight(idx))
            .collect()
    }

    /// Returns the successors (downstream nodes) of a given node.
    pub fn successors(&self, node_id: NodeId) -> Vec<(&Node, &Edge)> {
        let Some(&index) = self.node_index_map.get(&node_id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(index, Direction::Outgoing)
            .filter_map(|edge| {
                let target = self.graph.node_weight(edge.target())?;
                Some((target, edge.weight()))
            })
            .collect()
    }

    /// Returns the predecessors (upstream nodes) of a given node.
    pub fn predecessors(&self, node_id: NodeId) -> Vec<(&Node, &Edge)> {
        let Some(&index) = self.node_index_map.get(&node_id) else {
            return Vec::new();
        };

        self.graph
            .edges_directed(index, Direction::Incoming)
            .filter_map(|edge| {
                let source = self.graph.node_weight(edge.source())?;
                Some((source, edge.weight()))
            })
            .collect()
    }

    /// Validates the workflow graph.
    ///
    /// Checks:
    /// - All required input ports have incoming edges
    /// - No cycles (DAG validation)
    ///
    /// # Errors
    ///
    /// Returns an error describing the validation failure.
    pub fn validate(&self) -> Result<(), GraphError> {
        // Check required inputs
        for node in self.nodes() {
            let incoming_ports: Vec<_> = self
                .predecessors(node.id)
                .iter()
                .map(|(_, edge)| edge.target_port.as_str())
                .collect();

            for input in &node.inputs {
                if input.required && !incoming_ports.contains(&input.name.as_str()) {
                    return Err(GraphError::RequiredInputMissing {
                        node_id: node.id,
                        port_name: input.name.clone(),
                    });
                }
            }
        }

        // Check for cycles using DFS
        if petgraph::algo::is_cyclic_directed(&self.graph) {
            return Err(GraphError::CycleDetected);
        }

        Ok(())
    }

    /// Rebuilds the node index map after deserialization.
    pub fn rebuild_index_map(&mut self) {
        self.node_index_map.clear();
        for index in self.graph.node_indices() {
            if let Some(node) = self.graph.node_weight(index) {
                self.node_index_map.insert(node.id, index);
            }
        }
    }
}

impl Default for WorkflowGraph {
    fn default() -> Self {
        Self::new()
    }
}

/// Custom serde for petgraph DiGraph.
mod graph_serde {
    use super::*;
    use serde::de::{MapAccess, Visitor};
    use serde::ser::SerializeStruct;

    pub fn serialize<S>(graph: &DiGraph<Node, Edge>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let nodes: Vec<_> = graph.node_weights().cloned().collect();
        let edges: Vec<_> = graph
            .edge_references()
            .map(|e| {
                let source_id = graph.node_weight(e.source()).map(|n| n.id);
                let target_id = graph.node_weight(e.target()).map(|n| n.id);
                (source_id, target_id, e.weight().clone())
            })
            .collect();

        let mut state = serializer.serialize_struct("Graph", 2)?;
        state.serialize_field("nodes", &nodes)?;
        state.serialize_field("edges", &edges)?;
        state.end()
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<DiGraph<Node, Edge>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        type EdgeTuple = (Option<NodeId>, Option<NodeId>, Edge);

        struct GraphVisitor;

        impl<'de> Visitor<'de> for GraphVisitor {
            type Value = DiGraph<Node, Edge>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a workflow graph with nodes and edges")
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut nodes: Option<Vec<Node>> = None;
                let mut edges: Option<Vec<EdgeTuple>> = None;

                while let Some(key) = map.next_key::<String>()? {
                    match key.as_str() {
                        "nodes" => nodes = Some(map.next_value()?),
                        "edges" => edges = Some(map.next_value()?),
                        _ => {
                            let _ = map.next_value::<serde::de::IgnoredAny>()?;
                        }
                    }
                }

                let nodes = nodes.unwrap_or_default();
                let edges = edges.unwrap_or_default();

                let mut graph = DiGraph::new();
                let mut id_to_index = HashMap::new();

                for node in nodes {
                    let id = node.id;
                    let index = graph.add_node(node);
                    id_to_index.insert(id, index);
                }

                for (source_id, target_id, edge) in edges {
                    let (Some(source), Some(target)) = (source_id, target_id) else {
                        continue;
                    };
                    let (Some(&source_idx), Some(&target_idx)) =
                        (id_to_index.get(&source), id_to_index.get(&target))
                    else {
                        continue;
                    };
                    graph.add_edge(source_idx, target_idx, edge);
                }

                Ok(graph)
            }
        }

        deserializer.deserialize_struct("Graph", &["nodes", "edges"], GraphVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::{AiLayerNodeConfig, NodeConfig, TriggerNodeConfig};

    fn create_trigger_node(name: &str) -> Node {
        Node::new(
            name,
            NodeConfig::Trigger(TriggerNodeConfig::Schedule {
                cron: "0 7 * * *".to_string(),
                timezone: None,
            }),
        )
    }

    fn create_classify_node(name: &str) -> Node {
        Node::new(
            name,
            NodeConfig::AiLayer(AiLayerNodeConfig::Classify {
                categories: vec!["a".to_string(), "b".to_string()],
            }),
        )
    }

    #[test]
    fn add_and_get_node() {
        let mut graph = WorkflowGraph::new();
        let node = create_trigger_node("Test Trigger");
        let node_id = node.id;
        graph.add_node(node);

        let retrieved = graph.get_node(node_id);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().name, "Test Trigger");
    }

    #[test]
    fn add_edge_validates_ports() {
        let mut graph = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let classify = create_classify_node("Classifier");
        let trigger_id = trigger.id;
        let classify_id = classify.id;

        graph.add_node(trigger);
        graph.add_node(classify);

        // Valid edge
        let result = graph.add_edge(
            trigger_id,
            classify_id,
            Edge::new("output", "content"), // trigger's output -> classify's content input
        );
        assert!(result.is_ok());
    }

    #[test]
    fn add_edge_rejects_missing_port() {
        let mut graph = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let classify = create_classify_node("Classifier");
        let trigger_id = trigger.id;
        let classify_id = classify.id;

        graph.add_node(trigger);
        graph.add_node(classify);

        // Invalid edge - nonexistent port
        let result = graph.add_edge(trigger_id, classify_id, Edge::new("nonexistent", "content"));
        assert!(result.is_err());
    }

    #[test]
    fn entry_nodes_returns_nodes_without_incoming() {
        let mut graph = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let classify = create_classify_node("Classifier");
        let trigger_id = trigger.id;
        let classify_id = classify.id;

        graph.add_node(trigger);
        graph.add_node(classify);
        graph
            .add_edge(trigger_id, classify_id, Edge::new("output", "content"))
            .unwrap();

        let entries = graph.entry_nodes();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "Trigger");
    }

    #[test]
    fn validate_detects_missing_required_input() {
        let mut graph = WorkflowGraph::new();
        // Classify node has required 'model' and 'content' inputs
        // (AI nodes require a model input port)
        let classify = create_classify_node("Classifier");
        graph.add_node(classify);

        let result = graph.validate();
        assert!(result.is_err());
        match result.unwrap_err() {
            GraphError::RequiredInputMissing { port_name, .. } => {
                // 'model' is checked first since it's the first required input
                assert_eq!(port_name, "model");
            }
            _ => panic!("unexpected error type"),
        }
    }

    #[test]
    fn graph_serde_roundtrip() {
        let mut graph = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let classify = create_classify_node("Classifier");
        let trigger_id = trigger.id;
        let classify_id = classify.id;

        graph.add_node(trigger);
        graph.add_node(classify);
        graph
            .add_edge(trigger_id, classify_id, Edge::new("output", "content"))
            .unwrap();

        let json = serde_json::to_string(&graph).expect("serialize");
        let mut parsed: WorkflowGraph = serde_json::from_str(&json).expect("deserialize");
        parsed.rebuild_index_map();

        assert_eq!(parsed.node_count(), 2);
        assert_eq!(parsed.edge_count(), 1);
        assert!(parsed.get_node(trigger_id).is_some());
    }
}
