//! OIDC (OpenID Connect) configuration and client setup.
//!
//! This module provides configuration types for connecting to an external
//! OIDC identity provider for user authentication.

use serde::{Deserialize, Serialize};

/// Configuration for the OIDC identity provider.
///
/// This configuration is used to connect to an external OIDC provider
/// (e.g., Keycloak, Auth0, Authentik) for user authentication.
///
/// Fields with defaults can be omitted when loading from environment variables.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OidcConfig {
    /// The OIDC issuer URL (e.g., "https://auth.example.com/realms/main").
    /// Used for OIDC discovery.
    issuer_url: String,
    /// The OAuth2 client ID registered with the provider.
    client_id: String,
    /// The OAuth2 client secret.
    client_secret: String,
    /// The redirect URI for the OAuth2 callback (e.g., "https://app.example.com/auth/callback").
    redirect_uri: String,
    /// OAuth2 scopes to request as a comma-separated string.
    /// Default: "openid,email,profile"
    #[serde(default = "default_scopes")]
    scopes: String,
    /// The claim name in the ID token that contains user groups.
    /// Default: "groups"
    #[serde(default = "default_groups_claim")]
    groups_claim: String,
    /// The group name that grants user-level access to the platform.
    /// Default: "platform-users"
    #[serde(default = "default_user_group")]
    user_group: String,
    /// The group name that grants admin-level access to the platform.
    /// Default: "platform-admins"
    #[serde(default = "default_admin_group")]
    admin_group: String,
}

fn default_scopes() -> String {
    "openid,email,profile".to_string()
}

fn default_groups_claim() -> String {
    "groups".to_string()
}

fn default_user_group() -> String {
    "platform-users".to_string()
}

fn default_admin_group() -> String {
    "platform-admins".to_string()
}

impl OidcConfig {
    /// Creates a new OIDC configuration with defaults for optional fields.
    #[must_use]
    pub fn new(
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self {
            issuer_url,
            client_id,
            client_secret,
            redirect_uri,
            scopes: default_scopes(),
            groups_claim: default_groups_claim(),
            user_group: default_user_group(),
            admin_group: default_admin_group(),
        }
    }

    /// Creates a configuration builder for more customization.
    #[must_use]
    pub fn builder(
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> OidcConfigBuilder {
        OidcConfigBuilder::new(issuer_url, client_id, client_secret, redirect_uri)
    }

    /// Returns the OIDC issuer URL.
    #[must_use]
    pub fn issuer_url(&self) -> &str {
        &self.issuer_url
    }

    /// Returns the OAuth2 client ID.
    #[must_use]
    pub fn client_id(&self) -> &str {
        &self.client_id
    }

    /// Returns the OAuth2 client secret.
    #[must_use]
    pub fn client_secret(&self) -> &str {
        &self.client_secret
    }

    /// Returns the OAuth2 redirect URI.
    #[must_use]
    pub fn redirect_uri(&self) -> &str {
        &self.redirect_uri
    }

    /// Returns the OAuth2 scopes to request, parsed from comma-separated string.
    #[must_use]
    pub fn scopes(&self) -> Vec<&str> {
        self.scopes.split(',').map(str::trim).collect()
    }

    /// Returns the raw scopes string.
    #[must_use]
    pub fn scopes_raw(&self) -> &str {
        &self.scopes
    }

    /// Returns the name of the claim containing user groups.
    #[must_use]
    pub fn groups_claim(&self) -> &str {
        &self.groups_claim
    }

    /// Returns the group name for user-level access.
    #[must_use]
    pub fn user_group(&self) -> &str {
        &self.user_group
    }

    /// Returns the group name for admin-level access.
    #[must_use]
    pub fn admin_group(&self) -> &str {
        &self.admin_group
    }
}

/// Builder for `OidcConfig`.
#[derive(Debug)]
pub struct OidcConfigBuilder {
    issuer_url: String,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
    scopes: Vec<String>,
    groups_claim: String,
    user_group: String,
    admin_group: String,
}

impl OidcConfigBuilder {
    /// Creates a new builder with required fields.
    #[must_use]
    pub fn new(
        issuer_url: String,
        client_id: String,
        client_secret: String,
        redirect_uri: String,
    ) -> Self {
        Self {
            issuer_url,
            client_id,
            client_secret,
            redirect_uri,
            scopes: vec![
                "openid".to_string(),
                "email".to_string(),
                "profile".to_string(),
            ],
            groups_claim: default_groups_claim(),
            user_group: default_user_group(),
            admin_group: default_admin_group(),
        }
    }

    /// Sets the OAuth2 scopes to request.
    #[must_use]
    pub fn scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Adds a scope to the list of scopes to request.
    #[must_use]
    pub fn add_scope(mut self, scope: String) -> Self {
        if !self.scopes.contains(&scope) {
            self.scopes.push(scope);
        }
        self
    }

