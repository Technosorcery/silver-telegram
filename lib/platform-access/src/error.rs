//! Error types for the platform-access crate.
//!
//! Errors are designed for layered context using rootcause:
//! - `AuthenticationError`: Authentication failures (OIDC, session)
//! - `AuthorizationError`: Authorization failures (permission checks)

use silver_telegram_core::UserId;
use std::fmt;

/// Errors from authentication operations.
///
/// These errors represent failures in verifying user identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthenticationError {
    /// OIDC token validation failed.
    InvalidToken { reason: String },
    /// OIDC token has expired.
    TokenExpired,
    /// Session not found or invalid.
    InvalidSession { session_id: String },
    /// Session has expired.
    SessionExpired { session_id: String },
    /// OIDC provider error.
    ProviderError { provider: String, reason: String },
    /// Missing required claim in token.
    MissingClaim { claim: String },
    /// User not found after authentication.
    UserNotFound { subject: String },
}

impl fmt::Display for AuthenticationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken { reason } => {
                write!(f, "invalid token: {reason}")
            }
            Self::TokenExpired => {
                write!(f, "token has expired")
            }
            Self::InvalidSession { session_id } => {
                write!(f, "invalid session: {session_id}")
            }
            Self::SessionExpired { session_id } => {
                write!(f, "session has expired: {session_id}")
            }
            Self::ProviderError { provider, reason } => {
                write!(f, "OIDC provider '{provider}' error: {reason}")
            }
            Self::MissingClaim { claim } => {
                write!(f, "missing required claim: {claim}")
            }
            Self::UserNotFound { subject } => {
                write!(f, "user not found for subject: {subject}")
            }
        }
    }
}

impl std::error::Error for AuthenticationError {}

/// Errors from authorization operations.
///
/// These errors represent failures in permission checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthorizationError {
    /// User is not authenticated.
    NotAuthenticated,
    /// User lacks required permission.
    PermissionDenied {
        user_id: UserId,
        action: String,
        resource: String,
    },
    /// Authorization check failed due to system error.
    CheckFailed { reason: String },
}

impl fmt::Display for AuthorizationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotAuthenticated => {
                write!(f, "user is not authenticated")
            }
            Self::PermissionDenied {
                user_id,
                action,
                resource,
            } => {
                write!(
                    f,
                    "user {user_id} lacks permission to {action} on {resource}"
                )
            }
            Self::CheckFailed { reason } => {
                write!(f, "authorization check failed: {reason}")
            }
        }
    }
}

impl std::error::Error for AuthorizationError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authentication_error_invalid_token_display() {
        let err = AuthenticationError::InvalidToken {
            reason: "signature mismatch".to_string(),
        };
        assert!(err.to_string().contains("invalid token"));
        assert!(err.to_string().contains("signature mismatch"));
    }

    #[test]
    fn authentication_error_token_expired_display() {
        let err = AuthenticationError::TokenExpired;
        assert!(err.to_string().contains("expired"));
    }

    #[test]
    fn authentication_error_invalid_session_display() {
        let err = AuthenticationError::InvalidSession {
            session_id: "sess_123".to_string(),
        };
        assert!(err.to_string().contains("invalid session"));
        assert!(err.to_string().contains("sess_123"));
    }

    #[test]
    fn authentication_error_provider_error_display() {
        let err = AuthenticationError::ProviderError {
            provider: "keycloak".to_string(),
            reason: "connection timeout".to_string(),
        };
        assert!(err.to_string().contains("keycloak"));
        assert!(err.to_string().contains("connection timeout"));
    }

    #[test]
    fn authorization_error_not_authenticated_display() {
        let err = AuthorizationError::NotAuthenticated;
        assert!(err.to_string().contains("not authenticated"));
    }

    #[test]
    fn authorization_error_permission_denied_display() {
        let err = AuthorizationError::PermissionDenied {
            user_id: UserId::new(),
            action: "read".to_string(),
            resource: "workflow:123".to_string(),
        };
        assert!(err.to_string().contains("lacks permission"));
        assert!(err.to_string().contains("read"));
        assert!(err.to_string().contains("workflow:123"));
    }
}
