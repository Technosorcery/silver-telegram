//! User domain type and related structures.
//!
//! The User represents an authenticated user of the platform.
//! Users are identified by their OIDC subject claim and have a
//! corresponding internal UserId.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::UserId;

/// Represents an authenticated user of the platform.
///
/// Users are created after successful OIDC authentication and are
/// identified by their OIDC subject claim. The internal `id` is used
/// for all platform operations and authorization checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct User {
    /// Internal platform user ID.
    id: UserId,
    /// OIDC subject claim - unique identifier from the identity provider.
    subject: String,
    /// OIDC issuer URL - identifies which identity provider authenticated the user.
    issuer: String,
    /// User's email address (from OIDC email claim, if available).
    email: Option<String>,
    /// User's display name (from OIDC name or preferred_username claim).
    display_name: Option<String>,
    /// User's configured timezone (IANA timezone name, e.g., "America/New_York").
    /// Used for scheduling workflows at user-expected times.
    timezone: Option<String>,
    /// When the user record was created.
    created_at: DateTime<Utc>,
    /// When the user record was last updated.
    updated_at: DateTime<Utc>,
}

impl User {
    /// Creates a new user with the given OIDC claims.
    ///
    /// The user ID is generated automatically. Use this when creating
    /// a new user after their first authentication.
    #[must_use]
    pub fn new(subject: String, issuer: String) -> Self {
        let now = Utc::now();
        Self {
            id: UserId::new(),
            subject,
            issuer,
            email: None,
            display_name: None,
            timezone: None,
            created_at: now,
            updated_at: now,
        }
    }

    /// Creates a user with all fields specified.
    ///
    /// Use this when reconstituting a user from storage.
    #[must_use]
    #[expect(clippy::too_many_arguments)]
    pub fn with_all_fields(
        id: UserId,
        subject: String,
        issuer: String,
        email: Option<String>,
        display_name: Option<String>,
        timezone: Option<String>,
        created_at: DateTime<Utc>,
        updated_at: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            subject,
            issuer,
            email,
            display_name,
            timezone,
            created_at,
            updated_at,
        }
    }

    /// Returns the user's internal platform ID.
    #[must_use]
    pub fn id(&self) -> UserId {
        self.id
    }

    /// Returns the OIDC subject claim.
    #[must_use]
    pub fn subject(&self) -> &str {
        &self.subject
    }

    /// Returns the OIDC issuer URL.
    #[must_use]
    pub fn issuer(&self) -> &str {
        &self.issuer
    }

    /// Returns the user's email address, if available.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// Returns the user's display name, if available.
    #[must_use]
    pub fn display_name(&self) -> Option<&str> {
        self.display_name.as_deref()
    }

    /// Returns the user's configured timezone, if set.
    ///
    /// Returns an IANA timezone name (e.g., "America/New_York").
    #[must_use]
    pub fn timezone(&self) -> Option<&str> {
        self.timezone.as_deref()
    }

    /// Returns when the user was created.
    #[must_use]
    pub fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    /// Returns when the user was last updated.
    #[must_use]
    pub fn updated_at(&self) -> DateTime<Utc> {
        self.updated_at
    }

    /// Sets the user's email address.
    pub fn set_email(&mut self, email: Option<String>) {
        self.email = email;
        self.updated_at = Utc::now();
    }

    /// Sets the user's display name.
    pub fn set_display_name(&mut self, display_name: Option<String>) {
        self.display_name = display_name;
        self.updated_at = Utc::now();
    }

    /// Sets the user's timezone.
    ///
    /// Should be an IANA timezone name (e.g., "America/New_York", "Europe/London").
    pub fn set_timezone(&mut self, timezone: Option<String>) {
        self.timezone = timezone;
        self.updated_at = Utc::now();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_user_has_generated_id() {
        let user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );

        // ID should be valid (we can convert to string and back)
        let id_str = user.id().to_string();
        assert!(id_str.starts_with("usr_"));
    }

    #[test]
    fn new_user_has_subject_and_issuer() {
        let user = User::new(
            "user|abc123".to_string(),
            "https://auth.example.com".to_string(),
        );

        assert_eq!(user.subject(), "user|abc123");
        assert_eq!(user.issuer(), "https://auth.example.com");
    }

    #[test]
    fn new_user_has_no_optional_fields() {
        let user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );

        assert!(user.email().is_none());
        assert!(user.display_name().is_none());
        assert!(user.timezone().is_none());
    }

    #[test]
    fn new_user_has_timestamps() {
        let before = Utc::now();
        let user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        let after = Utc::now();

        assert!(user.created_at() >= before);
        assert!(user.created_at() <= after);
        assert_eq!(user.created_at(), user.updated_at());
    }

    #[test]
    fn set_email_updates_timestamp() {
        let mut user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        let original_updated_at = user.updated_at();

        // Small delay to ensure timestamp changes
        std::thread::sleep(std::time::Duration::from_millis(1));

        user.set_email(Some("user@example.com".to_string()));

        assert_eq!(user.email(), Some("user@example.com"));
        assert!(user.updated_at() > original_updated_at);
    }

    #[test]
    fn set_display_name_updates_timestamp() {
        let mut user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        let original_updated_at = user.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(1));

        user.set_display_name(Some("Alice".to_string()));

        assert_eq!(user.display_name(), Some("Alice"));
        assert!(user.updated_at() > original_updated_at);
    }

    #[test]
    fn set_timezone_updates_timestamp() {
        let mut user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        let original_updated_at = user.updated_at();

        std::thread::sleep(std::time::Duration::from_millis(1));

        user.set_timezone(Some("America/New_York".to_string()));

        assert_eq!(user.timezone(), Some("America/New_York"));
        assert!(user.updated_at() > original_updated_at);
    }

    #[test]
    fn with_all_fields_preserves_values() {
        let id = UserId::new();
        let created = Utc::now() - chrono::Duration::days(30);
        let updated = Utc::now() - chrono::Duration::days(1);

        let user = User::with_all_fields(
            id,
            "sub_456".to_string(),
            "https://auth.example.com".to_string(),
            Some("alice@example.com".to_string()),
            Some("Alice".to_string()),
            Some("Europe/London".to_string()),
            created,
            updated,
        );

        assert_eq!(user.id(), id);
        assert_eq!(user.subject(), "sub_456");
        assert_eq!(user.issuer(), "https://auth.example.com");
        assert_eq!(user.email(), Some("alice@example.com"));
        assert_eq!(user.display_name(), Some("Alice"));
        assert_eq!(user.timezone(), Some("Europe/London"));
        assert_eq!(user.created_at(), created);
        assert_eq!(user.updated_at(), updated);
    }

    #[test]
    fn user_serialization_roundtrip() {
        let mut user = User::new(
            "sub_123".to_string(),
            "https://auth.example.com".to_string(),
        );
        user.set_email(Some("test@example.com".to_string()));
        user.set_timezone(Some("America/Los_Angeles".to_string()));

        let json = serde_json::to_string(&user).expect("serialize");
        let parsed: User = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(user, parsed);
    }
}
