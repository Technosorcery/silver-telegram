//! Workflow engine for the silver-telegram platform.
//!
//! This crate provides the core workflow execution engine, including:
//!
//! - **Graph Model**: Directed graphs using petgraph with typed nodes and edges
//! - **Node Types**: Trigger, AI Layer, Integration, Transform, Control Flow, Memory, Output
//! - **Port System**: Named input/output ports with JSON Schema typing
//! - **Execution**: State machine for tracking workflow runs
//! - **Triggers**: Schedule, event, and manual trigger management
//! - **Envelope**: Versioned serialization wrapper for schema evolution

pub mod definition;
pub mod edge;
pub mod envelope;
pub mod error;
pub mod execution;
pub mod graph;
pub mod nats;
pub mod node;
pub mod orchestrator;
pub mod port;
pub mod remaining_work;
pub mod run_state;
pub mod trigger;
pub mod worker;

pub use definition::{Workflow, WorkflowMetadata};
pub use edge::Edge;
pub use envelope::{CURRENT_VERSION, Envelope, RawEnvelope};
pub use error::{ExecutionError, GraphError, WorkflowError};
pub use execution::{ExecutionState, NodeExecutionState, WorkflowRun};
pub use graph::WorkflowGraph;
pub use nats::{NatsConfig, NatsEventStore, NatsObjectStore, NatsSetupError, create_nats_stores};
pub use node::{Node, NodeCategory, NodeConfig, NodeId, NodePorts};
pub use orchestrator::{
    EventStore, EventStoreError, Orchestrator, OrchestratorError, WorkItem, WorkItemResult,
};
pub use port::{InputPort, OutputPort, PortSchema};
pub use remaining_work::RemainingWorkGraph;
pub use run_state::{RunState, RunStateBuilder, RunStateError};
pub use trigger::{Trigger, TriggerConfig, TriggerType};
pub use worker::{
    NodeExecutionError, NodeExecutor, ObjectStore, ObjectStoreError, Worker, WorkerError,
};
