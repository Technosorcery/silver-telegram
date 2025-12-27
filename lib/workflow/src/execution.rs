//! Workflow execution state machine.
//!
//! Per ADR-006, execution uses event sourcing with per-node completion persistence.
//! The state machine tracks:
//! - Overall run state
//! - Per-node execution state
//! - Remaining work graph

use crate::node::NodeId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use silver_telegram_core::{NodeExecutionId, TriggerId, WorkflowId, WorkflowRunId};

/// The overall state of a workflow run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionState {
    /// Run is queued, waiting for an orchestrator.
    Queued,
    /// Run is actively executing.
    Running,
    /// Run completed successfully (all nodes completed or skipped).
    Completed,
    /// Run failed (at least one node failed, blocking downstream).
    Failed,
    /// Run was cancelled by user or system.
    Cancelled,
}

impl ExecutionState {
    /// Returns true if this is a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled
        )
    }
}

/// The execution state of a single node within a run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeExecutionState {
    /// Node is waiting for predecessors to complete.
    Pending,
    /// Node is ready to execute (all predecessors complete).
    Ready,
    /// Node is currently executing.
    Running,
    /// Node completed successfully.
    Completed,
    /// Node failed.
    Failed,
    /// Node was skipped (e.g., branch not taken).
    Skipped,
}

impl NodeExecutionState {
    /// Returns true if this is a terminal state.
    #[must_use]
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Skipped
        )
    }

    /// Returns true if this node blocks downstream nodes.
    #[must_use]
    pub fn blocks_downstream(&self) -> bool {
        matches!(self, Self::Failed)
    }
}

/// A record of a single workflow run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowRun {
    /// Unique identifier for this run.
    pub id: WorkflowRunId,
    /// The workflow being executed.
    pub workflow_id: WorkflowId,
    /// The trigger that initiated this run, if any.
    pub trigger_id: Option<TriggerId>,
    /// Current execution state.
    pub state: ExecutionState,
    /// When the run was queued.
    pub queued_at: DateTime<Utc>,
    /// When the run started executing.
    pub started_at: Option<DateTime<Utc>>,
    /// When the run finished (completed, failed, or cancelled).
    pub finished_at: Option<DateTime<Utc>>,
    /// Input data that triggered the run.
    pub input: Option<JsonValue>,
    /// Final output of the run (if completed).
    pub output: Option<JsonValue>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl WorkflowRun {
    /// Creates a new workflow run in queued state.
    #[must_use]
    pub fn new(workflow_id: WorkflowId, trigger_id: Option<TriggerId>, input: Option<JsonValue>) -> Self {
        Self {
            id: WorkflowRunId::new(),
            workflow_id,
            trigger_id,
            state: ExecutionState::Queued,
            queued_at: Utc::now(),
            started_at: None,
            finished_at: None,
            input,
            output: None,
            error: None,
        }
    }

    /// Starts the run.
    pub fn start(&mut self) {
        self.state = ExecutionState::Running;
        self.started_at = Some(Utc::now());
    }

    /// Marks the run as completed.
    pub fn complete(&mut self, output: Option<JsonValue>) {
        self.state = ExecutionState::Completed;
        self.finished_at = Some(Utc::now());
        self.output = output;
    }

    /// Marks the run as failed.
    pub fn fail(&mut self, error: String) {
        self.state = ExecutionState::Failed;
        self.finished_at = Some(Utc::now());
        self.error = Some(error);
    }

    /// Marks the run as cancelled.
    pub fn cancel(&mut self) {
        self.state = ExecutionState::Cancelled;
        self.finished_at = Some(Utc::now());
    }

    /// Returns the duration of the run, if it has started.
    #[must_use]
    pub fn duration(&self) -> Option<chrono::Duration> {
        let start = self.started_at?;
        let end = self.finished_at.unwrap_or_else(Utc::now);
        Some(end - start)
    }
}

/// Execution record for a single node within a run.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct NodeExecution {
    /// Unique identifier for this node execution.
    pub id: NodeExecutionId,
    /// The run this execution belongs to.
    pub run_id: WorkflowRunId,
    /// The node being executed.
    pub node_id: NodeId,
    /// Current execution state.
    pub state: NodeExecutionState,
    /// When execution started.
    pub started_at: Option<DateTime<Utc>>,
    /// When execution finished.
    pub finished_at: Option<DateTime<Utc>>,
    /// Input data received.
    pub input: Option<JsonValue>,
    /// Output data produced (stored in NATS Object Store, this is the key).
    pub output_key: Option<String>,
    /// Error message if failed.
    pub error: Option<String>,
}

impl NodeExecution {
    /// Creates a new node execution in pending state.
    #[must_use]
    pub fn new(run_id: WorkflowRunId, node_id: NodeId) -> Self {
        Self {
            id: NodeExecutionId::new(),
            run_id,
            node_id,
            state: NodeExecutionState::Pending,
            started_at: None,
            finished_at: None,
            input: None,
            output_key: None,
            error: None,
        }
    }

    /// Marks the node as ready to execute.
    pub fn mark_ready(&mut self) {
        self.state = NodeExecutionState::Ready;
    }

    /// Starts execution of this node.
    pub fn start(&mut self, input: Option<JsonValue>) {
        self.state = NodeExecutionState::Running;
        self.started_at = Some(Utc::now());
        self.input = input;
    }

    /// Marks the node as completed.
    pub fn complete(&mut self, output_key: String) {
        self.state = NodeExecutionState::Completed;
        self.finished_at = Some(Utc::now());
        self.output_key = Some(output_key);
    }

