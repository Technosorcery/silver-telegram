//! Database repositories for the silver-telegram platform.
//!
//! This module provides data access for:
//! - Integration accounts and credentials
//! - Workflows and their components
//! - Workflow runs and execution history

pub mod integration;
pub mod workflow;
pub mod workflow_run;

pub use integration::{
    IntegrationAccount, IntegrationAccountRepository, IntegrationConfigRepository,
};
pub use workflow::{
    TriggerRecord, TriggerRepository, WorkflowMemoryRepository, WorkflowRecord, WorkflowRepository,
};
pub use workflow_run::{
    DecisionTraceRecord, DecisionTraceRepository, NodeExecutionRecord, NodeExecutionRepository,
    NodeState, WorkflowRunRecord, WorkflowRunRepository,
};
