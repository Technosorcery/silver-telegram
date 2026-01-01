//! OIDC client implementation using the openidconnect crate.

use openidconnect::core::{CoreAuthenticationFlow, CoreProviderMetadata};
use openidconnect::{
    AuthorizationCode, ClientId, ClientSecret, CsrfToken, IssuerUrl, Nonce, OAuth2TokenResponse,
    PkceCodeChallenge, PkceCodeVerifier, RedirectUrl, Scope, TokenResponse,
};
use silver_telegram_platform_access::{OidcClaims, OidcConfig};

/// OIDC client for authenticating users.
pub struct OidcClient {
    provider_metadata: CoreProviderMetadata,
    client_id: ClientId,
    client_secret: ClientSecret,
    redirect_url: RedirectUrl,
    config: OidcConfig,
}

/// Data needed to complete the OIDC callback.
#[derive(Debug, Clone)]
pub struct AuthState {
    pub csrf_token: String,
    pub pkce_verifier: String,
    pub nonce: String,
}

/// Result of a successful token exchange.
pub struct TokenResult {
    pub claims: OidcClaims,
    pub access_token: String,
    pub refresh_token: Option<String>,
}

impl OidcClient {
    /// Creates a new OIDC client by discovering the provider metadata.
    pub async fn discover(config: OidcConfig) -> Result<Self, OidcError> {
        let issuer_url = IssuerUrl::new(config.issuer_url().to_string())
            .map_err(|e| OidcError::Configuration(format!("invalid issuer URL: {}", e)))?;

        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| {
                OidcError::Configuration(format!("failed to create HTTP client: {}", e))
            })?;

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &http_client)
            .await
            .map_err(|e| OidcError::Discovery(format!("failed to discover provider: {}", e)))?;

        let redirect_url = RedirectUrl::new(config.redirect_uri().to_string())
            .map_err(|e| OidcError::Configuration(format!("invalid redirect URI: {}", e)))?;

        let client_id = ClientId::new(config.client_id().to_string());
        let client_secret = ClientSecret::new(config.client_secret().to_string());

        Ok(Self {
            provider_metadata,
            client_id,
            client_secret,
            redirect_url,
            config,
        })
    }

    /// Generates the authorization URL for redirecting the user.
    pub fn authorization_url(&self) -> (String, AuthState) {
        use openidconnect::core::CoreClient;

        let client = CoreClient::from_provider_metadata(
            self.provider_metadata.clone(),
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(self.redirect_url.clone());

        let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

        let mut auth_request = client
            .authorize_url(
                CoreAuthenticationFlow::AuthorizationCode,
                CsrfToken::new_random,
                Nonce::new_random,
            )
            .set_pkce_challenge(pkce_challenge);

        // Add configured scopes
        for scope in self.config.scopes() {
            auth_request = auth_request.add_scope(Scope::new(scope.to_string()));
        }

        let (auth_url, csrf_token, nonce) = auth_request.url();

        let state = AuthState {
            csrf_token: csrf_token.secret().clone(),
            pkce_verifier: pkce_verifier.secret().clone(),
            nonce: nonce.secret().clone(),
        };

        (auth_url.to_string(), state)
    }

    /// Exchanges the authorization code for tokens and extracts claims.
    pub async fn exchange_code(
        &self,
        code: &str,
        state: &AuthState,
    ) -> Result<TokenResult, OidcError> {
        use openidconnect::core::CoreClient;

        let client = CoreClient::from_provider_metadata(
            self.provider_metadata.clone(),
            self.client_id.clone(),
            Some(self.client_secret.clone()),
        )
        .set_redirect_uri(self.redirect_url.clone());

        let pkce_verifier = PkceCodeVerifier::new(state.pkce_verifier.clone());

        let http_client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .map_err(|e| {
                OidcError::TokenExchange(format!("failed to create HTTP client: {}", e))
            })?;

        let token_request = client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .map_err(|e| OidcError::TokenExchange(format!("token endpoint error: {}", e)))?;

        let token_response = token_request
            .set_pkce_verifier(pkce_verifier)
            .request_async(&http_client)
            .await
            .map_err(|e| OidcError::TokenExchange(format!("token exchange failed: {}", e)))?;

        // Extract the ID token
        let id_token = token_response
            .id_token()
            .ok_or_else(|| OidcError::TokenExchange("no ID token in response".to_string()))?;

        // Verify and extract claims
        let nonce = Nonce::new(state.nonce.clone());
        let claims = id_token
            .claims(&client.id_token_verifier(), &nonce)
            .map_err(|e| {
                OidcError::TokenValidation(format!("ID token validation failed: {}", e))
            })?;

        // Extract standard claims
        let subject = claims.subject().to_string();
        let issuer = claims.issuer().to_string();
        let email: Option<String> = claims.email().map(|e| e.as_str().to_string());
        let display_name: Option<String> = claims
            .name()
            .and_then(|n| n.get(None))
            .map(|n| n.as_str().to_string())
            .or_else(|| claims.preferred_username().map(|u| u.as_str().to_string()));

        // Extract groups from the ID token using the configured groups claim name
        // Parse the raw JWT payload to access custom claims that aren't in the standard set
        let groups = self.extract_groups_from_token_response(&token_response)?;

        let oidc_claims = OidcClaims::new(subject, issuer)
            .with_email(email)
            .with_display_name(display_name)
            .with_groups(groups);

        Ok(TokenResult {
            claims: oidc_claims,
            access_token: token_response.access_token().secret().clone(),
            refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
        })
    }

    /// Returns the configuration.
    pub fn config(&self) -> &OidcConfig {
        &self.config
    }

    /// Extracts groups from a token response using the configured groups claim.
    ///
    /// Different OIDC providers use different claim names for groups:
    /// - "groups" (common default)
    /// - "cognito:groups" (AWS Cognito)
    /// - "roles" (some providers)
    ///
    /// This method extracts the raw ID token JWT and parses it to get groups.
    fn extract_groups_from_token_response<TR>(
        &self,
        token_response: &TR,
    ) -> Result<Vec<String>, OidcError>
    where
        TR: serde::Serialize,
    {
        // Get the id_token from the response by serializing to JSON
        // The token response includes the raw id_token string
        let response_json = serde_json::to_value(token_response).map_err(|e| {
            OidcError::TokenValidation(format!("Failed to serialize token response: {}", e))
        })?;

        let id_token_str = response_json
            .get("id_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| OidcError::TokenValidation("No id_token in response".to_string()))?;

        // JWT is base64url(header).base64url(payload).signature
        let parts: Vec<&str> = id_token_str.split('.').collect();
        if parts.len() != 3 {
            return Err(OidcError::TokenValidation("Invalid JWT format".to_string()));
        }

        // Decode the payload (second part)
        use base64::Engine;
        let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(parts[1])
            .map_err(|e| {
                OidcError::TokenValidation(format!("Failed to decode JWT payload: {}", e))
            })?;

        // Parse as JSON
        let payload: serde_json::Value = serde_json::from_slice(&payload_bytes).map_err(|e| {
            OidcError::TokenValidation(format!("Failed to parse JWT payload: {}", e))
        })?;

        // Extract groups from the configured claim name
        let groups_claim = self.config.groups_claim();
        let groups = payload
            .get(groups_claim)
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        Ok(groups)
    }
}

/// OIDC-related errors.
#[derive(Debug)]
pub enum OidcError {
    /// Configuration error (invalid URLs, etc.)
    Configuration(String),
    /// Failed to discover provider metadata.
    Discovery(String),
    /// Token exchange failed.
    TokenExchange(String),
    /// Token validation failed.
    TokenValidation(String),
}

impl std::fmt::Display for OidcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(msg) => write!(f, "OIDC configuration error: {}", msg),
            Self::Discovery(msg) => write!(f, "OIDC discovery error: {}", msg),
            Self::TokenExchange(msg) => write!(f, "OIDC token exchange error: {}", msg),
            Self::TokenValidation(msg) => write!(f, "OIDC token validation error: {}", msg),
        }
    }
}

impl std::error::Error for OidcError {}