    /// Sets the claim name for user groups.
    #[must_use]
    pub fn groups_claim(mut self, claim: String) -> Self {
        self.groups_claim = claim;
        self
    }

    /// Sets the group name for user-level access.
    #[must_use]
    pub fn user_group(mut self, group: String) -> Self {
        self.user_group = group;
        self
    }

    /// Sets the group name for admin-level access.
    #[must_use]
    pub fn admin_group(mut self, group: String) -> Self {
        self.admin_group = group;
        self
    }

    /// Builds the `OidcConfig`.
    #[must_use]
    pub fn build(self) -> OidcConfig {
        OidcConfig {
            issuer_url: self.issuer_url,
            client_id: self.client_id,
            client_secret: self.client_secret,
            redirect_uri: self.redirect_uri,
            scopes: self.scopes.join(","),
            groups_claim: self.groups_claim,
            user_group: self.user_group,
            admin_group: self.admin_group,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_config_has_defaults() {
        let config = OidcConfig::new(
            "https://auth.example.com".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
            "https://app.example.com/auth/callback".to_string(),
        );

        assert_eq!(config.issuer_url(), "https://auth.example.com");
        assert_eq!(config.client_id(), "client-id");
        assert_eq!(config.client_secret(), "client-secret");
        assert_eq!(
            config.redirect_uri(),
            "https://app.example.com/auth/callback"
        );
        assert!(config.scopes().contains(&"openid"));
        assert!(config.scopes().contains(&"email"));
        assert!(config.scopes().contains(&"profile"));
        assert_eq!(config.groups_claim(), "groups");
        assert_eq!(config.user_group(), "platform-users");
        assert_eq!(config.admin_group(), "platform-admins");
    }

    #[test]
    fn builder_allows_customization() {
        let config = OidcConfig::builder(
            "https://auth.example.com".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
            "https://app.example.com/auth/callback".to_string(),
        )
        .groups_claim("cognito:groups".to_string())
        .user_group("MyApp-Users".to_string())
        .admin_group("MyApp-Admins".to_string())
        .add_scope("groups".to_string())
        .build();

        assert_eq!(config.groups_claim(), "cognito:groups");
        assert_eq!(config.user_group(), "MyApp-Users");
        assert_eq!(config.admin_group(), "MyApp-Admins");
        assert!(config.scopes().contains(&"groups"));
    }

    #[test]
    fn builder_add_scope_does_not_duplicate() {
        let config = OidcConfig::builder(
            "https://auth.example.com".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
            "https://app.example.com/auth/callback".to_string(),
        )
        .add_scope("openid".to_string()) // Already present
        .add_scope("custom".to_string())
        .build();

        let openid_count = config.scopes().iter().filter(|s| *s == &"openid").count();
        assert_eq!(openid_count, 1);
        assert!(config.scopes().contains(&"custom"));
    }

    #[test]
    fn config_serialization_roundtrip() {
        let config = OidcConfig::new(
            "https://auth.example.com".to_string(),
            "client-id".to_string(),
            "client-secret".to_string(),
            "https://app.example.com/auth/callback".to_string(),
        );

        let json = serde_json::to_string(&config).expect("serialize");
        let parsed: OidcConfig = serde_json::from_str(&json).expect("deserialize");

        assert_eq!(config.issuer_url(), parsed.issuer_url());
        assert_eq!(config.client_id(), parsed.client_id());
        assert_eq!(config.scopes(), parsed.scopes());
    }

    #[test]
    fn config_deserializes_with_defaults() {
        let json = r#"{
            "issuer_url": "https://auth.example.com",
            "client_id": "my-client",
            "client_secret": "secret",
            "redirect_uri": "https://app.example.com/callback"
        }"#;

        let config: OidcConfig = serde_json::from_str(json).expect("deserialize");

        assert_eq!(config.issuer_url(), "https://auth.example.com");
        assert_eq!(config.client_id(), "my-client");
        assert_eq!(config.scopes(), vec!["openid", "email", "profile"]);
        assert_eq!(config.groups_claim(), "groups");
        assert_eq!(config.user_group(), "platform-users");
        assert_eq!(config.admin_group(), "platform-admins");
    }

    #[test]
    fn scopes_parses_comma_separated() {
        let json = r#"{
            "issuer_url": "https://auth.example.com",
            "client_id": "my-client",
            "client_secret": "secret",
            "redirect_uri": "https://app.example.com/callback",
            "scopes": "openid, email, profile, groups"
        }"#;

        let config: OidcConfig = serde_json::from_str(json).expect("deserialize");

        assert_eq!(
            config.scopes(),
            vec!["openid", "email", "profile", "groups"]
        );
    }
}
