//! Gmail OAuth implementation for integration authentication.
//!
//! This module handles OAuth 2.0 flow for Gmail integrations:
//! - `/auth/gmail/start?integration_id=...` - Initiates OAuth flow
//! - `/auth/gmail/callback` - Handles OAuth callback from Google
//!
//! After successful authentication, OAuth tokens are stored with the
//! integration account for use by the Gmail client.

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
};
use axum_extra::extract::cookie::{Cookie, CookieJar, SameSite};
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, EmptyExtraTokenFields,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, StandardTokenResponse, TokenResponse,
    TokenUrl,
    basic::{BasicClient, BasicTokenType},
};
use serde::Deserialize;
use time::Duration as TimeDuration;

use crate::config::GoogleOAuthConfig;

/// Google OAuth authorization URL.
const GOOGLE_AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";

/// Google OAuth token URL.
const GOOGLE_TOKEN_URL: &str = "https://oauth2.googleapis.com/token";

/// Gmail OAuth scopes.
const GMAIL_SCOPES: &[&str] = &[
    "https://www.googleapis.com/auth/gmail.readonly",
    "https://www.googleapis.com/auth/gmail.send",
    "https://www.googleapis.com/auth/gmail.modify",
];

/// Cookie name for Gmail OAuth state.
const GMAIL_AUTH_STATE_COOKIE: &str = "gmail_auth_state";

/// Type alias for the token response type.
type GmailTokenResponse = StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>;

/// Gmail OAuth client configuration.
#[derive(Clone)]
pub struct GmailOAuthClient {
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_url: String,
}

impl GmailOAuthClient {
    /// Creates a new Gmail OAuth client from configuration.
    ///
    /// # Errors
    ///
    /// Returns an error if configuration is invalid.
    pub fn new(config: &GoogleOAuthConfig) -> Result<Self, GmailOAuthError> {
        let client_id = config
            .client_id
            .as_ref()
            .ok_or(GmailOAuthError::NotConfigured)?
            .clone();
        let client_secret = config
            .client_secret
            .as_ref()
            .ok_or(GmailOAuthError::NotConfigured)?
            .clone();
        let redirect_url = config
            .redirect_url
            .as_ref()
            .ok_or(GmailOAuthError::NotConfigured)?
            .clone();

        // Validate URLs
        let _ = RedirectUrl::new(redirect_url.clone())
            .map_err(|e| GmailOAuthError::Configuration(format!("invalid redirect URL: {}", e)))?;

        Ok(Self {
            client_id,
            client_secret,
            auth_url: GOOGLE_AUTH_URL.to_string(),
            token_url: GOOGLE_TOKEN_URL.to_string(),
            redirect_url,
        })
    }

    /// Generates the authorization URL for Gmail OAuth.
    ///
    /// Returns the URL to redirect the user to, along with auth state to store.
    pub fn authorization_url(&self) -> (String, GmailAuthState) {
        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_auth_uri(AuthUrl::new(self.auth_url.clone()).expect("valid auth URL"))
            .set_redirect_uri(
                RedirectUrl::new(self.redirect_url.clone()).expect("valid redirect URL"),
            );

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client
            .authorize_url(CsrfToken::new_random)
            .set_pkce_challenge(pkce_challenge);

        // Add Gmail scopes
        for scope in GMAIL_SCOPES {
            auth_request = auth_request.add_scope(Scope::new((*scope).to_string()));
        }

        // Request offline access for refresh token
        auth_request = auth_request.add_extra_param("access_type", "offline");
        // Force consent screen to always get refresh token
        auth_request = auth_request.add_extra_param("prompt", "consent");

        let (auth_url, csrf_token) = auth_request.url();

        let state = GmailAuthState {
            csrf_token: csrf_token.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
        };

        (auth_url.to_string(), state)
    }

    /// Exchanges the authorization code for tokens.
    pub async fn exchange_code(
        &self,
        code: &str,
        pkce_verifier: &str,
    ) -> Result<GmailTokens, GmailOAuthError> {
        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| GmailOAuthError::TokenExchange(format!("HTTP client error: {}", e)))?;

