//! Error types for the conversation crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `SessionError`: Errors from session operations
//! - `ContextError`: Errors from context store operations
//! - `ToolError`: Errors from tool execution
//! - `ConversationError`: High-level wrapper for context

use silver_telegram_core::ConversationSessionId;
use std::fmt;

/// Errors from session operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionError {
    /// Session not found.
    NotFound { id: ConversationSessionId },
    /// Session has expired.
    Expired { id: ConversationSessionId },
    /// Invalid session state transition.
    InvalidStateTransition { from: String, to: String },
    /// Storage operation failed.
    StorageFailed { reason: String },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "session not found: {id}"),
            Self::Expired { id } => write!(f, "session expired: {id}"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid state transition from {from} to {to}")
            }
            Self::StorageFailed { reason } => {
                write!(f, "session storage failed: {reason}")
            }
        }
    }
}

impl std::error::Error for SessionError {}

/// Errors from context store operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ContextError {
    /// Fact not found.
    FactNotFound { id: String },
    /// Storage operation failed.
    StorageFailed { reason: String },
    /// Query failed.
    QueryFailed { reason: String },
    /// Invalid fact data.
    InvalidData { reason: String },
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FactNotFound { id } => write!(f, "fact not found: {id}"),
            Self::StorageFailed { reason } => {
                write!(f, "context storage failed: {reason}")
            }
            Self::QueryFailed { reason } => {
                write!(f, "context query failed: {reason}")
            }
            Self::InvalidData { reason } => {
                write!(f, "invalid fact data: {reason}")
            }
        }
    }
}

impl std::error::Error for ContextError {}

/// Errors from tool execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolError {
    /// Tool not found.
    NotFound { name: String },
    /// Tool execution failed.
    ExecutionFailed { name: String, reason: String },
    /// Invalid tool input.
    InvalidInput { name: String, reason: String },
    /// Tool requires confirmation.
    RequiresConfirmation { name: String },
}

impl fmt::Display for ToolError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { name } => write!(f, "tool not found: {name}"),
            Self::ExecutionFailed { name, reason } => {
                write!(f, "tool '{name}' execution failed: {reason}")
            }
            Self::InvalidInput { name, reason } => {
                write!(f, "invalid input for tool '{name}': {reason}")
            }
            Self::RequiresConfirmation { name } => {
                write!(f, "tool '{name}' requires user confirmation")
            }
        }
    }
}

impl std::error::Error for ToolError {}

/// High-level conversation errors.
///
/// Use these to add context when wrapping lower-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationError {
    /// Session operation context (use as context wrapper).
    SessionOperation { session_id: ConversationSessionId },
    /// Message processing context (use as context wrapper).
    MessageProcessing { session_id: ConversationSessionId },
    /// Invalid message format.
    InvalidMessage { reason: String },
}

impl fmt::Display for ConversationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionOperation { session_id } => {
                write!(f, "session operation failed for {session_id}")
            }
            Self::MessageProcessing { session_id } => {
                write!(f, "message processing failed for session {session_id}")
            }
            Self::InvalidMessage { reason } => {
                write!(f, "invalid message: {reason}")
            }
        }
    }
}

impl std::error::Error for ConversationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_error_display() {
        let id = ConversationSessionId::new();
        let err = SessionError::NotFound { id };
        assert!(err.to_string().contains("session not found"));
    }

    #[test]
    fn context_error_display() {
        let err = ContextError::FactNotFound {
            id: "fact_123".to_string(),
        };
        assert!(err.to_string().contains("fact not found"));
    }

    #[test]
    fn tool_error_display() {
        let err = ToolError::ExecutionFailed {
            name: "search_emails".to_string(),
            reason: "timeout".to_string(),
        };
        assert!(err.to_string().contains("search_emails"));
        assert!(err.to_string().contains("timeout"));
    }
}
