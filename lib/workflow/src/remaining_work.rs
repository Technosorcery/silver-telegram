//! Remaining work graph for workflow execution.
//!
//! Per ADR-006, execution uses a "remaining work graph" algorithm:
//! - Start with full workflow graph
//! - Remove nodes that have completed
//! - Failed nodes get a self-edge (never become ready, block downstream)
//! - Nodes with 0 incoming edges are ready for execution
//! - When no nodes have 0 incoming edges AND no nodes executing → run complete
//!
//! This pattern is similar to DependentValueGraph in systeminit/si.

use crate::execution::NodeExecutionState;
use crate::graph::WorkflowGraph;
use crate::node::NodeId;
use petgraph::Direction;
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use std::collections::{HashMap, HashSet};

/// The remaining work graph tracks which nodes still need to execute.
///
/// This is a simplified view of the workflow graph that:
/// - Excludes completed/skipped nodes
/// - Marks failed nodes with self-edges (blocking)
/// - Provides efficient lookup of ready nodes
#[derive(Debug, Clone)]
pub struct RemainingWorkGraph {
    /// The simplified graph for tracking dependencies.
    /// Node weights are NodeIds, edge weights are ().
    graph: DiGraph<NodeId, ()>,
    /// Map from NodeId to graph index for O(1) lookup.
    node_to_index: HashMap<NodeId, NodeIndex>,
    /// Nodes that are currently executing.
    executing: HashSet<NodeId>,
    /// Nodes that have failed (have self-edges, block downstream).
    failed: HashSet<NodeId>,
}

impl RemainingWorkGraph {
    /// Creates a new remaining work graph from a workflow graph.
    ///
    /// Initially all nodes are pending and included in the graph.
    #[must_use]
    pub fn from_workflow(workflow_graph: &WorkflowGraph) -> Self {
        let mut graph = DiGraph::new();
        let mut node_to_index = HashMap::new();

        // Add all nodes
        for node in workflow_graph.nodes() {
            let idx = graph.add_node(node.id);
            node_to_index.insert(node.id, idx);
        }

        // Add all edges
        for node in workflow_graph.nodes() {
            let source_idx = node_to_index[&node.id];
            for (successor, _edge) in workflow_graph.successors(node.id) {
                let target_idx = node_to_index[&successor.id];
                graph.add_edge(source_idx, target_idx, ());
            }
        }

        Self {
            graph,
            node_to_index,
            executing: HashSet::new(),
            failed: HashSet::new(),
        }
    }

    /// Marks a node as currently executing.
    ///
    /// The node must still be in the remaining work graph.
    pub fn mark_executing(&mut self, node_id: NodeId) {
        if self.node_to_index.contains_key(&node_id) {
            self.executing.insert(node_id);
        }
    }

    /// Marks a node as completed and removes it from the graph.
    ///
    /// This unblocks downstream nodes that were waiting for this node.
    pub fn mark_completed(&mut self, node_id: NodeId) {
        self.executing.remove(&node_id);
        if let Some(idx) = self.node_to_index.remove(&node_id) {
            self.graph.remove_node(idx);
            // Rebuild the index map since removal invalidates indices
            self.rebuild_index_map();
        }
    }

    /// Marks a node as failed.
    ///
    /// Per ADR-006, failed nodes get a self-edge so they never become ready
    /// and block all downstream nodes.
    pub fn mark_failed(&mut self, node_id: NodeId) {
        self.executing.remove(&node_id);
        if let Some(&idx) = self.node_to_index.get(&node_id) {
            // Add self-edge to ensure this node never becomes "ready"
            self.graph.add_edge(idx, idx, ());
            self.failed.insert(node_id);
        }
    }

    /// Marks a node as skipped and removes it from the graph.
    ///
    /// Skipped nodes are treated like completed nodes - they unblock downstream.
    pub fn mark_skipped(&mut self, node_id: NodeId) {
        self.mark_completed(node_id);
    }

