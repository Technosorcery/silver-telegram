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
