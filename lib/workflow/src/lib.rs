//! Workflow engine for the silver-telegram platform.
//!
//! This crate provides the core workflow execution engine, including:
//!
//! - **Graph Model**: Directed graphs using petgraph with typed nodes and edges
//! - **Node Types**: Trigger, AI Layer, Integration, Transform, Control Flow, Memory, Output
//! - **Port System**: Named input/output ports with JSON Schema typing
//! - **Execution**: State machine for tracking workflow runs
//! - **Triggers**: Schedule, event, and manual trigger management

pub mod definition;
pub mod edge;
pub mod error;
pub mod execution;
pub mod graph;
pub mod node;
pub mod port;
pub mod trigger;

pub use definition::{Workflow, WorkflowMetadata};
pub use edge::Edge;
pub use error::{ExecutionError, GraphError, WorkflowError};
pub use execution::{ExecutionState, NodeExecutionState, WorkflowRun};
pub use graph::WorkflowGraph;
pub use node::{Node, NodeCategory, NodeConfig, NodeId};
pub use port::{InputPort, OutputPort, PortSchema};
pub use trigger::{Trigger, TriggerConfig, TriggerType};
