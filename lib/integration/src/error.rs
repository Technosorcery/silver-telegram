//! Error types for the integration crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `ConnectorError`: Errors from connector operations
//! - `CredentialError`: Errors from credential storage/retrieval
//! - `IntegrationError`: High-level wrapper for context

use silver_telegram_core::{CredentialId, IntegrationAccountId};
use std::fmt;

/// Errors from connector operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectorError {
    /// Connection to service failed.
    ConnectionFailed { reason: String },
    /// Authentication failed.
    AuthenticationFailed { reason: String },
    /// Rate limit exceeded.
    RateLimited { retry_after_secs: Option<u64> },
    /// Operation not supported.
    OperationNotSupported { operation: String },
    /// Invalid operation parameters.
    InvalidParameters { operation: String, reason: String },
    /// Protocol error.
    ProtocolError { reason: String },
    /// Timeout waiting for response.
    Timeout,
}

impl fmt::Display for ConnectorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionFailed { reason } => {
                write!(f, "connection failed: {reason}")
            }
            Self::AuthenticationFailed { reason } => {
                write!(f, "authentication failed: {reason}")
            }
            Self::RateLimited { retry_after_secs } => {
                if let Some(secs) = retry_after_secs {
                    write!(f, "rate limited, retry after {secs}s")
                } else {
                    write!(f, "rate limited")
                }
            }
            Self::OperationNotSupported { operation } => {
                write!(f, "operation not supported: {operation}")
            }
            Self::InvalidParameters { operation, reason } => {
                write!(f, "invalid parameters for '{operation}': {reason}")
            }
            Self::ProtocolError { reason } => {
                write!(f, "protocol error: {reason}")
            }
            Self::Timeout => write!(f, "operation timed out"),
        }
    }
}

impl std::error::Error for ConnectorError {}

/// Errors from credential operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CredentialError {
    /// Credential not found.
    NotFound { id: CredentialId },
    /// Encryption failed.
    EncryptionFailed { reason: String },
    /// Decryption failed.
    DecryptionFailed { reason: String },
    /// Invalid credential format.
    InvalidFormat { reason: String },
    /// Storage operation failed.
    StorageFailed { reason: String },
}

impl fmt::Display for CredentialError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound { id } => {
                write!(f, "credential not found: {id}")
            }
            Self::EncryptionFailed { reason } => {
                write!(f, "encryption failed: {reason}")
            }
            Self::DecryptionFailed { reason } => {
                write!(f, "decryption failed: {reason}")
            }
            Self::InvalidFormat { reason } => {
                write!(f, "invalid credential format: {reason}")
            }
            Self::StorageFailed { reason } => {
                write!(f, "storage operation failed: {reason}")
            }
        }
    }
}

impl std::error::Error for CredentialError {}

/// High-level integration errors.
///
/// Use these to add context when wrapping lower-level errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntegrationError {
    /// Integration account not found.
    AccountNotFound { id: IntegrationAccountId },
    /// Connector operation context (use as context wrapper).
    ConnectorOperation {
        connector_id: String,
        operation: String,
    },
    /// Credential operation context (use as context wrapper).
    CredentialOperation { credential_id: CredentialId },
}

impl fmt::Display for IntegrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AccountNotFound { id } => {
                write!(f, "integration account not found: {id}")
            }
            Self::ConnectorOperation {
                connector_id,
                operation,
            } => {
                write!(
                    f,
                    "connector '{connector_id}' operation '{operation}' failed"
                )
            }
            Self::CredentialOperation { credential_id } => {
                write!(f, "credential operation failed for {credential_id}")
            }
        }
    }
}

impl std::error::Error for IntegrationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn connector_error_display() {
        let err = ConnectorError::ConnectionFailed {
            reason: "host unreachable".to_string(),
        };
        assert!(err.to_string().contains("connection failed"));
        assert!(err.to_string().contains("host unreachable"));
    }

    #[test]
    fn connector_error_rate_limited() {
        let err = ConnectorError::RateLimited {
            retry_after_secs: Some(60),
        };
        assert!(err.to_string().contains("60s"));
    }

    #[test]
    fn credential_error_display() {
        let id = CredentialId::new();
        let err = CredentialError::NotFound { id };
        assert!(err.to_string().contains("credential not found"));
    }

    #[test]
    fn integration_error_display() {
        let err = IntegrationError::ConnectorOperation {
            connector_id: "gmail".to_string(),
            operation: "fetch_emails".to_string(),
        };
        assert!(err.to_string().contains("gmail"));
        assert!(err.to_string().contains("fetch_emails"));
    }
}
