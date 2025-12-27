//! Error types for the AI crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `LlmError`: Low-level LLM backend operations
//! - `PromptError`: Prompt template operations
//! - `CoordinateError`: Coordination session errors
//! - `FeedbackError`: Feedback storage/retrieval errors

use crate::coordinate::CoordinateSessionId;
use crate::llm_call::LlmInvocationId;
use std::fmt;

/// Errors from LLM backend operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LlmError {
    /// Provider is unavailable.
    ProviderUnavailable { provider: String, reason: String },
    /// Request failed.
    RequestFailed { reason: String },
    /// Response parsing failed.
    ResponseParseFailed { reason: String },
    /// Timeout waiting for response.
    Timeout,
    /// Rate limit exceeded.
    RateLimited { retry_after_secs: Option<u64> },
    /// Invalid configuration.
    InvalidConfig { reason: String },
}

impl fmt::Display for LlmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProviderUnavailable { provider, reason } => {
                write!(f, "LLM provider '{provider}' unavailable: {reason}")
            }
            Self::RequestFailed { reason } => {
                write!(f, "LLM request failed: {reason}")
            }
            Self::ResponseParseFailed { reason } => {
                write!(f, "failed to parse LLM response: {reason}")
            }
            Self::Timeout => write!(f, "LLM request timed out"),
            Self::RateLimited { retry_after_secs } => {
                if let Some(secs) = retry_after_secs {
                    write!(f, "rate limited, retry after {secs}s")
                } else {
                    write!(f, "rate limited")
                }
            }
            Self::InvalidConfig { reason } => {
                write!(f, "invalid LLM configuration: {reason}")
            }
        }
    }
}

impl std::error::Error for LlmError {}

/// Errors from prompt operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PromptError {
    /// Template not found.
    TemplateNotFound { name: String },
    /// Missing required variable.
    MissingVariable { template: String, variable: String },
    /// Variable validation failed.
    InvalidVariable {
        template: String,
        variable: String,
        reason: String,
    },
    /// Template parsing failed.
    ParseFailed { reason: String },
}

impl fmt::Display for PromptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TemplateNotFound { name } => {
                write!(f, "prompt template not found: {name}")
            }
            Self::MissingVariable { template, variable } => {
                write!(
                    f,
                    "missing required variable '{variable}' in template '{template}'"
                )
            }
            Self::InvalidVariable {
                template,
                variable,
                reason,
            } => {
                write!(
                    f,
                    "invalid variable '{variable}' in template '{template}': {reason}"
                )
            }
            Self::ParseFailed { reason } => {
                write!(f, "failed to parse prompt template: {reason}")
            }
        }
    }
}

impl std::error::Error for PromptError {}

/// Errors from coordinate operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoordinateError {
    /// Maximum iterations exceeded.
    MaxIterationsExceeded { max: u32, goal: String },
    /// Tool execution failed.
    ToolFailed { tool_name: String, reason: String },
    /// Tool not found.
    ToolNotFound { tool_name: String },
    /// Invalid tool input.
    InvalidToolInput { tool_name: String, reason: String },
    /// Coordinator decision parsing failed.
    DecisionParseFailed { reason: String },
}

impl fmt::Display for CoordinateError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxIterationsExceeded { max, goal } => {
                write!(f, "exceeded {max} iterations for goal: {goal}")
            }
            Self::ToolFailed { tool_name, reason } => {
                write!(f, "tool '{tool_name}' failed: {reason}")
            }
            Self::ToolNotFound { tool_name } => {
                write!(f, "tool not found: {tool_name}")
            }
            Self::InvalidToolInput { tool_name, reason } => {
                write!(f, "invalid input for tool '{tool_name}': {reason}")
            }
            Self::DecisionParseFailed { reason } => {
                write!(f, "failed to parse coordinator decision: {reason}")
            }
        }
    }
}

impl std::error::Error for CoordinateError {}

/// Errors from feedback operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FeedbackError {
    /// Storage operation failed.
    StoreFailed { reason: String },
    /// Retrieval failed.
    RetrieveFailed { reason: String },
    /// Invalid feedback data.
    InvalidData { reason: String },
}

impl fmt::Display for FeedbackError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::StoreFailed { reason } => {
                write!(f, "failed to store feedback: {reason}")
            }
            Self::RetrieveFailed { reason } => {
                write!(f, "failed to retrieve feedback: {reason}")
            }
            Self::InvalidData { reason } => {
                write!(f, "invalid feedback data: {reason}")
            }
        }
    }
}

impl std::error::Error for FeedbackError {}

/// High-level AI operation errors.
///
/// Use these to add context when wrapping lower-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// LLM call context (use as context wrapper).
    LlmCall { invocation_id: LlmInvocationId },
    /// Coordinate session context (use as context wrapper).
    CoordinateSession { session_id: CoordinateSessionId },
    /// Output schema validation failed.
    SchemaValidationFailed { expected: String, actual: String },
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LlmCall { invocation_id } => {
                write!(f, "LLM call {invocation_id} failed")
            }
            Self::CoordinateSession { session_id } => {
                write!(f, "coordinate session {session_id} failed")
            }
            Self::SchemaValidationFailed { expected, actual } => {
                write!(
                    f,
                    "output schema validation failed: expected {expected}, got {actual}"
                )
            }
        }
    }
}

impl std::error::Error for AiError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn llm_error_display() {
        let err = LlmError::ProviderUnavailable {
            provider: "ollama".to_string(),
            reason: "connection refused".to_string(),
        };
        assert!(err.to_string().contains("ollama"));
        assert!(err.to_string().contains("connection refused"));
    }

    #[test]
    fn prompt_error_display() {
        let err = PromptError::MissingVariable {
            template: "classify_email".to_string(),
            variable: "content".to_string(),
        };
        assert!(err.to_string().contains("content"));
        assert!(err.to_string().contains("classify_email"));
    }

    #[test]
    fn coordinate_error_display() {
        let err = CoordinateError::MaxIterationsExceeded {
            max: 10,
            goal: "plan trip".to_string(),
        };
        assert!(err.to_string().contains("10"));
        assert!(err.to_string().contains("plan trip"));
    }

    #[test]
    fn feedback_error_display() {
        let err = FeedbackError::StoreFailed {
            reason: "database unavailable".to_string(),
        };
        assert!(err.to_string().contains("database unavailable"));
    }
}
