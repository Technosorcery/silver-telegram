//! Session management for authenticated users.
//!
//! Sessions represent an authenticated user's active connection to the platform.
//! They are created after successful OIDC authentication and are used to track
//! the user's identity and roles throughout their session.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::UserId;

use crate::role::RoleSet;

/// Unique identifier for a session.
///
/// Session IDs are opaque strings generated during session creation.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(String);

impl SessionId {
    /// Creates a new session ID from a string.
    #[must_use]
    pub fn new(id: String) -> Self {
        Self(id)
    }

    /// Returns the session ID as a string slice.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl std::fmt::Display for SessionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<String> for SessionId {
    fn from(s: String) -> Self {
        Self(s)
    }
}

impl From<&str> for SessionId {
    fn from(s: &str) -> Self {
        Self(s.to_string())
    }
}

/// Represents an active authenticated session.
///
/// A session is created after successful OIDC authentication and contains
/// the user's identity and derived roles. Sessions have an expiration time
/// and can be explicitly invalidated.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Session {
    /// Unique identifier for this session.
    id: SessionId,
    /// The authenticated user's ID.
    user_id: UserId,
    /// Roles derived from OIDC groups at authentication time.
    roles: RoleSet,
    /// When the session was created.
    created_at: DateTime<Utc>,
    /// When the session expires.
    expires_at: DateTime<Utc>,
    /// OIDC access token (for API calls that need it).
    access_token: Option<String>,
    /// OIDC refresh token (for token refresh).
    refresh_token: Option<String>,
}

impl Session {
    /// Creates a new session for the given user.
    ///
    /// The session is valid for the specified duration.
    #[must_use]
    pub fn new(id: SessionId, user_id: UserId, roles: RoleSet, duration: Duration) -> Self {
        let now = Utc::now();
        Self {
            id,
            user_id,
            roles,
            created_at: now,
            expires_at: now + duration,
            access_token: None,
            refresh_token: None,
        }
    }

    /// Creates a session with OIDC tokens.
    #[must_use]
    pub fn with_tokens(
        id: SessionId,
        user_id: UserId,
        roles: RoleSet,
        duration: Duration,
        access_token: String,
        refresh_token: Option<String>,
    ) -> Self {
        let mut session = Self::new(id, user_id, roles, duration);
        session.access_token = Some(access_token);
        session.refresh_token = refresh_token;
        session
    }

    /// Returns the session ID.
    #[must_use]
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// Returns the authenticated user's ID.
    #[must_use]
    pub fn user_id(&self) -> UserId {
        self.user_id
    }

    /// Returns the user's roles.
    #[must_use]
    pub fn roles(&self) -> &RoleSet {
        &self.roles
    }

    /// Returns when the session was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns when the session expires.
    #[must_use]
    pub fn expires_at(&self) -> DateTime<Utc> {
        self.expires_at
    }

    /// Returns the OIDC access token, if present.
    #[must_use]
    pub fn access_token(&self) -> Option<&str> {
        self.access_token.as_deref()
    }

    /// Returns the OIDC refresh token, if present.
    #[must_use]
    pub fn refresh_token(&self) -> Option<&str> {
        self.refresh_token.as_deref()
    }

    /// Returns true if the session has expired.
    #[must_use]
    pub fn is_expired(&self) -> bool {
        Utc::now() >= self.expires_at
    }

    /// Returns true if the session is still valid (not expired).
    #[must_use]
    pub fn is_valid(&self) -> bool {
        !self.is_expired()
    }

    /// Returns true if the user has platform access.
    #[must_use]
    pub fn has_access(&self) -> bool {
        self.roles.has_access()
    }

    /// Returns true if the user has admin access.
    #[must_use]
    pub fn is_admin(&self) -> bool {
        self.roles.is_admin()
    }

