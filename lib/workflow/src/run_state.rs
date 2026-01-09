//! Run state reconstruction from events.
//!
//! Per ADR-006, the event stream is the source of truth for run state.
//! On crash recovery, state is reconstructed by replaying events.
//!
//! This module provides:
//! - `RunState`: The complete state of a workflow run
//! - `RunStateBuilder`: Reconstructs state from an event stream

use crate::execution::{ExecutionEvent, ExecutionState, NodeExecution};
use crate::graph::WorkflowGraph;
use crate::node::NodeId;
use crate::remaining_work::RemainingWorkGraph;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use silver_telegram_core::{TriggerId, WorkflowId, WorkflowRunId};
use std::collections::HashMap;

/// Complete state of a workflow run.
///
/// This structure holds all information needed to resume execution
/// after a crash or to report on a run's status.
#[derive(Debug, Clone)]
pub struct RunState {
    /// The run ID.
    pub run_id: WorkflowRunId,
    /// The workflow being executed.
    pub workflow_id: WorkflowId,
    /// The trigger that initiated this run, if any.
    pub trigger_id: Option<TriggerId>,
    /// Current execution state of the run.
    pub execution_state: ExecutionState,
    /// When the run was queued.
    pub queued_at: DateTime<Utc>,
    /// When the run started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Input data that triggered the run.
    pub input: Option<JsonValue>,
    /// Final output (if completed).
    pub output: Option<JsonValue>,
    /// Error message (if failed).
    pub error: Option<String>,
    /// Per-node execution state.
    pub node_states: HashMap<NodeId, NodeExecution>,
    /// The remaining work graph for scheduling.
    remaining_work: RemainingWorkGraph,
}

impl RunState {
    /// Returns nodes that are ready to execute.
    #[must_use]
    pub fn ready_nodes(&self) -> Vec<NodeId> {
        self.remaining_work.ready_nodes()
    }

    /// Returns true if the run is complete (terminal state or no more work).
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.execution_state.is_terminal() || self.remaining_work.is_complete()
    }

    /// Returns true if there are any failed nodes.
    #[must_use]
    pub fn has_failures(&self) -> bool {
        self.remaining_work.has_failures()
    }

    /// Returns the remaining work graph for inspection.
    #[must_use]
    pub fn remaining_work(&self) -> &RemainingWorkGraph {
        &self.remaining_work
    }

    /// Marks a node as executing.
    ///
    /// Updates both the node execution record and the remaining work graph.
    pub fn mark_node_executing(&mut self, node_id: NodeId, input: Option<JsonValue>) {
        self.remaining_work.mark_executing(node_id);
        if let Some(node_exec) = self.node_states.get_mut(&node_id) {
            node_exec.start(input);
        }
    }

    /// Marks a node as completed.
    ///
    /// Updates both the node execution record and the remaining work graph.
    pub fn mark_node_completed(&mut self, node_id: NodeId, output_key: String) {
        self.remaining_work.mark_completed(node_id);
        if let Some(node_exec) = self.node_states.get_mut(&node_id) {
            node_exec.complete(output_key);
        }
    }

    /// Marks a node as failed.
    ///
    /// Updates both the node execution record and the remaining work graph.
    pub fn mark_node_failed(&mut self, node_id: NodeId, error: String) {
        self.remaining_work.mark_failed(node_id);
        if let Some(node_exec) = self.node_states.get_mut(&node_id) {
            node_exec.fail(error);
        }
    }

    /// Marks a node as skipped.
    ///
    /// Updates both the node execution record and the remaining work graph.
    pub fn mark_node_skipped(&mut self, node_id: NodeId) {
        self.remaining_work.mark_skipped(node_id);
        if let Some(node_exec) = self.node_states.get_mut(&node_id) {
            node_exec.skip();
        }
    }

    /// Finalizes the run as completed.
    pub fn complete(&mut self, output: Option<JsonValue>, timestamp: DateTime<Utc>) {
        self.execution_state = ExecutionState::Completed;
        self.finished_at = Some(timestamp);
        self.output = output;
    }

    /// Finalizes the run as failed.
    pub fn fail(&mut self, error: String, timestamp: DateTime<Utc>) {
        self.execution_state = ExecutionState::Failed;
        self.finished_at = Some(timestamp);
        self.error = Some(error);
    }

    /// Finalizes the run as cancelled.
    pub fn cancel(&mut self, timestamp: DateTime<Utc>) {
        self.execution_state = ExecutionState::Cancelled;
        self.finished_at = Some(timestamp);
    }
}

/// Builder for reconstructing run state from events.
///
/// This implements event sourcing: the event stream is the source of truth,
/// and we rebuild state by replaying events in order.
pub struct RunStateBuilder {
    workflow_graph: WorkflowGraph,
}

impl RunStateBuilder {
    /// Creates a new builder with the given workflow graph.
    #[must_use]
    pub fn new(workflow_graph: WorkflowGraph) -> Self {
        Self { workflow_graph }
    }

