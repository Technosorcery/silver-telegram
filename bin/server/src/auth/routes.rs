//! Authentication routes for login, callback, and logout.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use chrono::Duration as ChronoDuration;
use serde::Deserialize;
use silver_telegram_platform_access::{RoleSet, Session, User};
use std::sync::Arc;
use time::Duration as TimeDuration;

use super::{
    AppState,
    db::{SessionRepository, UserRepository, generate_session_id},
    oidc::AuthState,
};

/// Session cookie name.
const SESSION_COOKIE: &str = "session";

/// Auth state cookie name (for CSRF protection during OIDC flow).
const AUTH_STATE_COOKIE: &str = "auth_state";

/// Query parameters for the OIDC callback.
#[derive(Debug, Deserialize)]
pub struct CallbackQuery {
    code: String,
    state: String,
}

/// Initiates the OIDC login flow by redirecting to the identity provider.
pub async fn login(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    let (auth_url, auth_state) = state.oidc_client.authorization_url();

    // Store the auth state in a secure cookie for validation on callback
    let auth_state_json = serde_json::to_string(&AuthStateData {
        csrf_token: auth_state.csrf_token,
        pkce_verifier: auth_state.pkce_verifier,
        nonce: auth_state.nonce,
    })
    .expect("serialize auth state");

    let cookie = Cookie::build((AUTH_STATE_COOKIE, auth_state_json))
        .path("/")
        .http_only(true)
        .secure(state.session_config.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::minutes(10));

    (jar.add(cookie), Redirect::to(&auth_url))
}

/// Handles the OIDC callback after the user authenticates with the identity provider.
pub async fn callback(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CallbackQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, AuthError> {
    // Retrieve and validate auth state from cookie
    let auth_state_cookie = jar
        .get(AUTH_STATE_COOKIE)
        .ok_or(AuthError::MissingAuthState)?;

    let auth_state_data: AuthStateData =
        serde_json::from_str(auth_state_cookie.value()).map_err(|_| AuthError::InvalidAuthState)?;

    // Validate CSRF token
    if query.state != auth_state_data.csrf_token {
        return Err(AuthError::CsrfMismatch);
    }

    let auth_state = AuthState {
        csrf_token: auth_state_data.csrf_token,
        pkce_verifier: auth_state_data.pkce_verifier,
        nonce: auth_state_data.nonce,
    };

    // Exchange the authorization code for tokens
    let token_result = state
        .oidc_client
        .exchange_code(&query.code, &auth_state)
        .await
        .map_err(|e| AuthError::TokenExchange(e.to_string()))?;

    let claims = token_result.claims;

    // Check if user has access (has user group)
    let roles = RoleSet::from_groups(
        &claims.groups,
        state.oidc_client.config().user_group(),
        state.oidc_client.config().admin_group(),
    );

    if !roles.has_access() {
        return Err(AuthError::AccessDenied);
    }

    // Find or create user
    let user_repo = UserRepository::new(state.db_pool.clone());
    let existing_user = user_repo
        .find_by_subject_issuer(&claims.subject, &claims.issuer)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    let user = match existing_user {
        Some(mut user) => {
            // Update user info from claims
            user.set_email(claims.email.clone());
            user.set_display_name(claims.display_name.clone());
            user_repo
                .update(&user)
                .await
                .map_err(|e| AuthError::Database(e.to_string()))?;
            user
        }
        None => {
            // Create new user
            let mut user = User::new(claims.subject.clone(), claims.issuer.clone());
            user.set_email(claims.email.clone());
            user.set_display_name(claims.display_name.clone());
            user_repo
                .create(&user)
                .await
                .map_err(|e| AuthError::Database(e.to_string()))?;
            user
        }
    };

    // Create session
    let session_id = generate_session_id();
    let session_duration = state.session_config.duration_minutes;
    let session = Session::with_tokens(
        session_id.clone(),
        user.id(),
        roles,
        ChronoDuration::minutes(session_duration),
        token_result.access_token,
        token_result.refresh_token,
    );

    let session_repo = SessionRepository::new(state.db_pool.clone());
    session_repo
        .create(&session)
        .await
        .map_err(|e| AuthError::Database(e.to_string()))?;

    // Set session cookie
    let session_cookie = Cookie::build((SESSION_COOKIE, session_id.as_str().to_string()))
        .path("/")
        .http_only(true)
        .secure(state.session_config.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::minutes(session_duration));

    // Remove auth state cookie
    let remove_auth_state = Cookie::build((AUTH_STATE_COOKIE, ""))
        .path("/")
        .max_age(TimeDuration::ZERO);

    let jar = jar.add(session_cookie).add(remove_auth_state);

    Ok((jar, Redirect::to("/")))
}

/// Logs out the user by deleting their session.
pub async fn logout(State(state): State<Arc<AppState>>, jar: CookieJar) -> impl IntoResponse {
    // Get session ID from cookie
    if let Some(session_cookie) = jar.get(SESSION_COOKIE) {
        let session_id =
            silver_telegram_platform_access::SessionId::new(session_cookie.value().to_string());

        // Delete session from database
        let session_repo = SessionRepository::new(state.db_pool.clone());
        let _ = session_repo.delete(&session_id).await;
    }

    // Remove session cookie
    let remove_session = Cookie::build((SESSION_COOKIE, ""))
        .path("/")
        .max_age(TimeDuration::ZERO);

    (jar.add(remove_session), Redirect::to("/"))
}

/// Serializable auth state for cookie storage.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct AuthStateData {
    csrf_token: String,
    pkce_verifier: String,
    nonce: String,
}

/// Authentication errors.
#[derive(Debug)]
pub enum AuthError {
    MissingAuthState,
    InvalidAuthState,
    CsrfMismatch,
    TokenExchange(String),
    AccessDenied,
    Database(String),
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::MissingAuthState => (StatusCode::BAD_REQUEST, "Missing auth state"),
            Self::InvalidAuthState => (StatusCode::BAD_REQUEST, "Invalid auth state"),
            Self::CsrfMismatch => (StatusCode::BAD_REQUEST, "CSRF token mismatch"),
            Self::TokenExchange(msg) => {
                tracing::error!("Token exchange failed: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed")
            }
            Self::AccessDenied => (
                StatusCode::FORBIDDEN,
                "Access denied - you are not authorized to use this platform",
            ),
            Self::Database(msg) => {
                tracing::error!("Database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal server error")
            }
        };

        (status, message).into_response()
    }
}
