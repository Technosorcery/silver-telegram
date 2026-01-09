//! Domain error types for server operations.
//!
//! This module provides typed error variants for server-side operations,
//! following the rootcause pattern described in CLAUDE.md.

use leptos::server_fn::error::ServerFnError;
use std::fmt;

/// Session-related errors.
#[derive(Debug)]
pub enum SessionError {
    /// User is not authenticated (no session cookie).
    NotAuthenticated,
    /// Session was not found in database.
    NotFound { session_id: String },
    /// Session has expired.
    Expired { session_id: String },
    /// Admin access is required for this operation.
    AdminRequired,
    /// Database error while accessing session.
    DatabaseError { details: String },
}

impl fmt::Display for SessionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAuthenticated => write!(f, "not authenticated"),
            Self::NotFound { session_id } => {
                write!(f, "session '{}' not found", session_id)
            }
            Self::Expired { session_id } => {
                write!(f, "session '{}' has expired", session_id)
            }
            Self::AdminRequired => write!(f, "admin access required"),
            Self::DatabaseError { details } => {
                write!(f, "session database error: {}", details)
            }
        }
    }
}

impl SessionError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            SessionError::NotAuthenticated => ServerFnError::new("Not authenticated"),
            SessionError::NotFound { .. } => ServerFnError::new("Session not found"),
            SessionError::Expired { .. } => ServerFnError::new("Session expired"),
            SessionError::AdminRequired => ServerFnError::new("Admin access required"),
            SessionError::DatabaseError { .. } => ServerFnError::new("Database error"),
        }
    }
}

/// Workflow-related errors.
#[derive(Debug)]
pub enum WorkflowError {
    /// Workflow was not found.
    NotFound { id: String },
    /// Invalid workflow ID format.
    InvalidId { id: String, reason: String },
    /// Access to workflow was denied.
    AccessDenied { id: String },
    /// Workflow is in an invalid state for the operation.
    InvalidState {
        id: String,
        state: String,
        required: String,
    },
    /// Database error while accessing workflow.
    DatabaseError { details: String },
    /// Authorization check failed.
    AuthorizationError { details: String },
    /// Failed to parse workflow graph.
    InvalidGraph { details: String },
}

impl fmt::Display for WorkflowError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "workflow '{}' not found", id),
            Self::InvalidId { id, reason } => {
                write!(f, "invalid workflow id '{}': {}", id, reason)
            }
            Self::AccessDenied { id } => {
                write!(f, "access denied to workflow '{}'", id)
            }
            Self::InvalidState {
                id,
                state,
                required,
            } => {
                write!(
                    f,
                    "workflow '{}' is in state '{}', requires '{}'",
                    id, state, required
                )
            }
            Self::DatabaseError { details } => {
                write!(f, "workflow database error: {}", details)
            }
            Self::AuthorizationError { details } => {
                write!(f, "workflow authorization error: {}", details)
            }
            Self::InvalidGraph { details } => {
                write!(f, "invalid workflow graph: {}", details)
            }
        }
    }
}

impl WorkflowError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            WorkflowError::NotFound { .. } => ServerFnError::new("Workflow not found"),
            WorkflowError::InvalidId { .. } => ServerFnError::new("Invalid workflow ID"),
            WorkflowError::AccessDenied { .. } => ServerFnError::new("Access denied"),
            WorkflowError::InvalidState { .. } => {
                ServerFnError::new("Workflow is not in the required state")
            }
            WorkflowError::DatabaseError { .. } => ServerFnError::new("Database error"),
            WorkflowError::AuthorizationError { .. } => ServerFnError::new("Authorization error"),
            WorkflowError::InvalidGraph { .. } => ServerFnError::new("Invalid workflow graph"),
        }
    }
}

/// Integration-related errors.
#[derive(Debug)]
pub enum IntegrationError {
    /// Integration was not found.
    NotFound { id: String },
    /// Invalid integration ID format.
    InvalidId { id: String, reason: String },
    /// Access to integration was denied.
    AccessDenied { id: String },
    /// Invalid configuration data.
    InvalidConfig { details: String },
    /// Database error while accessing integration.
    DatabaseError { details: String },
    /// Authorization check failed.
    AuthorizationError { details: String },
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "integration '{}' not found", id),
            Self::InvalidId { id, reason } => {
                write!(f, "invalid integration id '{}': {}", id, reason)
            }
            Self::AccessDenied { id } => {
                write!(f, "access denied to integration '{}'", id)
            }
            Self::InvalidConfig { details } => {
                write!(f, "invalid integration config: {}", details)
            }
            Self::DatabaseError { details } => {
                write!(f, "integration database error: {}", details)
            }
            Self::AuthorizationError { details } => {
                write!(f, "integration authorization error: {}", details)
            }
        }
    }
}