    /// Reconstructs run state from a sequence of events.
    ///
    /// Events must be provided in order (earliest first).
    ///
    /// # Errors
    ///
    /// Returns an error if the event sequence is invalid (e.g., missing RunQueued).
    pub fn build_from_events(
        &self,
        events: impl IntoIterator<Item = ExecutionEvent>,
    ) -> Result<RunState, RunStateError> {
        let mut events_iter = events.into_iter();

        // First event must be RunQueued
        let first_event = events_iter.next().ok_or(RunStateError::NoEvents)?;

        let (run_id, workflow_id, trigger_id, input, queued_at) = match first_event {
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id,
                input,
                timestamp,
            } => (run_id, workflow_id, trigger_id, input, timestamp),
            _ => return Err(RunStateError::MissingRunQueued),
        };

        // Initialize remaining work graph
        let remaining_work = RemainingWorkGraph::from_workflow(&self.workflow_graph);

        // Initialize node states for all nodes
        let mut node_states = HashMap::new();
        for node in self.workflow_graph.nodes() {
            node_states.insert(node.id, NodeExecution::new(run_id, node.id));
        }

        let mut state = RunState {
            run_id,
            workflow_id,
            trigger_id,
            execution_state: ExecutionState::Queued,
            queued_at,
            started_at: None,
            finished_at: None,
            input,
            output: None,
            error: None,
            node_states,
            remaining_work,
        };

        // Replay remaining events
        for event in events_iter {
            apply_event(&mut state, event)?;
        }

        Ok(state)
    }
}

/// Applies a single event to the run state.
fn apply_event(state: &mut RunState, event: ExecutionEvent) -> Result<(), RunStateError> {
    match event {
        ExecutionEvent::RunQueued { .. } => {
            // Duplicate RunQueued is an error
            return Err(RunStateError::DuplicateRunQueued);
        }
        ExecutionEvent::RunStarted { timestamp, .. } => {
            state.execution_state = ExecutionState::Running;
            state.started_at = Some(timestamp);
        }
        ExecutionEvent::NodeStarted { node_id, input, .. } => {
            state.mark_node_executing(node_id, input);
        }
        ExecutionEvent::NodeCompleted {
            node_id,
            output_key,
            ..
        } => {
            state.mark_node_completed(node_id, output_key);
        }
        ExecutionEvent::NodeFailed { node_id, error, .. } => {
            state.mark_node_failed(node_id, error);
        }
        ExecutionEvent::NodeSkipped { node_id, .. } => {
            state.mark_node_skipped(node_id);
        }
        ExecutionEvent::RunCompleted {
            output, timestamp, ..
        } => {
            state.complete(output, timestamp);
        }
        ExecutionEvent::RunFailed {
            error, timestamp, ..
        } => {
            state.fail(error, timestamp);
        }
        ExecutionEvent::RunCancelled { timestamp, .. } => {
            state.cancel(timestamp);
        }
    }
    Ok(())
}

/// Errors that can occur during run state reconstruction.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RunStateError {
    /// No events provided.
    NoEvents,
    /// First event was not RunQueued.
    MissingRunQueued,
    /// Duplicate RunQueued event.
    DuplicateRunQueued,
    /// Event references unknown node.
    UnknownNode { node_id: String },
}

impl std::fmt::Display for RunStateError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoEvents => write!(f, "no events provided"),
            Self::MissingRunQueued => write!(f, "first event must be RunQueued"),
            Self::DuplicateRunQueued => write!(f, "duplicate RunQueued event"),
            Self::UnknownNode { node_id } => write!(f, "unknown node: {node_id}"),
        }
    }
}