    /// Returns nodes that are ready to execute (have no pending predecessors).
    ///
    /// A node is ready when:
    /// - It has 0 incoming edges in the remaining work graph
    /// - It is not already executing
    #[must_use]
    pub fn ready_nodes(&self) -> Vec<NodeId> {
        self.graph
            .node_indices()
            .filter(|&idx| {
                // Has no incoming edges
                self.graph.edges_directed(idx, Direction::Incoming).count() == 0
            })
            .filter_map(|idx| {
                let node_id = self.graph.node_weight(idx)?;
                // Not already executing
                if self.executing.contains(node_id) {
                    return None;
                }
                Some(*node_id)
            })
            .collect()
    }

    /// Returns true if the execution is complete.
    ///
    /// Execution is complete when:
    /// - No nodes are ready (0 incoming edges)
    /// - No nodes are executing
    ///
    /// This happens either when all nodes completed/skipped, or when
    /// remaining nodes are all blocked by failed nodes.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.executing.is_empty() && self.ready_nodes().is_empty()
    }

    /// Returns true if there are any failed nodes.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        !self.failed.is_empty()
    }

    /// Returns the set of failed node IDs.
    #[must_use]
    pub fn failed_nodes(&self) -> &HashSet<NodeId> {
        &self.failed
    }

    /// Returns the set of nodes currently executing.
    #[must_use]
    pub fn executing_nodes(&self) -> &HashSet<NodeId> {
        &self.executing
    }

    /// Returns the number of nodes remaining in the graph.
    ///
    /// This includes executing nodes and blocked nodes.
    #[must_use]
    pub fn remaining_count(&self) -> usize {
        self.graph.node_count()
    }

    /// Returns true if the given node is still in the remaining work graph.
    #[must_use]
    pub fn contains(&self, node_id: NodeId) -> bool {
        self.node_to_index.contains_key(&node_id)
    }

    /// Returns the current state of a node based on the work graph.
    #[must_use]
    pub fn node_state(&self, node_id: NodeId) -> NodeExecutionState {
        if !self.contains(node_id) {
            // Node was removed - either completed or skipped
            return NodeExecutionState::Completed;
        }
        if self.executing.contains(&node_id) {
            return NodeExecutionState::Running;
        }
        if self.failed.contains(&node_id) {
            return NodeExecutionState::Failed;
        }
        if self.ready_nodes().contains(&node_id) {
            return NodeExecutionState::Ready;
        }
        NodeExecutionState::Pending
    }

    /// Returns all nodes that are blocked by failures.
    ///
    /// A node is blocked if it is reachable from a failed node but not failed itself.
    #[must_use]
    pub fn blocked_nodes(&self) -> Vec<NodeId> {
        // Collect all nodes that are downstream of failed nodes
        let mut blocked = HashSet::new();

        for &failed_id in &self.failed {
            if let Some(&start_idx) = self.node_to_index.get(&failed_id) {
                // BFS from failed node to find all downstream nodes
                let mut to_visit = vec![start_idx];
                while let Some(idx) = to_visit.pop() {
                    for edge in self.graph.edges_directed(idx, Direction::Outgoing) {
                        let target_idx = edge.target();
                        // Skip self-edges
                        if target_idx == idx {
                            continue;
                        }
                        if let Some(&target_id) = self.graph.node_weight(target_idx)
                            && !self.failed.contains(&target_id)
                            && blocked.insert(target_id)
                        {
                            to_visit.push(target_idx);
                        }
                    }
                }
            }
        }

        blocked.into_iter().collect()
    }

    /// Rebuilds the node-to-index map after graph modifications.
    fn rebuild_index_map(&mut self) {
        self.node_to_index.clear();
        for idx in self.graph.node_indices() {
            if let Some(&node_id) = self.graph.node_weight(idx) {
                self.node_to_index.insert(node_id, idx);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::Edge;
    use crate::node::{AiLayerNodeConfig, Node, NodeConfig, TriggerNodeConfig};

    fn create_trigger_node(name: &str) -> Node {
        Node::new(
            name,
            NodeConfig::Trigger(TriggerNodeConfig::Schedule {
                cron: "0 7 * * *".to_string(),
                timezone: None,
            }),
        )
    }

    fn create_ai_node(name: &str) -> Node {
        Node::new(
            name,
            NodeConfig::AiLayer(AiLayerNodeConfig::Generate {
                instructions: "Test".to_string(),
            }),
        )
    }

    #[test]
    fn empty_workflow_is_immediately_complete() {
        let workflow = WorkflowGraph::new();
        let work = RemainingWorkGraph::from_workflow(&workflow);

        assert!(work.is_complete());
        assert!(!work.has_failures());
        assert_eq!(work.remaining_count(), 0);
    }

    #[test]
    fn single_node_workflow() {
        let mut workflow = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let trigger_id = trigger.id;
        workflow.add_node(trigger);

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Single node should be ready
        assert_eq!(work.ready_nodes(), vec![trigger_id]);
        assert!(!work.is_complete());

        // Mark as executing
        work.mark_executing(trigger_id);
        assert!(work.ready_nodes().is_empty());
        assert!(!work.is_complete());

        // Mark as completed
        work.mark_completed(trigger_id);
        assert!(work.is_complete());
        assert_eq!(work.remaining_count(), 0);
    }

    #[test]
    fn linear_workflow_execution() {
        let mut workflow = WorkflowGraph::new();

        // A -> B -> C
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let node_c = create_ai_node("C");
        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;

        workflow.add_node(node_a);
        workflow.add_node(node_b);
        workflow.add_node(node_c);
        workflow
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_b, id_c, Edge::new("generated", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Initially only A is ready
        assert_eq!(work.ready_nodes(), vec![id_a]);

        // Execute A
        work.mark_executing(id_a);
        work.mark_completed(id_a);

        // Now B is ready
        assert_eq!(work.ready_nodes(), vec![id_b]);

        // Execute B
        work.mark_executing(id_b);
        work.mark_completed(id_b);

        // Now C is ready
        assert_eq!(work.ready_nodes(), vec![id_c]);

        // Execute C
        work.mark_executing(id_c);
        work.mark_completed(id_c);

        assert!(work.is_complete());
    }

    #[test]
    fn parallel_workflow_execution() {
        let mut workflow = WorkflowGraph::new();

        // A -> B
        //  \-> C
        // (B and C can run in parallel after A)
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let node_c = create_ai_node("C");
        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;

        workflow.add_node(node_a);
        workflow.add_node(node_b);
        workflow.add_node(node_c);
        workflow
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_a, id_c, Edge::new("output", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Complete A
        work.mark_executing(id_a);
        work.mark_completed(id_a);

        // Both B and C should be ready
        let ready = work.ready_nodes();
        assert_eq!(ready.len(), 2);
        assert!(ready.contains(&id_b));
        assert!(ready.contains(&id_c));
    }

    #[test]
    fn join_waits_for_all_predecessors() {
        let mut workflow = WorkflowGraph::new();

        // A -> B -\
        //      C -> D (D waits for both B and C)
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let node_c = create_ai_node("C");
        let node_d = create_ai_node("D");
        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;
        let id_d = node_d.id;

        workflow.add_node(node_a);
        workflow.add_node(node_b);
        workflow.add_node(node_c);
        workflow.add_node(node_d);
        workflow
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_a, id_c, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_b, id_d, Edge::new("generated", "context"))
            .unwrap();
        workflow
            .add_edge(id_c, id_d, Edge::new("generated", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Complete A
        work.mark_executing(id_a);
        work.mark_completed(id_a);

        // B and C are ready
        let ready = work.ready_nodes();
        assert!(ready.contains(&id_b));
        assert!(ready.contains(&id_c));
        assert!(!ready.contains(&id_d)); // D is not ready yet

        // Complete B (but not C)
        work.mark_executing(id_b);
        work.mark_completed(id_b);

        // D still not ready (waiting for C)
        let ready = work.ready_nodes();
        assert!(ready.contains(&id_c));
        assert!(!ready.contains(&id_d));

        // Complete C
        work.mark_executing(id_c);
        work.mark_completed(id_c);

        // Now D is ready
        assert_eq!(work.ready_nodes(), vec![id_d]);
    }

    #[test]
    fn failed_node_blocks_downstream() {
        let mut workflow = WorkflowGraph::new();

        // A -> B -> C
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let node_c = create_ai_node("C");
        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;

        workflow.add_node(node_a);
        workflow.add_node(node_b);
        workflow.add_node(node_c);
        workflow
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_b, id_c, Edge::new("generated", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Complete A
        work.mark_executing(id_a);
        work.mark_completed(id_a);

        // Fail B
        work.mark_executing(id_b);
        work.mark_failed(id_b);

        // Execution is complete (no ready nodes, nothing executing)
        assert!(work.is_complete());
        assert!(work.has_failures());
        assert!(work.failed_nodes().contains(&id_b));

        // C is blocked
        let blocked = work.blocked_nodes();
        assert!(blocked.contains(&id_c));
    }

    #[test]
    fn partial_completion_with_independent_branches() {
        let mut workflow = WorkflowGraph::new();

        // A -> B -> C (independent branch 1)
        // D -> E -> F (independent branch 2)
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let node_c = create_ai_node("C");
        let node_d = create_trigger_node("D");
        let node_e = create_ai_node("E");
        let node_f = create_ai_node("F");

        let id_a = node_a.id;
        let id_b = node_b.id;
        let id_c = node_c.id;
        let id_d = node_d.id;
        let id_e = node_e.id;
        let id_f = node_f.id;

        workflow.add_node(node_a);
        workflow.add_node(node_b);
        workflow.add_node(node_c);
        workflow.add_node(node_d);
        workflow.add_node(node_e);
        workflow.add_node(node_f);

        workflow
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_b, id_c, Edge::new("generated", "context"))
            .unwrap();
        workflow
            .add_edge(id_d, id_e, Edge::new("output", "context"))
            .unwrap();
        workflow
            .add_edge(id_e, id_f, Edge::new("generated", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Both branches start with A and D ready
        let ready = work.ready_nodes();
        assert!(ready.contains(&id_a));
        assert!(ready.contains(&id_d));

        // Complete branch 1: A, B
        work.mark_executing(id_a);
        work.mark_completed(id_a);
        work.mark_executing(id_b);
        work.mark_failed(id_b); // B fails

        // Branch 2 can still proceed
        work.mark_executing(id_d);
        work.mark_completed(id_d);
        work.mark_executing(id_e);
        work.mark_completed(id_e);
        work.mark_executing(id_f);
        work.mark_completed(id_f);

        // Execution is complete
        assert!(work.is_complete());
        // But we have failures
        assert!(work.has_failures());
        // C is blocked by B's failure
        assert!(work.blocked_nodes().contains(&id_c));
    }

    #[test]
    fn node_state_tracking() {
        let mut workflow = WorkflowGraph::new();
        let trigger = create_trigger_node("Trigger");
        let ai = create_ai_node("AI");
        let trigger_id = trigger.id;
        let ai_id = ai.id;

        workflow.add_node(trigger);
        workflow.add_node(ai);
        workflow
            .add_edge(trigger_id, ai_id, Edge::new("output", "context"))
            .unwrap();

        let mut work = RemainingWorkGraph::from_workflow(&workflow);

        // Initially
        assert_eq!(work.node_state(trigger_id), NodeExecutionState::Ready);
        assert_eq!(work.node_state(ai_id), NodeExecutionState::Pending);

        // After marking trigger as executing
        work.mark_executing(trigger_id);
        assert_eq!(work.node_state(trigger_id), NodeExecutionState::Running);

        // After completing trigger
        work.mark_completed(trigger_id);
        assert_eq!(work.node_state(trigger_id), NodeExecutionState::Completed);
        assert_eq!(work.node_state(ai_id), NodeExecutionState::Ready);
    }
}
