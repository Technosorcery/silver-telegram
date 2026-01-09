//! Workflow orchestrator for coordinating execution.
//!
//! Per ADR-006:
//! - One orchestrator per run
//! - Determines ready nodes, publishes work items
//! - Handles graph logic (workers handle execution)
//! - JetStream ack handles crash recovery
//!
//! The orchestrator runs the execution loop:
//! 1. Load/reconstruct run state from events
//! 2. Determine ready nodes
//! 3. Publish work items for workers
//! 4. Process completion/failure events
//! 5. Finalize the run when complete

use crate::definition::Workflow;
use crate::envelope::Envelope;
use crate::execution::{ExecutionEvent, ExecutionState};
use crate::node::NodeId;
use crate::run_state::{RunState, RunStateBuilder, RunStateError};
use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use silver_telegram_core::WorkflowRunId;
use std::collections::HashMap;

/// A work item to be executed by a worker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkItem {
    /// The run this work item belongs to.
    pub run_id: WorkflowRunId,
    /// The node to execute.
    pub node_id: NodeId,
    /// Input data for the node (collected from predecessor outputs).
    pub inputs: HashMap<String, String>, // port_name -> object_store_key
}

/// Result of a work item execution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum WorkItemResult {
    /// Node executed successfully.
    Completed {
        /// The run ID.
        run_id: WorkflowRunId,
        /// The node ID.
        node_id: NodeId,
        /// Object store key for the output.
        output_key: String,
    },
    /// Node execution failed.
    Failed {
        /// The run ID.
        run_id: WorkflowRunId,
        /// The node ID.
        node_id: NodeId,
        /// Error message.
        error: String,
    },
}

/// Trait for event persistence and messaging.
///
/// This abstraction allows the orchestrator to be tested without NATS
/// while still supporting the real NATS implementation in production.
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Publishes an event to the event stream.
    async fn publish(&self, event: Envelope<ExecutionEvent>) -> Result<(), EventStoreError>;

    /// Loads all events for a run.
    async fn load_events(
        &self,
        run_id: WorkflowRunId,
    ) -> Result<Vec<ExecutionEvent>, EventStoreError>;

    /// Publishes a work item for workers to process.
    async fn publish_work_item(&self, item: Envelope<WorkItem>) -> Result<(), EventStoreError>;
}

/// Errors from event store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EventStoreError {
    /// Failed to connect to the event store.
    ConnectionFailed { message: String },
    /// Failed to publish event.
    PublishFailed { message: String },
    /// Failed to load events.
    LoadFailed { message: String },
}

impl std::fmt::Display for EventStoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConnectionFailed { message } => {
                write!(f, "event store connection failed: {message}")
            }
            Self::PublishFailed { message } => write!(f, "event publish failed: {message}"),
            Self::LoadFailed { message } => write!(f, "event load failed: {message}"),
        }
    }
}

impl std::error::Error for EventStoreError {}

/// Errors that can occur during orchestration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OrchestratorError {
    /// Event store error.
    EventStore(EventStoreError),
    /// Run state error.
    RunState(RunStateError),
    /// Run not found.
    RunNotFound { run_id: WorkflowRunId },
    /// Run already in terminal state.
    RunAlreadyTerminal { run_id: WorkflowRunId },
}

impl std::fmt::Display for OrchestratorError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EventStore(e) => write!(f, "event store error: {e}"),
            Self::RunState(e) => write!(f, "run state error: {e}"),
            Self::RunNotFound { run_id } => write!(f, "run not found: {run_id}"),
            Self::RunAlreadyTerminal { run_id } => {
                write!(f, "run already in terminal state: {run_id}")
            }
        }
    }
}

impl std::error::Error for OrchestratorError {}

impl From<EventStoreError> for OrchestratorError {
    fn from(e: EventStoreError) -> Self {
        Self::EventStore(e)
    }
}

impl From<RunStateError> for OrchestratorError {
    fn from(e: RunStateError) -> Self {
        Self::RunState(e)
    }
}

/// The workflow orchestrator.
///
/// Coordinates execution of a single workflow run.
pub struct Orchestrator<E: EventStore> {
    workflow: Workflow,
    event_store: E,
    state: Option<RunState>,
}

impl<E: EventStore> Orchestrator<E> {
    /// Creates a new orchestrator for the given workflow.
    pub fn new(workflow: Workflow, event_store: E) -> Self {
        Self {
            workflow,
            event_store,
            state: None,
        }
    }

