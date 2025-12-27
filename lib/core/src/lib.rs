//! Core domain types and utilities for the silver-telegram platform.
//!
//! This crate provides the foundational types, error handling, and shared
//! utilities used throughout the silver-telegram autonomous personal assistant.

pub mod error;
pub mod id;

pub use error::{
    AiError, ConversationError, CredentialError, IntegrationError, Result, SchedulerError,
    WorkflowError,
};
pub use id::{
    ConversationSessionId, CredentialId, IntegrationAccountId, MessageId, NodeExecutionId,
    TriggerId, UserId, WorkflowId, WorkflowRunId,
};
