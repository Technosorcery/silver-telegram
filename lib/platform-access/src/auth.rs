//! Authentication middleware and extractors for Axum.
//!
//! This module provides authentication primitives for the web layer:
//! - `AuthenticatedUser`: Extractor for requiring authenticated users
//! - `OptionalUser`: Extractor for optional authentication
//! - `RequireAdmin`: Extractor for requiring admin access

use crate::error::AuthenticationError;
use crate::role::RoleSet;
use crate::session::{Session, SessionId};
use crate::user::User;
use silver_telegram_core::UserId;

/// Represents an authenticated user context extracted from the request.
///
/// This is available in handlers after successful authentication.
/// It contains the session information and can be used to:
/// - Get the current user's ID
/// - Check roles/permissions
/// - Access session metadata
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// The current session.
    session: Session,
    /// The user record (may be fetched lazily in real implementation).
    user: User,
}

impl AuthenticatedUser {
    /// Creates a new authenticated user context.
    #[must_use]
    pub fn new(session: Session, user: User) -> Self {
        Self { session, user }
    }

    /// Returns the authenticated user's ID.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.session.user_id()
    }

    /// Returns the current session.
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Returns the user record.
    #[must_use]
    pub fn user(&self) -> &User {
        &self.user
    }

    /// Returns the user's roles.
    #[must_use]
    pub fn roles(&self) -> &RoleSet {
        self.session.roles()
    }

    /// Returns true if the user has admin access.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.session.is_admin()
    }
}

/// Claims extracted from an OIDC ID token.
///
/// These are used to create/update user records and determine roles.
#[derive(Debug, Clone)]
pub struct OidcClaims {
    /// The subject claim (unique user identifier from the provider).
    pub subject: String,
    /// The issuer URL.
    pub issuer: String,
    /// Email address (optional).
    pub email: Option<String>,
    /// Display name (optional, from name or preferred_username).
    pub display_name: Option<String>,
    /// Group memberships (from the configured groups claim).
    pub groups: Vec<String>,
}

impl OidcClaims {
    /// Creates a new set of OIDC claims.
    #[must_use]
    pub fn new(subject: String, issuer: String) -> Self {
        Self {
            subject,
            issuer,
            email: None,
            display_name: None,
            groups: Vec::new(),
        }
    }

    /// Sets the email claim.
    #[must_use]
    pub fn with_email(mut self, email: Option<String>) -> Self {
        self.email = email;
        self
    }

    /// Sets the display name.
    #[must_use]
    pub fn with_display_name(mut self, name: Option<String>) -> Self {
        self.display_name = name;
        self
    }

    /// Sets the groups.
    #[must_use]
    pub fn with_groups(mut self, groups: Vec<String>) -> Self {
        self.groups = groups;
        self
    }
}

/// Result of an authentication attempt.
#[derive(Debug)]
pub enum AuthResult {
    /// User is authenticated with a valid session.
    Authenticated(Box<AuthenticatedUser>),
    /// No session found - user needs to log in.
    Unauthenticated,
    /// Session found but expired.
    SessionExpired { session_id: SessionId },
    /// User authenticated but lacks platform access (no user group).
    AccessDenied { user_id: UserId },
}

/// Result of an OIDC authentication callback.
#[derive(Debug)]
pub enum CallbackResult {
    /// Authentication successful - user created/updated and session established.
    Success {
        /// The authenticated user.
        user: Box<User>,
        /// The new session.
        session: Box<Session>,
        /// Whether this is a new user (first login).
        is_new_user: bool,
    },
    /// User lacks required group membership.
    AccessDenied {
        /// The user's subject claim.
        subject: String,
    },
    /// Authentication failed.
    Failed(AuthenticationError),
}

/// Login initiation data for redirecting to the OIDC provider.
#[derive(Debug, Clone)]
pub struct LoginInitiation {
    /// The URL to redirect the user to for authentication.
    pub authorization_url: String,
    /// State parameter for CSRF protection (store in session/cookie).
    pub state: String,
    /// PKCE code verifier (store securely for the callback).
    pub pkce_verifier: String,
    /// Nonce for ID token validation (store for the callback).
    pub nonce: String,
}

/// Data needed to process an OIDC callback.
#[derive(Debug, Clone)]
pub struct CallbackData {
    /// The authorization code from the provider.
    pub code: String,
    /// The state parameter (must match the one from login initiation).
    pub state: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::role::RoleSet;
    use chrono::Duration;

    #[test]
    fn authenticated_user_has_user_info() {
        let user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        let session = Session::new(
            SessionId::new("sess_abc".to_string()),
            user.id(),
            RoleSet::user(),
            Duration::hours(1),
        );

        let auth_user = AuthenticatedUser::new(session.clone(), user.clone());

        assert_eq!(auth_user.user_id(), user.id());
        assert_eq!(auth_user.user().subject(), "sub_123");
        assert!(!auth_user.is_admin());
    }

    #[test]
    fn authenticated_user_with_admin() {
        let user = User::new(
            "sub_admin".to_string(),
            "https://auth.example.com".to_string(),
        );
        let session = Session::new(
            SessionId::new("sess_admin".to_string()),
            user.id(),
            RoleSet::admin(),
            Duration::hours(1),
        );

        let auth_user = AuthenticatedUser::new(session, user);

        assert!(auth_user.is_admin());
    }

    #[test]
    fn oidc_claims_builder() {
        let claims = OidcClaims::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        )
        .with_email(Some("user@example.com".to_string()))
        .with_display_name(Some("Test User".to_string()))
        .with_groups(vec!["platform-users".to_string()]);

        assert_eq!(claims.subject, "sub_123");
        assert_eq!(claims.issuer, "https://auth.example.com");
        assert_eq!(claims.email, Some("user@example.com".to_string()));
        assert_eq!(claims.display_name, Some("Test User".to_string()));
        assert_eq!(claims.groups, vec!["platform-users"]);
    }
}