    /// Updates the session tokens and extends expiration.
    pub fn refresh(
        &mut self,
        access_token: String,
        refresh_token: Option<String>,
        duration: Duration,
    ) {
        self.access_token = Some(access_token);
        self.refresh_token = refresh_token;
        self.expires_at = Utc::now() + duration;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_session_id() -> SessionId {
        SessionId::new("sess_test_123".to_string())
    }

    #[test]
    fn session_id_display() {
        let id = test_session_id();
        assert_eq!(id.to_string(), "sess_test_123");
    }

    #[test]
    fn session_id_from_string() {
        let id: SessionId = "test_session".to_string().into();
        assert_eq!(id.as_str(), "test_session");
    }

    #[test]
    fn session_id_from_str() {
        let id: SessionId = "test_session".into();
        assert_eq!(id.as_str(), "test_session");
    }

    #[test]
    fn new_session_has_correct_fields() {
        let session_id = test_session_id();
        let user_id = UserId::new();
        let roles = RoleSet::user();
        let duration = Duration::hours(1);

        let before = Utc::now();
        let session = Session::new(session_id.clone(), user_id, roles.clone(), duration);
        let after = Utc::now();

        assert_eq!(session.id(), &session_id);
        assert_eq!(session.user_id(), user_id);
        assert_eq!(session.roles(), &roles);
        assert!(session.created_at() >= before);
        assert!(session.created_at() <= after);
        assert!(session.expires_at() > session.created_at());
        assert!(session.access_token().is_none());
        assert!(session.refresh_token().is_none());
    }

    #[test]
    fn session_with_tokens() {
        let session = Session::with_tokens(
            test_session_id(),
            UserId::new(),
            RoleSet::user(),
            Duration::hours(1),
            "access_token_123".to_string(),
            Some("refresh_token_456".to_string()),
        );

        assert_eq!(session.access_token(), Some("access_token_123"));
        assert_eq!(session.refresh_token(), Some("refresh_token_456"));
    }

    #[test]
    fn session_expiration() {
        // Create a session that expires immediately
        let session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::user(),
            Duration::seconds(-1), // Already expired
        );

        assert!(session.is_expired());
        assert!(!session.is_valid());
    }

    #[test]
    fn session_not_expired() {
        let session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::user(),
            Duration::hours(1),
        );

        assert!(!session.is_expired());
        assert!(session.is_valid());
    }

    #[test]
    fn session_access_from_roles() {
        let user_session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::user(),
            Duration::hours(1),
        );
        assert!(user_session.has_access());
        assert!(!user_session.is_admin());

        let admin_session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::admin(),
            Duration::hours(1),
        );
        assert!(admin_session.has_access());
        assert!(admin_session.is_admin());

        let no_access_session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::none(),
            Duration::hours(1),
        );
        assert!(!no_access_session.has_access());
        assert!(!no_access_session.is_admin());
    }

    #[test]
    fn session_refresh() {
        let mut session = Session::new(
            test_session_id(),
            UserId::new(),
            RoleSet::user(),
            Duration::seconds(1),
        );

        let old_expires = session.expires_at();

        // Wait briefly and refresh
        std::thread::sleep(std::time::Duration::from_millis(10));
        session.refresh(
            "new_access_token".to_string(),
            Some("new_refresh_token".to_string()),
            Duration::hours(2),
        );

        assert_eq!(session.access_token(), Some("new_access_token"));
        assert_eq!(session.refresh_token(), Some("new_refresh_token"));
        assert!(session.expires_at() > old_expires);
    }

    #[test]
    fn session_serialization_roundtrip() {
        let session = Session::with_tokens(
            test_session_id(),
            UserId::new(),
            RoleSet::admin(),
            Duration::hours(1),
            "token".to_string(),
            None,
        );

        let json = serde_json::to_string(&session).expect("serialize");
        let parsed: Session = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(session.id(), parsed.id());
        assert_eq!(session.user_id(), parsed.user_id());
        assert_eq!(session.roles(), parsed.roles());
    }
}
