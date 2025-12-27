//! Credential vault for secure credential storage.
//!
//! All integration credentials are encrypted at rest.
//! No plaintext credentials are stored in configuration or logs.

use crate::error::CredentialError;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use silver_telegram_core::{CredentialId, IntegrationAccountId, UserId};

/// The type of credential.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CredentialType {
    /// OAuth 2.0 tokens.
    Oauth2,
    /// API key.
    ApiKey,
    /// Username and password.
    BasicAuth,
    /// Bearer token.
    BearerToken,
    /// Custom credential format.
    Custom,
}

/// Credential data (encrypted at rest).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CredentialData {
    /// OAuth 2.0 tokens.
    Oauth2 {
        access_token: String,
        refresh_token: Option<String>,
        token_type: String,
        expires_at: Option<DateTime<Utc>>,
        scope: Option<String>,
    },
    /// API key.
    ApiKey {
        key: String,
        header_name: Option<String>,
    },
    /// Basic authentication.
    BasicAuth { username: String, password: String },
    /// Bearer token.
    BearerToken { token: String },
    /// Custom credential data.
    Custom { data: serde_json::Value },
}

impl CredentialData {
    /// Creates OAuth2 credential data.
    #[must_use]
    pub fn oauth2(access_token: impl Into<String>) -> Self {
        Self::Oauth2 {
            access_token: access_token.into(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: None,
            scope: None,
        }
    }

    /// Creates API key credential data.
    #[must_use]
    pub fn api_key(key: impl Into<String>) -> Self {
        Self::ApiKey {
            key: key.into(),
            header_name: None,
        }
    }

    /// Creates basic auth credential data.
    #[must_use]
    pub fn basic_auth(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self::BasicAuth {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Returns the credential type.
    #[must_use]
    pub fn credential_type(&self) -> CredentialType {
        match self {
            Self::Oauth2 { .. } => CredentialType::Oauth2,
            Self::ApiKey { .. } => CredentialType::ApiKey,
            Self::BasicAuth { .. } => CredentialType::BasicAuth,
            Self::BearerToken { .. } => CredentialType::BearerToken,
            Self::Custom { .. } => CredentialType::Custom,
        }
    }

    /// Checks if OAuth2 credentials need refresh.
    #[must_use]
    pub fn needs_refresh(&self) -> bool {
        if let Self::Oauth2 {
            expires_at: Some(expires),
            ..
        } = self
        {
            // Refresh if expiring within 5 minutes
            return *expires < Utc::now() + chrono::Duration::minutes(5);
        }
        false
    }
}

/// A stored credential.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Credential {
    /// Unique identifier.
    pub id: CredentialId,
    /// The integration account this credential belongs to.
    pub integration_account_id: IntegrationAccountId,
    /// The user who owns this credential.
    pub user_id: UserId,
    /// Credential name/label.
    pub name: String,
    /// Credential type.
    pub credential_type: CredentialType,
    /// When the credential was created.
    pub created_at: DateTime<Utc>,
    /// When the credential was last updated.
    pub updated_at: DateTime<Utc>,
    /// When the credential was last used.
    pub last_used_at: Option<DateTime<Utc>>,
}

impl Credential {
    /// Creates a new credential.
    #[must_use]
    pub fn new(
        integration_account_id: IntegrationAccountId,
        user_id: UserId,
        name: impl Into<String>,
        credential_type: CredentialType,
    ) -> Self {
        let now = Utc::now();
        Self {
            id: CredentialId::new(),
            integration_account_id,
            user_id,
            name: name.into(),
            credential_type,
            created_at: now,
            updated_at: now,
            last_used_at: None,
        }
    }

    /// Marks the credential as used.
    pub fn mark_used(&mut self) {
        self.last_used_at = Some(Utc::now());
    }

    /// Marks the credential as updated.
    pub fn mark_updated(&mut self) {
        self.updated_at = Utc::now();
    }
}

/// Trait for credential storage.
///
/// Implementations must encrypt credentials at rest.
#[async_trait]
pub trait CredentialVault: Send + Sync {
    /// Stores a credential with its data.
    ///
    /// # Errors
    ///
    /// Returns an error if storage fails.
    async fn store(
        &self,
        credential: Credential,
        data: CredentialData,
    ) -> Result<CredentialId, CredentialError>;

    /// Retrieves credential metadata (without data).
    async fn get_metadata(&self, id: CredentialId) -> Result<Credential, CredentialError>;

    /// Retrieves credential data (decrypted).
    async fn get_data(&self, id: CredentialId) -> Result<CredentialData, CredentialError>;

    /// Updates credential data.
    async fn update_data(
        &self,
        id: CredentialId,
        data: CredentialData,
    ) -> Result<(), CredentialError>;

    /// Deletes a credential.
    async fn delete(&self, id: CredentialId) -> Result<(), CredentialError>;

    /// Lists credentials for an integration account.
    async fn list_for_account(
        &self,
        account_id: IntegrationAccountId,
    ) -> Result<Vec<Credential>, CredentialError>;

    /// Lists credentials for a user.
    async fn list_for_user(&self, user_id: UserId) -> Result<Vec<Credential>, CredentialError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oauth2_credential_data() {
        let data = CredentialData::oauth2("access_token_123");
        assert_eq!(data.credential_type(), CredentialType::Oauth2);
    }

    #[test]
    fn api_key_credential_data() {
        let data = CredentialData::api_key("my_api_key");
        assert_eq!(data.credential_type(), CredentialType::ApiKey);
    }

    #[test]
    fn basic_auth_credential_data() {
        let data = CredentialData::basic_auth("user", "pass");
        assert_eq!(data.credential_type(), CredentialType::BasicAuth);
    }

    #[test]
    fn oauth2_needs_refresh() {
        let expired = CredentialData::Oauth2 {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() - chrono::Duration::hours(1)),
            scope: None,
        };
        assert!(expired.needs_refresh());

        let valid = CredentialData::Oauth2 {
            access_token: "token".to_string(),
            refresh_token: None,
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now() + chrono::Duration::hours(1)),
            scope: None,
        };
        assert!(!valid.needs_refresh());
    }

    #[test]
    fn credential_creation() {
        let cred = Credential::new(
            IntegrationAccountId::new(),
            UserId::new(),
            "Gmail Token",
            CredentialType::Oauth2,
        );

        assert_eq!(cred.name, "Gmail Token");
        assert_eq!(cred.credential_type, CredentialType::Oauth2);
        assert!(cred.last_used_at.is_none());
    }

    #[test]
    fn credential_serde_roundtrip() {
        let data = CredentialData::Oauth2 {
            access_token: "token".to_string(),
            refresh_token: Some("refresh".to_string()),
            token_type: "Bearer".to_string(),
            expires_at: Some(Utc::now()),
            scope: Some("email".to_string()),
        };

        let json = serde_json::to_string(&data).expect("serialize");
        let parsed: CredentialData = serde_json::from_str(&json).expect("deserialize");

        match parsed {
            CredentialData::Oauth2 { access_token, .. } => {
                assert_eq!(access_token, "token");
            }
            _ => panic!("wrong credential type"),
        }
    }
}
