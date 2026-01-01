//! Authorization error types.

use std::fmt;

/// Authorization errors.
#[derive(Debug)]
pub enum AuthzError {
    /// Permission denied.
    PermissionDenied {
        /// The resource that was accessed.
        resource: String,
        /// The permission that was requested.
        permission: String,
    },
    /// Failed to connect to SpiceDB.
    ConnectionFailed {
        /// Error details.
        details: String,
    },
    /// SpiceDB request failed.
    RequestFailed {
        /// Error details.
        details: String,
    },
    /// Invalid resource or subject.
    InvalidInput {
        /// Error details.
        details: String,
    },
}

impl fmt::Display for AuthzError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PermissionDenied {
                resource,
                permission,
            } => {
                write!(
                    f,
                    "permission '{}' denied on resource '{}'",
                    permission, resource
                )
            }
            Self::ConnectionFailed { details } => {
                write!(f, "failed to connect to authorization service: {}", details)
            }
            Self::RequestFailed { details } => {
                write!(f, "authorization request failed: {}", details)
            }
            Self::InvalidInput { details } => {
                write!(f, "invalid authorization input: {}", details)
            }
        }
    }
}

impl std::error::Error for AuthzError {}