    /// Marks the node as failed.
    pub fn fail(&mut self, error: String) {
        self.state = NodeExecutionState::Failed;
        self.finished_at = Some(Utc::now());
        self.error = Some(error);
    }

    /// Marks the node as skipped.
    pub fn skip(&mut self) {
        self.state = NodeExecutionState::Skipped;
        self.finished_at = Some(Utc::now());
    }
}

/// Events for workflow execution (for event sourcing).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ExecutionEvent {
    /// Run was queued.
    RunQueued {
        run_id: WorkflowRunId,
        workflow_id: WorkflowId,
        trigger_id: Option<TriggerId>,
        input: Option<JsonValue>,
        timestamp: DateTime<Utc>,
    },
    /// Run started executing.
    RunStarted {
        run_id: WorkflowRunId,
        timestamp: DateTime<Utc>,
    },
    /// Node started executing.
    NodeStarted {
        run_id: WorkflowRunId,
        node_id: NodeId,
        input: Option<JsonValue>,
        timestamp: DateTime<Utc>,
    },
    /// Node completed successfully.
    NodeCompleted {
        run_id: WorkflowRunId,
        node_id: NodeId,
        output_key: String,
        timestamp: DateTime<Utc>,
    },
    /// Node failed.
    NodeFailed {
        run_id: WorkflowRunId,
        node_id: NodeId,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Node was skipped.
    NodeSkipped {
        run_id: WorkflowRunId,
        node_id: NodeId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
    /// Run completed successfully.
    RunCompleted {
        run_id: WorkflowRunId,
        output: Option<JsonValue>,
        timestamp: DateTime<Utc>,
    },
    /// Run failed.
    RunFailed {
        run_id: WorkflowRunId,
        error: String,
        timestamp: DateTime<Utc>,
    },
    /// Run was cancelled.
    RunCancelled {
        run_id: WorkflowRunId,
        reason: String,
        timestamp: DateTime<Utc>,
    },
}

impl ExecutionEvent {
    /// Returns the run ID associated with this event.
    #[must_use]
    pub fn run_id(&self) -> WorkflowRunId {
        match self {
            Self::RunQueued { run_id, .. }
            | Self::RunStarted { run_id, .. }
            | Self::NodeStarted { run_id, .. }
            | Self::NodeCompleted { run_id, .. }
            | Self::NodeFailed { run_id, .. }
            | Self::NodeSkipped { run_id, .. }
            | Self::RunCompleted { run_id, .. }
            | Self::RunFailed { run_id, .. }
            | Self::RunCancelled { run_id, .. } => *run_id,
        }
    }

    /// Returns the timestamp of this event.
    #[must_use]
    pub fn timestamp(&self) -> DateTime<Utc> {
        match self {
            Self::RunQueued { timestamp, .. }
            | Self::RunStarted { timestamp, .. }
            | Self::NodeStarted { timestamp, .. }
            | Self::NodeCompleted { timestamp, .. }
            | Self::NodeFailed { timestamp, .. }
            | Self::NodeSkipped { timestamp, .. }
            | Self::RunCompleted { timestamp, .. }
            | Self::RunFailed { timestamp, .. }
            | Self::RunCancelled { timestamp, .. } => *timestamp,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn execution_state_terminal() {
        assert!(!ExecutionState::Queued.is_terminal());
        assert!(!ExecutionState::Running.is_terminal());
        assert!(ExecutionState::Completed.is_terminal());
        assert!(ExecutionState::Failed.is_terminal());
        assert!(ExecutionState::Cancelled.is_terminal());
    }

    #[test]
    fn node_state_blocks_downstream() {
        assert!(!NodeExecutionState::Completed.blocks_downstream());
        assert!(NodeExecutionState::Failed.blocks_downstream());
        assert!(!NodeExecutionState::Skipped.blocks_downstream());
    }

    #[test]
    fn workflow_run_lifecycle() {
        let workflow_id = WorkflowId::new();
        let mut run = WorkflowRun::new(workflow_id, None, None);

        assert_eq!(run.state, ExecutionState::Queued);
        assert!(run.started_at.is_none());

        run.start();
        assert_eq!(run.state, ExecutionState::Running);
        assert!(run.started_at.is_some());

        run.complete(Some(serde_json::json!({"result": "ok"})));
        assert_eq!(run.state, ExecutionState::Completed);
        assert!(run.finished_at.is_some());
        assert!(run.output.is_some());
    }

    #[test]
    fn node_execution_lifecycle() {
        let run_id = WorkflowRunId::new();
        let node_id = NodeId::new();
        let mut exec = NodeExecution::new(run_id, node_id);

        assert_eq!(exec.state, NodeExecutionState::Pending);

        exec.mark_ready();
        assert_eq!(exec.state, NodeExecutionState::Ready);

        exec.start(Some(serde_json::json!({"input": "data"})));
        assert_eq!(exec.state, NodeExecutionState::Running);

        exec.complete("output_123".to_string());
        assert_eq!(exec.state, NodeExecutionState::Completed);
        assert_eq!(exec.output_key, Some("output_123".to_string()));
    }

    #[test]
    fn execution_event_serde_roundtrip() {
        let event = ExecutionEvent::NodeCompleted {
            run_id: WorkflowRunId::new(),
            node_id: NodeId::new(),
            output_key: "key_123".to_string(),
            timestamp: Utc::now(),
        };

        let json = serde_json::to_string(&event).expect("serialize");
        let parsed: ExecutionEvent = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(event.run_id(), parsed.run_id());
    }
}