    /// Initializes or resumes a run.
    ///
    /// If run_id is provided, loads existing state from events.
    /// Otherwise, creates a new run.
    pub async fn initialize(
        &mut self,
        run_id: Option<WorkflowRunId>,
    ) -> Result<(), OrchestratorError> {
        match run_id {
            Some(id) => self.resume(id).await,
            None => self.start_new_run().await,
        }
    }

    /// Starts a new run.
    async fn start_new_run(&mut self) -> Result<(), OrchestratorError> {
        let run_id = WorkflowRunId::new();
        let workflow_id = self.workflow.id;
        let timestamp = Utc::now();

        // Publish RunQueued event
        let event = ExecutionEvent::RunQueued {
            run_id,
            workflow_id,
            trigger_id: None,
            input: None,
            timestamp,
        };
        self.event_store
            .publish(Envelope::new(event.clone()))
            .await?;

        // Build initial state
        let builder = RunStateBuilder::new(self.workflow.graph.clone());
        let state = builder.build_from_events(vec![event])?;
        self.state = Some(state);

        Ok(())
    }

    /// Resumes an existing run from events.
    async fn resume(&mut self, run_id: WorkflowRunId) -> Result<(), OrchestratorError> {
        let events = self.event_store.load_events(run_id).await?;
        if events.is_empty() {
            return Err(OrchestratorError::RunNotFound { run_id });
        }

        let builder = RunStateBuilder::new(self.workflow.graph.clone());
        let state = builder.build_from_events(events)?;

        if state.execution_state.is_terminal() {
            return Err(OrchestratorError::RunAlreadyTerminal { run_id });
        }

        self.state = Some(state);
        Ok(())
    }

    /// Starts execution of the run.
    ///
    /// Publishes RunStarted event and schedules ready nodes.
    pub async fn start(&mut self) -> Result<(), OrchestratorError> {
        let state = self.state.as_mut().ok_or(OrchestratorError::RunNotFound {
            run_id: WorkflowRunId::new(), // placeholder
        })?;

        if state.execution_state != ExecutionState::Queued {
            return Ok(()); // Already started
        }

        let run_id = state.run_id;
        let timestamp = Utc::now();

        // Publish RunStarted event
        let event = ExecutionEvent::RunStarted { run_id, timestamp };
        self.event_store.publish(Envelope::new(event)).await?;
        state.execution_state = ExecutionState::Running;
        state.started_at = Some(timestamp);

        // Schedule ready nodes
        self.schedule_ready_nodes().await?;

        Ok(())
    }

    /// Schedules all ready nodes for execution.
    async fn schedule_ready_nodes(&mut self) -> Result<(), OrchestratorError> {
        // First, collect all the information we need while borrowing immutably
        let (run_id, nodes_to_schedule) = {
            let state = self.state.as_ref().ok_or(OrchestratorError::RunNotFound {
                run_id: WorkflowRunId::new(),
            })?;

            let run_id = state.run_id;
            let ready = state.ready_nodes();

            // Collect inputs for each ready node
            let nodes_to_schedule: Vec<(NodeId, HashMap<String, String>)> = ready
                .into_iter()
                .map(|node_id| {
                    let inputs = self.collect_inputs_immutable(state, node_id);
                    (node_id, inputs)
                })
                .collect();

            (run_id, nodes_to_schedule)
        };

        // Now process each node
        let timestamp = Utc::now();
        for (node_id, inputs) in nodes_to_schedule {
            let input_json = serde_json::to_value(&inputs).unwrap_or(JsonValue::Null);

            // Publish NodeStarted event
            let event = ExecutionEvent::NodeStarted {
                run_id,
                node_id,
                input: Some(input_json.clone()),
                timestamp,
            };
            self.event_store.publish(Envelope::new(event)).await?;

            // Update state
            if let Some(state) = self.state.as_mut() {
                state.mark_node_executing(node_id, Some(input_json));
            }

            // Publish work item for workers
            let work_item = WorkItem {
                run_id,
                node_id,
                inputs,
            };
            self.event_store
                .publish_work_item(Envelope::new(work_item))
                .await?;
        }

        Ok(())
    }

    /// Collects inputs for a node from predecessor outputs (immutable borrow version).
    fn collect_inputs_immutable(
        &self,
        state: &RunState,
        node_id: NodeId,
    ) -> HashMap<String, String> {
        let mut inputs = HashMap::new();

        // Get predecessors from workflow graph
        for (predecessor, edge) in self.workflow.graph.predecessors(node_id) {
            if let Some(exec) = state.node_states.get(&predecessor.id)
                && let Some(output_key) = &exec.output_key
            {
                // Map output port to input port
                inputs.insert(edge.target_port.clone(), output_key.clone());
            }
        }

        inputs
    }