impl std::error::Error for RunStateError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::Edge;
    use crate::execution::NodeExecutionState;
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

    fn create_simple_workflow() -> (WorkflowGraph, NodeId, NodeId) {
        let mut graph = WorkflowGraph::new();

        // A -> B
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let id_a = node_a.id;
        let id_b = node_b.id;

        graph.add_node(node_a);
        graph.add_node(node_b);
        graph
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();

        (graph, id_a, id_b)
    }

    #[test]
    fn build_from_run_queued_only() {
        let (graph, id_a, _id_b) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let timestamp = Utc::now();

        let events = vec![ExecutionEvent::RunQueued {
            run_id,
            workflow_id,
            trigger_id: None,
            input: None,
            timestamp,
        }];

        let state = builder.build_from_events(events).unwrap();

        assert_eq!(state.run_id, run_id);
        assert_eq!(state.workflow_id, workflow_id);
        assert_eq!(state.execution_state, ExecutionState::Queued);
        assert!(state.started_at.is_none());
        assert!(!state.is_complete());

        // Only A should be ready
        let ready = state.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&id_a));
    }

    #[test]
    fn build_from_started_run() {
        let (graph, id_a, id_b) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let t1 = Utc::now();
        let t2 = t1 + chrono::Duration::seconds(1);
        let t3 = t2 + chrono::Duration::seconds(1);

        let events = vec![
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::RunStarted {
                run_id,
                timestamp: t2,
            },
            ExecutionEvent::NodeStarted {
                run_id,
                node_id: id_a,
                input: None,
                timestamp: t3,
            },
        ];

        let state = builder.build_from_events(events).unwrap();

        assert_eq!(state.execution_state, ExecutionState::Running);
        assert!(state.started_at.is_some());

        // A is executing, not ready
        assert!(!state.ready_nodes().contains(&id_a));
        assert!(state.remaining_work().executing_nodes().contains(&id_a));

        // B is still pending
        let b_state = state.node_states.get(&id_b).unwrap();
        assert_eq!(b_state.state, NodeExecutionState::Pending);
    }

    #[test]
    fn build_from_partial_execution() {
        let (graph, id_a, id_b) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let t1 = Utc::now();

        let events = vec![
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::RunStarted {
                run_id,
                timestamp: t1,
            },
            ExecutionEvent::NodeStarted {
                run_id,
                node_id: id_a,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::NodeCompleted {
                run_id,
                node_id: id_a,
                output_key: "output_a".to_string(),
                timestamp: t1,
            },
        ];

        let state = builder.build_from_events(events).unwrap();

        // A completed, B now ready
        let ready = state.ready_nodes();
        assert_eq!(ready.len(), 1);
        assert!(ready.contains(&id_b));

        // Verify A's state
        let a_state = state.node_states.get(&id_a).unwrap();
        assert_eq!(a_state.state, NodeExecutionState::Completed);
        assert_eq!(a_state.output_key, Some("output_a".to_string()));
    }

    #[test]
    fn build_from_completed_run() {
        let (graph, id_a, id_b) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let t1 = Utc::now();

        let events = vec![
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::RunStarted {
                run_id,
                timestamp: t1,
            },
            ExecutionEvent::NodeStarted {
                run_id,
                node_id: id_a,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::NodeCompleted {
                run_id,
                node_id: id_a,
                output_key: "output_a".to_string(),
                timestamp: t1,
            },
            ExecutionEvent::NodeStarted {
                run_id,
                node_id: id_b,
                input: Some(serde_json::json!({"data": "test"})),
                timestamp: t1,
            },
            ExecutionEvent::NodeCompleted {
                run_id,
                node_id: id_b,
                output_key: "output_b".to_string(),
                timestamp: t1,
            },
            ExecutionEvent::RunCompleted {
                run_id,
                output: Some(serde_json::json!({"result": "success"})),
                timestamp: t1,
            },
        ];

        let state = builder.build_from_events(events).unwrap();

        assert_eq!(state.execution_state, ExecutionState::Completed);
        assert!(state.is_complete());
        assert!(state.finished_at.is_some());
        assert_eq!(state.output, Some(serde_json::json!({"result": "success"})));
    }

    #[test]
    fn build_from_failed_run() {
        let (graph, id_a, _id_b) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let t1 = Utc::now();

        let events = vec![
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::RunStarted {
                run_id,
                timestamp: t1,
            },
            ExecutionEvent::NodeStarted {
                run_id,
                node_id: id_a,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::NodeFailed {
                run_id,
                node_id: id_a,
                error: "connection timeout".to_string(),
                timestamp: t1,
            },
            ExecutionEvent::RunFailed {
                run_id,
                error: "workflow failed due to node failure".to_string(),
                timestamp: t1,
            },
        ];

        let state = builder.build_from_events(events).unwrap();

        assert_eq!(state.execution_state, ExecutionState::Failed);
        assert!(state.is_complete());
        assert!(state.has_failures());
        assert!(state.error.is_some());
    }

    #[test]
    fn error_on_no_events() {
        let (graph, _, _) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let result = builder.build_from_events(Vec::new());
        assert!(matches!(result, Err(RunStateError::NoEvents)));
    }

    #[test]
    fn error_on_missing_run_queued() {
        let (graph, _, _) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let t1 = Utc::now();

        // Start with RunStarted instead of RunQueued
        let events = vec![ExecutionEvent::RunStarted {
            run_id,
            timestamp: t1,
        }];

        let result = builder.build_from_events(events);
        assert!(matches!(result, Err(RunStateError::MissingRunQueued)));
    }

    #[test]
    fn error_on_duplicate_run_queued() {
        let (graph, _, _) = create_simple_workflow();
        let builder = RunStateBuilder::new(graph);

        let run_id = WorkflowRunId::new();
        let workflow_id = WorkflowId::new();
        let t1 = Utc::now();

        let events = vec![
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
            ExecutionEvent::RunQueued {
                run_id,
                workflow_id,
                trigger_id: None,
                input: None,
                timestamp: t1,
            },
        ];

        let result = builder.build_from_events(events);
        assert!(matches!(result, Err(RunStateError::DuplicateRunQueued)));
    }
}
