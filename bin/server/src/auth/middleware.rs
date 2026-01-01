//! Authentication middleware and extractors for Axum.

use axum::{
    extract::{FromRef, FromRequestParts},
    http::{StatusCode, request::Parts},
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::CookieJar;
use silver_telegram_platform_access::{AuthenticatedUser, SessionId};
use std::sync::Arc;

use super::{
    AppState,
    db::{SessionRepository, UserRepository},
};

/// Session cookie name.
const SESSION_COOKIE: &str = "session";

/// Extractor for requiring an authenticated user.
///
/// If the user is not authenticated, they will be redirected to the login page.
pub struct RequireAuth(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireAuth
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let app_state = Arc::<AppState>::from_ref(state);
        let jar = CookieJar::from_request_parts(parts, state)
            .await
            .map_err(|_| AuthRejection::InternalError)?;

        // Get session ID from cookie
        let session_cookie = jar
            .get(SESSION_COOKIE)
            .ok_or(AuthRejection::NotAuthenticated)?;

        let session_id = SessionId::new(session_cookie.value().to_string());

        // Look up session in database
        let session_repo = SessionRepository::new(app_state.db_pool.clone());
        let session = session_repo
            .find_by_id(&session_id)
            .await
            .map_err(|_| AuthRejection::InternalError)?
            .ok_or(AuthRejection::NotAuthenticated)?;

        // Check if session is expired
        if session.is_expired() {
            // Delete the expired session
            let _ = session_repo.delete(&session_id).await;
            return Err(AuthRejection::SessionExpired);
        }

        // Check if user has access
        if !session.has_access() {
            return Err(AuthRejection::AccessDenied);
        }

        // Load user from database
        let user_repo = UserRepository::new(app_state.db_pool.clone());
        let user = user_repo
            .find_by_id(session.user_id())
            .await
            .map_err(|_| AuthRejection::InternalError)?
            .ok_or(AuthRejection::NotAuthenticated)?;

        Ok(RequireAuth(AuthenticatedUser::new(session, user)))
    }
}

/// Extractor for optionally getting the authenticated user.
///
/// Returns None if the user is not authenticated.
pub struct OptionalAuth(pub Option<AuthenticatedUser>);

impl<S> FromRequestParts<S> for OptionalAuth
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = std::convert::Infallible;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        match RequireAuth::from_request_parts(parts, state).await {
            Ok(RequireAuth(user)) => Ok(OptionalAuth(Some(user))),
            Err(_) => Ok(OptionalAuth(None)),
        }
    }
}

/// Extractor for requiring an authenticated admin user.
pub struct RequireAdmin(pub AuthenticatedUser);

impl<S> FromRequestParts<S> for RequireAdmin
where
    Arc<AppState>: FromRef<S>,
    S: Send + Sync,
{
    type Rejection = AuthRejection;

    async fn from_request_parts(parts: &mut Parts, state: &S) -> Result<Self, Self::Rejection> {
        let RequireAuth(user) = RequireAuth::from_request_parts(parts, state).await?;

        if !user.is_admin() {
            return Err(AuthRejection::AdminRequired);
        }

        Ok(RequireAdmin(user))
    }
}

/// Rejection type for authentication extractors.
#[derive(Debug)]
pub enum AuthRejection {
    NotAuthenticated,
    SessionExpired,
    AccessDenied,
    AdminRequired,
    InternalError,
}

impl IntoResponse for AuthRejection {
    fn into_response(self) -> Response {
        match self {
            Self::NotAuthenticated | Self::SessionExpired => {
                Redirect::to("/auth/login").into_response()
            }
            Self::AccessDenied => (StatusCode::FORBIDDEN, "Access denied").into_response(),
            Self::AdminRequired => (StatusCode::FORBIDDEN, "Admin access required").into_response(),
            Self::InternalError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error").into_response()
            }
        }
    }
}