    /// Handles a work item result (completion or failure).
    pub async fn handle_result(&mut self, result: WorkItemResult) -> Result<(), OrchestratorError> {
        let state = self.state.as_mut().ok_or(OrchestratorError::RunNotFound {
            run_id: WorkflowRunId::new(),
        })?;

        let timestamp = Utc::now();

        match result {
            WorkItemResult::Completed {
                run_id,
                node_id,
                output_key,
            } => {
                // Publish NodeCompleted event
                let event = ExecutionEvent::NodeCompleted {
                    run_id,
                    node_id,
                    output_key: output_key.clone(),
                    timestamp,
                };
                self.event_store.publish(Envelope::new(event)).await?;
                state.mark_node_completed(node_id, output_key);
            }
            WorkItemResult::Failed {
                run_id,
                node_id,
                error,
            } => {
                // Publish NodeFailed event
                let event = ExecutionEvent::NodeFailed {
                    run_id,
                    node_id,
                    error: error.clone(),
                    timestamp,
                };
                self.event_store.publish(Envelope::new(event)).await?;
                state.mark_node_failed(node_id, error);
            }
        }

        // Check if run is complete
        if state.remaining_work().is_complete() {
            self.finalize_run().await?;
        } else {
            // Schedule any newly ready nodes
            self.schedule_ready_nodes().await?;
        }

        Ok(())
    }

    /// Finalizes the run (marks as completed or failed).
    async fn finalize_run(&mut self) -> Result<(), OrchestratorError> {
        let state = self.state.as_mut().ok_or(OrchestratorError::RunNotFound {
            run_id: WorkflowRunId::new(),
        })?;

        let run_id = state.run_id;
        let timestamp = Utc::now();

        if state.has_failures() {
            // Run failed due to node failures
            let event = ExecutionEvent::RunFailed {
                run_id,
                error: "workflow failed due to node failures".to_string(),
                timestamp,
            };
            self.event_store.publish(Envelope::new(event)).await?;
            state.fail(
                "workflow failed due to node failures".to_string(),
                timestamp,
            );
        } else {
            // Run completed successfully
            let event = ExecutionEvent::RunCompleted {
                run_id,
                output: None, // TODO: collect final output
                timestamp,
            };
            self.event_store.publish(Envelope::new(event)).await?;
            state.complete(None, timestamp);
        }

        Ok(())
    }

    /// Returns the current run state.
    #[must_use]
    pub fn state(&self) -> Option<&RunState> {
        self.state.as_ref()
    }

    /// Returns the run ID.
    #[must_use]
    pub fn run_id(&self) -> Option<WorkflowRunId> {
        self.state.as_ref().map(|s| s.run_id)
    }

    /// Returns true if the run is complete.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.state.as_ref().is_some_and(|s| s.is_complete())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::edge::Edge;
    use crate::node::{AiLayerNodeConfig, Node, NodeConfig, TriggerNodeConfig};
    use std::sync::{Arc, Mutex};

    /// In-memory event store for testing.
    struct InMemoryEventStore {
        events: Arc<Mutex<Vec<Envelope<ExecutionEvent>>>>,
        work_items: Arc<Mutex<Vec<Envelope<WorkItem>>>>,
    }

    impl InMemoryEventStore {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
                work_items: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn events(&self) -> Vec<ExecutionEvent> {
            self.events
                .lock()
                .unwrap()
                .iter()
                .map(|e| e.payload.clone())
                .collect()
        }

        fn work_items(&self) -> Vec<WorkItem> {
            self.work_items
                .lock()
                .unwrap()
                .iter()
                .map(|w| w.payload.clone())
                .collect()
        }
    }

    #[async_trait]
    impl EventStore for InMemoryEventStore {
        async fn publish(&self, event: Envelope<ExecutionEvent>) -> Result<(), EventStoreError> {
            self.events.lock().unwrap().push(event);
            Ok(())
        }

        async fn load_events(
            &self,
            run_id: WorkflowRunId,
        ) -> Result<Vec<ExecutionEvent>, EventStoreError> {
            Ok(self
                .events
                .lock()
                .unwrap()
                .iter()
                .filter(|e| e.payload.run_id() == run_id)
                .map(|e| e.payload.clone())
                .collect())
        }

        async fn publish_work_item(&self, item: Envelope<WorkItem>) -> Result<(), EventStoreError> {
            self.work_items.lock().unwrap().push(item);
            Ok(())
        }
    }

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

