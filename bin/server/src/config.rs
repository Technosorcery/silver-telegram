//! Centralized server configuration.
//!
//! This module provides strongly-typed configuration for the server,
//! loaded via the `config` crate from environment variables.
//!
//! See [`OidcConfig`](silver_telegram_platform_access::OidcConfig) for
//! OIDC authentication configuration.

use serde::Deserialize;
use silver_telegram_platform_access::OidcConfig;

/// Server configuration composed from library configs.
#[derive(Debug, Deserialize)]
pub struct ServerConfig {
    /// PostgreSQL database connection URL.
    pub database_url: String,

    /// Session configuration.
    #[serde(default)]
    pub session: SessionConfig,

    /// OIDC authentication configuration.
    pub oidc: OidcConfig,

    /// SpiceDB authorization configuration.
    #[serde(default)]
    pub spicedb: SpiceDbConfig,

    /// Google OAuth configuration for Gmail integration.
    #[serde(default)]
    pub google: GoogleOAuthConfig,
}

/// Google OAuth configuration for Gmail integration.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct GoogleOAuthConfig {
    /// Google OAuth client ID.
    #[serde(default)]
    pub client_id: Option<String>,

    /// Google OAuth client secret.
    #[serde(default)]
    pub client_secret: Option<String>,

    /// Redirect URL for OAuth callback (e.g., "http://localhost:3000/auth/gmail/callback").
    #[serde(default)]
    pub redirect_url: Option<String>,
}

impl GoogleOAuthConfig {
    /// Returns whether Google OAuth is configured.
    #[must_use]
    pub fn is_configured(&self) -> bool {
        self.client_id.is_some() && self.client_secret.is_some() && self.redirect_url.is_some()
    }
}

/// SpiceDB authorization configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SpiceDbConfig {
    /// SpiceDB gRPC endpoint (e.g., "http://localhost:50051").
    #[serde(default = "default_spicedb_endpoint")]
    pub endpoint: String,

    /// Preshared key for SpiceDB authentication.
    #[serde(default = "default_spicedb_preshared_key")]
    pub preshared_key: String,
}

fn default_spicedb_endpoint() -> String {
    "http://localhost:50051".to_string()
}

fn default_spicedb_preshared_key() -> String {
    "silver_dev_key".to_string()
}

impl Default for SpiceDbConfig {
    fn default() -> Self {
        Self {
            endpoint: default_spicedb_endpoint(),
            preshared_key: default_spicedb_preshared_key(),
        }
    }
}

/// Session-related configuration.
#[derive(Debug, Clone, Deserialize)]
pub struct SessionConfig {
    /// Session duration in minutes.
    /// Short sessions bound revocation latency.
    #[serde(default = "default_session_duration_minutes")]
    pub duration_minutes: i64,

    /// Interval between session cleanup runs, in seconds.
    #[serde(default = "default_cleanup_interval_seconds")]
    pub cleanup_interval_seconds: u64,

    /// Whether to set the Secure flag on cookies (requires HTTPS).
    /// Defaults to true for production safety; set to false for local HTTP development.
    #[serde(default = "default_secure_cookies")]
    pub secure_cookies: bool,
}

fn default_session_duration_minutes() -> i64 {
    5
}

fn default_cleanup_interval_seconds() -> u64 {
    300
}

fn default_secure_cookies() -> bool {
    true
}

impl Default for SessionConfig {
    fn default() -> Self {
        Self {
            duration_minutes: default_session_duration_minutes(),
            cleanup_interval_seconds: default_cleanup_interval_seconds(),
            secure_cookies: default_secure_cookies(),
        }
    }
}

impl ServerConfig {
    /// Loads configuration from environment variables.
    ///
    /// # Errors
    ///
    /// Returns an error if required configuration is missing or invalid.
    pub fn from_env() -> Result<Self, config::ConfigError> {
        config::Config::builder()
            .add_source(
                config::Environment::default()
                    .separator("__")
                    .try_parsing(true),
            )
            .build()?
            .try_deserialize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_config_has_correct_defaults() {
        let config = SessionConfig::default();
        assert_eq!(config.duration_minutes, 5);
        assert_eq!(config.cleanup_interval_seconds, 300);
    }
}
