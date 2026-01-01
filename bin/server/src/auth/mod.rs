//! Authentication module for the silver-telegram server.
//!
//! This module provides:
//! - OIDC authentication with external identity providers
//! - Database-backed session management
//! - Authentication middleware/extractors for Axum routes
//!
//! # Authorization Model
//!
//! This module handles **platform access** authorization: determining whether a user
//! can log into the platform at all. This is based on OIDC group membership:
//! - Users with the configured user group can access the platform
//! - Users with the admin group get additional administrative capabilities
//!
//! Session-embedded roles are used for this purpose because:
//! - Platform access is checked on every request (performance critical)
//! - Group membership changes take effect on next login (or session expiry)
//! - Short session durations (5 minutes) bound the revocation latency
//!
//! **Resource authorization** (can user X access workflow Y?) will be handled by
//! SpiceDB as specified in ADR-002. When resource-level authorization is implemented:
//! - SpiceDB will handle relationship-based access checks (ownership, sharing, etc.)
//! - The `user_id` from the session will be used to query SpiceDB
//! - No `user_id` columns on resource tables - relationships live in SpiceDB

pub mod db;
pub mod gmail;
pub mod middleware;
pub mod oidc;
pub mod routes;

pub use gmail::{GmailOAuthClient, GmailOAuthState, gmail_callback, gmail_start};

use crate::config::SessionConfig;
use sqlx::PgPool;

pub use middleware::{OptionalAuth, RequireAdmin, RequireAuth};
pub use oidc::OidcClient;
pub use routes::{callback, login, logout};

/// Shared application state.
pub struct AppState {
    /// Database connection pool.
    pub db_pool: PgPool,
    /// OIDC client for authentication.
    pub oidc_client: OidcClient,
    /// Session configuration.
    pub session_config: SessionConfig,
}

impl AppState {
    /// Creates a new application state.
    pub fn new(db_pool: PgPool, oidc_client: OidcClient, session_config: SessionConfig) -> Self {
        Self {
            db_pool,
            oidc_client,
            session_config,
        }
    }
}