    fn create_simple_workflow() -> (Workflow, NodeId, NodeId) {
        let mut workflow = Workflow::new("Test Workflow");

        // A -> B
        let node_a = create_trigger_node("A");
        let node_b = create_ai_node("B");
        let id_a = node_a.id;
        let id_b = node_b.id;

        workflow.graph.add_node(node_a);
        workflow.graph.add_node(node_b);
        workflow
            .graph
            .add_edge(id_a, id_b, Edge::new("output", "context"))
            .unwrap();

        (workflow, id_a, id_b)
    }

    #[tokio::test]
    async fn orchestrator_starts_new_run() {
        let (workflow, id_a, _id_b) = create_simple_workflow();
        let event_store = InMemoryEventStore::new();
        let mut orchestrator = Orchestrator::new(workflow, event_store);

        orchestrator.initialize(None).await.unwrap();
        orchestrator.start().await.unwrap();

        let events = orchestrator.event_store.events();
        assert_eq!(events.len(), 3); // RunQueued, RunStarted, NodeStarted

        match &events[0] {
            ExecutionEvent::RunQueued { .. } => {}
            _ => panic!("expected RunQueued"),
        }
        match &events[1] {
            ExecutionEvent::RunStarted { .. } => {}
            _ => panic!("expected RunStarted"),
        }
        match &events[2] {
            ExecutionEvent::NodeStarted { node_id, .. } => {
                assert_eq!(*node_id, id_a);
            }
            _ => panic!("expected NodeStarted"),
        }

        // Should have published work item for node A
        let work_items = orchestrator.event_store.work_items();
        assert_eq!(work_items.len(), 1);
        assert_eq!(work_items[0].node_id, id_a);
    }

    #[tokio::test]
    async fn orchestrator_handles_completion() {
        let (workflow, id_a, id_b) = create_simple_workflow();
        let event_store = InMemoryEventStore::new();
        let mut orchestrator = Orchestrator::new(workflow, event_store);

        orchestrator.initialize(None).await.unwrap();
        orchestrator.start().await.unwrap();

        let run_id = orchestrator.run_id().unwrap();

        // Complete node A
        orchestrator
            .handle_result(WorkItemResult::Completed {
                run_id,
                node_id: id_a,
                output_key: "output_a".to_string(),
            })
            .await
            .unwrap();

        // Node B should now be scheduled
        let work_items = orchestrator.event_store.work_items();
        assert_eq!(work_items.len(), 2); // A and B
        assert_eq!(work_items[1].node_id, id_b);

        // Complete node B
        orchestrator
            .handle_result(WorkItemResult::Completed {
                run_id,
                node_id: id_b,
                output_key: "output_b".to_string(),
            })
            .await
            .unwrap();

        // Run should be complete
        assert!(orchestrator.is_complete());
        let events = orchestrator.event_store.events();
        let last_event = events.last().unwrap();
        match last_event {
            ExecutionEvent::RunCompleted { .. } => {}
            _ => panic!("expected RunCompleted"),
        }
    }

    #[tokio::test]
    async fn orchestrator_handles_failure() {
        let (workflow, id_a, _id_b) = create_simple_workflow();
        let event_store = InMemoryEventStore::new();
        let mut orchestrator = Orchestrator::new(workflow, event_store);

        orchestrator.initialize(None).await.unwrap();
        orchestrator.start().await.unwrap();

        let run_id = orchestrator.run_id().unwrap();

        // Fail node A
        orchestrator
            .handle_result(WorkItemResult::Failed {
                run_id,
                node_id: id_a,
                error: "test error".to_string(),
            })
            .await
            .unwrap();

        // Run should be complete (with failure)
        assert!(orchestrator.is_complete());
        let state = orchestrator.state().unwrap();
        assert!(state.has_failures());
        assert_eq!(state.execution_state, ExecutionState::Failed);
    }

    #[tokio::test]
    async fn orchestrator_collects_inputs() {
        let (workflow, id_a, id_b) = create_simple_workflow();
        let event_store = InMemoryEventStore::new();
        let mut orchestrator = Orchestrator::new(workflow, event_store);

        orchestrator.initialize(None).await.unwrap();
        orchestrator.start().await.unwrap();

        let run_id = orchestrator.run_id().unwrap();

        // Complete node A
        orchestrator
            .handle_result(WorkItemResult::Completed {
                run_id,
                node_id: id_a,
                output_key: "output_key_123".to_string(),
            })
            .await
            .unwrap();

        // B's work item should have A's output as input
        let work_items = orchestrator.event_store.work_items();
        let b_work_item = work_items.iter().find(|w| w.node_id == id_b).unwrap();
        assert_eq!(
            b_work_item.inputs.get("context"),
            Some(&"output_key_123".to_string())
        );
    }
}