impl IntegrationError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            IntegrationError::NotFound { .. } => ServerFnError::new("Integration not found"),
            IntegrationError::InvalidId { .. } => ServerFnError::new("Invalid integration ID"),
            IntegrationError::AccessDenied { .. } => ServerFnError::new("Access denied"),
            IntegrationError::InvalidConfig { .. } => ServerFnError::new("Invalid configuration"),
            IntegrationError::DatabaseError { .. } => ServerFnError::new("Database error"),
            IntegrationError::AuthorizationError { .. } => {
                ServerFnError::new("Authorization error")
            }
        }
    }
}

/// Workflow run-related errors.
#[derive(Debug)]
pub enum WorkflowRunError {
    /// Run was not found.
    NotFound { id: String },
    /// Invalid run ID format.
    InvalidId { id: String, reason: String },
    /// Database error while accessing run.
    DatabaseError { details: String },
}

impl fmt::Display for WorkflowRunError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "workflow run '{}' not found", id),
            Self::InvalidId { id, reason } => {
                write!(f, "invalid run id '{}': {}", id, reason)
            }
            Self::DatabaseError { details } => {
                write!(f, "workflow run database error: {}", details)
            }
        }
    }
}

impl WorkflowRunError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            WorkflowRunError::NotFound { .. } => ServerFnError::new("Run not found"),
            WorkflowRunError::InvalidId { .. } => ServerFnError::new("Invalid run ID"),
            WorkflowRunError::DatabaseError { .. } => ServerFnError::new("Database error"),
        }
    }
}

/// User-related errors.
#[derive(Debug)]
pub enum UserError {
    /// User was not found.
    NotFound { id: String },
    /// Database error while accessing user.
    DatabaseError { details: String },
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => write!(f, "user '{}' not found", id),
            Self::DatabaseError { details } => {
                write!(f, "user database error: {}", details)
            }
        }
    }
}

impl UserError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            UserError::NotFound { .. } => ServerFnError::new("User not found"),
            UserError::DatabaseError { .. } => ServerFnError::new("Database error"),
        }
    }
}

/// Model discovery-related errors.
#[derive(Debug)]
pub enum ModelDiscoveryError {
    /// Integration was not found.
    IntegrationNotFound { id: String },
    /// Integration is not an OpenAI-compatible type.
    InvalidIntegrationType { id: String, actual_type: String },
    /// Failed to connect to the endpoint.
    ConnectionFailed { endpoint: String, reason: String },
    /// Failed to parse the model list response.
    ParseError { reason: String },
    /// Request timed out.
    Timeout { endpoint: String },
    /// Database error while fetching integration config.
    DatabaseError { details: String },
    /// Access denied to integration.
    AccessDenied { id: String },
}

impl fmt::Display for ModelDiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IntegrationNotFound { id } => {
                write!(f, "integration '{}' not found", id)
            }
            Self::InvalidIntegrationType { id, actual_type } => {
                write!(
                    f,
                    "integration '{}' is type '{}', expected 'openai_compatible'",
                    id, actual_type
                )
            }
            Self::ConnectionFailed { endpoint, reason } => {
                write!(f, "failed to connect to '{}': {}", endpoint, reason)
            }
            Self::ParseError { reason } => {
                write!(f, "failed to parse model list: {}", reason)
            }
            Self::Timeout { endpoint } => {
                write!(f, "request to '{}' timed out", endpoint)
            }
            Self::DatabaseError { details } => {
                write!(f, "database error: {}", details)
            }
            Self::AccessDenied { id } => {
                write!(f, "access denied to integration '{}'", id)
            }
        }
    }
}

impl ModelDiscoveryError {
    /// Convert to a user-safe ServerFnError.
    pub fn into_server_error(self) -> ServerFnError {
        match &self {
            ModelDiscoveryError::IntegrationNotFound { .. } => {
                ServerFnError::new("Integration not found")
            }
            ModelDiscoveryError::InvalidIntegrationType { .. } => {
                ServerFnError::new("Integration is not an OpenAI-compatible provider")
            }
            ModelDiscoveryError::ConnectionFailed { reason, .. } => {
                ServerFnError::new(format!("Connection failed: {}", reason))
            }
            ModelDiscoveryError::ParseError { reason } => {
                ServerFnError::new(format!("Failed to parse models: {}", reason))
            }
            ModelDiscoveryError::Timeout { .. } => ServerFnError::new("Request timed out"),
            ModelDiscoveryError::DatabaseError { .. } => ServerFnError::new("Database error"),
            ModelDiscoveryError::AccessDenied { .. } => ServerFnError::new("Access denied"),
        }
    }
}
