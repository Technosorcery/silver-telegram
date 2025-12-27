//! Error types for the silver-telegram platform.
//!
//! This module provides the foundation for error handling using the rootcause crate.
//! Each domain area has its own error type with specific variants for that domain.

use rootcause::Report;
use std::fmt;

/// A Result type alias using rootcause's Report for error handling.
pub type Result<T, C = ()> = std::result::Result<T, Report<C>>;

/// Errors related to workflow operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WorkflowError {
    /// Workflow with the given ID was not found.
    NotFound { id: String },
    /// Invalid state transition was attempted.
    InvalidStateTransition { from: String, to: String },
    /// Workflow definition is invalid.
    InvalidDefinition { reason: String },
    /// Node with the given ID was not found in the workflow.
    NodeNotFound { workflow_id: String, node_id: String },
    /// Port connection is invalid.
    InvalidPortConnection {
        source_node: String,
        source_port: String,
        target_node: String,
        target_port: String,
        reason: String,
    },
    /// Schema validation failed.
    SchemaValidation { node_id: String, reason: String },
    /// Workflow execution failed.
    ExecutionFailed { run_id: String, reason: String },
    /// A required input port has no incoming edge.
    RequiredInputMissing { node_id: String, port_name: String },
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "workflow not found: {id}"),
            Self::InvalidStateTransition { from, to } => {
                write!(f, "invalid state transition from {from} to {to}")
            }
            Self::InvalidDefinition { reason } => {
                write!(f, "invalid workflow definition: {reason}")
            }
            Self::NodeNotFound { workflow_id, node_id } => {
                write!(f, "node {node_id} not found in workflow {workflow_id}")
            }
            Self::InvalidPortConnection {
                source_node,
                source_port,
                target_node,
                target_port,
                reason,
            } => {
                write!(
                    f,
                    "invalid connection from {source_node}:{source_port} to {target_node}:{target_port}: {reason}"
                )
            }
            Self::SchemaValidation { node_id, reason } => {
                write!(f, "schema validation failed for node {node_id}: {reason}")
            }
            Self::ExecutionFailed { run_id, reason } => {
                write!(f, "workflow run {run_id} failed: {reason}")
            }
            Self::RequiredInputMissing { node_id, port_name } => {
                write!(
                    f,
                    "required input port {port_name} on node {node_id} has no incoming edge"
                )
            }
        }
    }
}

impl std::error::Error for WorkflowError {}

/// Errors related to conversation operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversationError {
    /// Session with the given ID was not found.
    SessionNotFound { id: String },
    /// Session has expired.
    SessionExpired { id: String },
    /// Invalid message format.
    InvalidMessage { reason: String },
    /// Context retrieval failed.
    ContextRetrievalFailed { reason: String },
    /// Tool invocation failed.
    ToolInvocationFailed { tool_name: String, reason: String },
}

impl fmt::Display for ConversationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SessionNotFound { id } => write!(f, "session not found: {id}"),
            Self::SessionExpired { id } => write!(f, "session expired: {id}"),
            Self::InvalidMessage { reason } => write!(f, "invalid message: {reason}"),
            Self::ContextRetrievalFailed { reason } => {
                write!(f, "context retrieval failed: {reason}")
            }
            Self::ToolInvocationFailed { tool_name, reason } => {
                write!(f, "tool {tool_name} invocation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for ConversationError {}

/// Errors related to integration operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    /// Integration with the given ID was not found.
    NotFound { id: String },
    /// Connection to external service failed.
    ConnectionFailed { service: String, reason: String },
    /// Authentication failed.
    AuthenticationFailed { service: String, reason: String },
    /// Rate limit exceeded.
    RateLimitExceeded { service: String, retry_after_secs: Option<u64> },
    /// Invalid credentials.
    InvalidCredentials { service: String },
    /// Operation not supported by this integration.
    OperationNotSupported { service: String, operation: String },
    /// Protocol error.
    ProtocolError { service: String, reason: String },
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "integration not found: {id}"),
            Self::ConnectionFailed { service, reason } => {
                write!(f, "connection to {service} failed: {reason}")
            }
            Self::AuthenticationFailed { service, reason } => {
                write!(f, "authentication with {service} failed: {reason}")
            }
            Self::RateLimitExceeded {
                service,
                retry_after_secs,
            } => {
                if let Some(secs) = retry_after_secs {
                    write!(f, "rate limit exceeded for {service}, retry after {secs}s")
                } else {
                    write!(f, "rate limit exceeded for {service}")
                }
            }
            Self::InvalidCredentials { service } => {
                write!(f, "invalid credentials for {service}")
            }
            Self::OperationNotSupported { service, operation } => {
                write!(f, "operation {operation} not supported by {service}")
            }
            Self::ProtocolError { service, reason } => {
                write!(f, "protocol error with {service}: {reason}")
            }
        }
    }
}