        let client = BasicClient::new(ClientId::new(self.client_id.clone()))
            .set_client_secret(ClientSecret::new(self.client_secret.clone()))
            .set_token_uri(TokenUrl::new(self.token_url.clone()).expect("valid token URL"))
            .set_redirect_uri(
                RedirectUrl::new(self.redirect_url.clone()).expect("valid redirect URL"),
            );

        let pkce_verifier = PkceCodeVerifier::new(pkce_verifier.to_string());

        let token_result: GmailTokenResponse = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client)
            .await
            .map_err(|e| GmailOAuthError::TokenExchange(format!("Token exchange failed: {}", e)))?;

        let access_token = token_result.access_token().secret().clone();
        let refresh_token = token_result.refresh_token().map(|t| t.secret().clone());
        let expires_in = token_result.expires_in();

        Ok(GmailTokens {
            access_token,
            refresh_token,
            expires_in_seconds: expires_in.map(|d| d.as_secs()),
        })
    }
}

/// State stored during Gmail OAuth flow.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GmailAuthState {
    pub csrf_token: String,
    pub pkce_verifier: String,
}

/// Result of Gmail token exchange.
#[derive(Debug)]
pub struct GmailTokens {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_in_seconds: Option<u64>,
}

/// Gmail OAuth errors.
#[derive(Debug)]
pub enum GmailOAuthError {
    /// Google OAuth is not configured.
    NotConfigured,
    /// Configuration error.
    Configuration(String),
    /// Token exchange failed.
    TokenExchange(String),
    /// CSRF token mismatch.
    CsrfMismatch,
    /// Missing auth state.
    MissingAuthState,
    /// Invalid integration ID.
    InvalidIntegration,
    /// Database error.
    Database(String),
    /// Authorization error.
    Authorization(String),
}

impl std::fmt::Display for GmailOAuthError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotConfigured => write!(f, "Google OAuth is not configured"),
            Self::Configuration(msg) => write!(f, "Configuration error: {}", msg),
            Self::TokenExchange(msg) => write!(f, "Token exchange error: {}", msg),
            Self::CsrfMismatch => write!(f, "CSRF token mismatch"),
            Self::MissingAuthState => write!(f, "Missing auth state"),
            Self::InvalidIntegration => write!(f, "Invalid integration ID"),
            Self::Database(msg) => write!(f, "Database error: {}", msg),
            Self::Authorization(msg) => write!(f, "Authorization error: {}", msg),
        }
    }
}

impl std::error::Error for GmailOAuthError {}

impl IntoResponse for GmailOAuthError {
    fn into_response(self) -> Response {
        let (status, message) = match &self {
            Self::NotConfigured => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Gmail integration not available",
            ),
            Self::Configuration(_) => (StatusCode::INTERNAL_SERVER_ERROR, "Configuration error"),
            Self::TokenExchange(msg) => {
                tracing::error!("Gmail token exchange failed: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Authentication failed")
            }
            Self::CsrfMismatch => (StatusCode::BAD_REQUEST, "Invalid request state"),
            Self::MissingAuthState => (StatusCode::BAD_REQUEST, "Missing authentication state"),
            Self::InvalidIntegration => (StatusCode::BAD_REQUEST, "Invalid integration"),
            Self::Database(msg) => {
                tracing::error!("Gmail OAuth database error: {}", msg);
                (StatusCode::INTERNAL_SERVER_ERROR, "Internal error")
            }
            Self::Authorization(msg) => {
                tracing::warn!("Gmail OAuth authorization denied: {}", msg);
                (StatusCode::FORBIDDEN, "Access denied")
            }
        };

        (status, message).into_response()
    }
}

/// Query parameters for starting Gmail OAuth.
#[derive(Debug, Deserialize)]
pub struct GmailStartQuery {
    /// The integration account ID to authenticate.
    pub integration_id: String,
}

/// Query parameters for Gmail OAuth callback.
#[derive(Debug, Deserialize)]
pub struct GmailCallbackQuery {
    /// Authorization code from Google.
    pub code: String,
    /// CSRF state token.
    pub state: String,
}

/// State stored during OAuth flow (includes integration ID).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct GmailOAuthStateData {
    csrf_token: String,
    pkce_verifier: String,
    integration_id: String,
}

