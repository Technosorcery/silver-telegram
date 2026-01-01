//! Helper functions for server functions with proper error handling and logging.
//!
//! This module provides utilities for common patterns in server functions,
//! including authentication, authorization, and error handling.

use crate::auth::db::SessionRepository;
use crate::error::SessionError;
use leptos::prelude::*;
use silver_telegram_authz::AuthzClient;
use silver_telegram_core::id::UserId;
use silver_telegram_platform_access::SessionId;
use silver_telegram_platform_access::session::Session;
use sqlx::PgPool;
use std::sync::Arc;

/// Authenticated session information.
pub struct AuthenticatedSession {
    pub session: Session,
    pub user_id: UserId,
}

/// Extracts and validates the current session from the request.
///
/// This function:
/// 1. Gets the session cookie
/// 2. Looks up the session in the database
/// 3. Validates the session is not expired
/// 4. Returns the session and user ID
///
/// Logs structured errors for debugging while returning user-safe error types.
pub async fn get_authenticated_session() -> Result<AuthenticatedSession, SessionError> {
    // Get session cookie
    let session_id_str = leptos_axum::extract::<axum_extra::extract::CookieJar>()
        .await
        .map_err(|e| {
            tracing::debug!(error = %e, "Failed to extract cookie jar");
            SessionError::NotAuthenticated
        })?
        .get("session")
        .map(|c| c.value().to_string())
        .ok_or(SessionError::NotAuthenticated)?;

    let session_id = SessionId::new(session_id_str.clone());

    // Get database pool
    let pool = expect_context::<PgPool>();
    let session_repo = SessionRepository::new(pool);

    // Look up session
    let session = session_repo
        .find_by_id(&session_id)
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                session_id = %session_id_str,
                "Database error looking up session"
            );
            SessionError::DatabaseError {
                details: e.to_string(),
            }
        })?
        .ok_or_else(|| {
            tracing::debug!(session_id = %session_id_str, "Session not found in database");
            SessionError::NotFound {
                session_id: session_id_str.clone(),
            }
        })?;

    // Check session is valid
    if !session.has_access() {
        tracing::debug!(
            session_id = %session_id_str,
            expired = session.is_expired(),
            "Session expired or access denied"
        );
        return Err(SessionError::Expired {
            session_id: session_id_str,
        });
    }

    let user_id = session.user_id();

    Ok(AuthenticatedSession { session, user_id })
}

/// Extracts and validates an admin session.
///
/// This function:
/// 1. Gets the authenticated session
/// 2. Verifies the user has admin access
///
/// Returns SessionError::AdminRequired if not an admin.
pub async fn get_admin_session() -> Result<AuthenticatedSession, SessionError> {
    let auth = get_authenticated_session().await?;

    if !auth.session.is_admin() {
        tracing::warn!(
            user_id = %auth.user_id,
            "Non-admin user attempted admin operation"
        );
        return Err(SessionError::AdminRequired);
    }

    Ok(auth)
}

/// Gets the authorization client from the request context.
pub fn get_authz_client() -> Arc<AuthzClient> {
    expect_context::<Arc<AuthzClient>>()
}

/// Gets the database pool from the request context.
pub fn get_db_pool() -> PgPool {
    expect_context::<PgPool>()
}