impl std::error::Error for IntegrationError {}

/// Errors related to AI primitive operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AiError {
    /// LLM provider is unavailable.
    ProviderUnavailable { provider: String, reason: String },
    /// LLM call failed.
    LlmCallFailed { reason: String },
    /// Output parsing failed.
    OutputParsingFailed { reason: String },
    /// Schema constraint violation.
    SchemaConstraintViolation { expected: String, actual: String },
    /// Coordinate loop exceeded maximum iterations.
    MaxIterationsExceeded { max: u32, goal: String },
    /// Prompt template not found.
    PromptNotFound { name: String },
    /// Invalid prompt template.
    InvalidPromptTemplate { name: String, reason: String },
}

impl fmt::Display for AiError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ProviderUnavailable { provider, reason } => {
                write!(f, "LLM provider {provider} unavailable: {reason}")
            }
            Self::LlmCallFailed { reason } => write!(f, "LLM call failed: {reason}"),
            Self::OutputParsingFailed { reason } => write!(f, "output parsing failed: {reason}"),
            Self::SchemaConstraintViolation { expected, actual } => {
                write!(f, "schema constraint violation: expected {expected}, got {actual}")
            }
            Self::MaxIterationsExceeded { max, goal } => {
                write!(f, "coordinate loop exceeded {max} iterations for goal: {goal}")
            }
            Self::PromptNotFound { name } => write!(f, "prompt template not found: {name}"),
            Self::InvalidPromptTemplate { name, reason } => {
                write!(f, "invalid prompt template {name}: {reason}")
            }
        }
    }
}

impl std::error::Error for AiError {}

/// Errors related to scheduler operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SchedulerError {
    /// Invalid cron expression.
    InvalidCronExpression { expression: String, reason: String },
    /// Trigger with the given ID was not found.
    TriggerNotFound { id: String },
    /// Failed to register trigger.
    TriggerRegistrationFailed { reason: String },
    /// Trigger already exists.
    TriggerAlreadyExists { id: String },
}

impl fmt::Display for SchedulerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCronExpression { expression, reason } => {
                write!(f, "invalid cron expression '{expression}': {reason}")
            }
            Self::TriggerNotFound { id } => write!(f, "trigger not found: {id}"),
            Self::TriggerRegistrationFailed { reason } => {
                write!(f, "trigger registration failed: {reason}")
            }
            Self::TriggerAlreadyExists { id } => write!(f, "trigger already exists: {id}"),
        }
    }
}

impl std::error::Error for SchedulerError {}

/// Errors related to credential vault operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    /// Credential with the given ID was not found.
    NotFound { id: String },
    /// Encryption failed.
    EncryptionFailed { reason: String },
    /// Decryption failed.
    DecryptionFailed { reason: String },
    /// Invalid credential format.
    InvalidFormat { reason: String },
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "credential not found: {id}"),
            Self::EncryptionFailed { reason } => write!(f, "encryption failed: {reason}"),
            Self::DecryptionFailed { reason } => write!(f, "decryption failed: {reason}"),
            Self::InvalidFormat { reason } => write!(f, "invalid credential format: {reason}"),
        }
    }
}

impl std::error::Error for CredentialError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_type_works() {
        let ok: Result<i32> = Ok(42);
        assert_eq!(ok.expect("should be ok"), 42);
    }

    #[test]
    fn workflow_error_display() {
        let err = WorkflowError::NotFound {
            id: "wf-123".to_string(),
        };
        assert_eq!(err.to_string(), "workflow not found: wf-123");
    }

    #[test]
    fn conversation_error_display() {
        let err = ConversationError::SessionNotFound {
            id: "sess-456".to_string(),
        };
        assert_eq!(err.to_string(), "session not found: sess-456");
    }

    #[test]
    fn integration_error_display() {
        let err = IntegrationError::RateLimitExceeded {
            service: "gmail".to_string(),
            retry_after_secs: Some(60),
        };
        assert_eq!(
            err.to_string(),
            "rate limit exceeded for gmail, retry after 60s"
        );
    }

    #[test]
    fn ai_error_display() {
        let err = AiError::MaxIterationsExceeded {
            max: 10,
            goal: "plan trip".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "coordinate loop exceeded 10 iterations for goal: plan trip"
        );
    }
}