/// Shared state for Gmail OAuth routes.
#[derive(Clone)]
pub struct GmailOAuthState {
    pub oauth_client: GmailOAuthClient,
    pub db_pool: sqlx::PgPool,
    pub secure_cookies: bool,
}

/// Initiates Gmail OAuth flow for an integration.
pub async fn gmail_start(
    State(state): State<GmailOAuthState>,
    Query(query): Query<GmailStartQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, GmailOAuthError> {
    // Generate authorization URL
    let (auth_url, auth_state) = state.oauth_client.authorization_url();

    // Store state with integration ID in cookie
    let state_data = GmailOAuthStateData {
        csrf_token: auth_state.csrf_token,
        pkce_verifier: auth_state.pkce_verifier,
        integration_id: query.integration_id,
    };

    let state_json = serde_json::to_string(&state_data).expect("serialize Gmail OAuth state");

    let cookie = Cookie::build((GMAIL_AUTH_STATE_COOKIE, state_json))
        .path("/")
        .http_only(true)
        .secure(state.secure_cookies)
        .same_site(SameSite::Lax)
        .max_age(TimeDuration::minutes(10));

    Ok((jar.add(cookie), Redirect::to(&auth_url)))
}

/// Handles Gmail OAuth callback from Google.
pub async fn gmail_callback(
    State(state): State<GmailOAuthState>,
    Query(query): Query<GmailCallbackQuery>,
    jar: CookieJar,
) -> Result<impl IntoResponse, GmailOAuthError> {
    use crate::db::{IntegrationAccountRepository, IntegrationConfigRepository};
    use silver_telegram_core::IntegrationAccountId;
    use std::str::FromStr;

    // Get and validate state from cookie
    let state_cookie = jar
        .get(GMAIL_AUTH_STATE_COOKIE)
        .ok_or(GmailOAuthError::MissingAuthState)?;

    let state_data: GmailOAuthStateData = serde_json::from_str(state_cookie.value())
        .map_err(|_| GmailOAuthError::MissingAuthState)?;

    // Validate CSRF token
    if query.state != state_data.csrf_token {
        return Err(GmailOAuthError::CsrfMismatch);
    }

    // Exchange code for tokens
    let tokens = state
        .oauth_client
        .exchange_code(&query.code, &state_data.pkce_verifier)
        .await?;

    // Parse integration ID
    let integration_id = IntegrationAccountId::from_str(&state_data.integration_id)
        .map_err(|_| GmailOAuthError::InvalidIntegration)?;

    // Update integration config with OAuth tokens
    let config_data = serde_json::json!({
        "oauth_configured": true,
        "has_refresh_token": tokens.refresh_token.is_some(),
    });

    let config_repo = IntegrationConfigRepository::new(state.db_pool.clone());
    config_repo
        .upsert(integration_id, config_data)
        .await
        .map_err(|e| GmailOAuthError::Database(e.to_string()))?;

    // Update integration status to connected
    let account_repo = IntegrationAccountRepository::new(state.db_pool.clone());
    let mut account = account_repo
        .find_by_id(integration_id)
        .await
        .map_err(|e| GmailOAuthError::Database(e.to_string()))?
        .ok_or(GmailOAuthError::InvalidIntegration)?;

    account.mark_connected();
    account_repo
        .update(&account)
        .await
        .map_err(|e| GmailOAuthError::Database(e.to_string()))?;

    // Store tokens securely (in a real implementation, these would be encrypted)
    // For now, we store them in the integration config
    // TODO: Use proper credential vault with encryption
    let token_config = serde_json::json!({
        "oauth_configured": true,
        "access_token": tokens.access_token,
        "refresh_token": tokens.refresh_token,
        "expires_in_seconds": tokens.expires_in_seconds,
        "authenticated_at": chrono::Utc::now().to_rfc3339(),
    });

    config_repo
        .upsert(integration_id, token_config)
        .await
        .map_err(|e| GmailOAuthError::Database(e.to_string()))?;

    // Clear the state cookie
    let remove_state = Cookie::build((GMAIL_AUTH_STATE_COOKIE, ""))
        .path("/")
        .max_age(TimeDuration::ZERO);

    // Redirect back to integrations page
    Ok((jar.add(remove_state), Redirect::to("/integrations")))
}
