use chrono::{DateTime, Duration, Utc};
use oauth2::reqwest::async_http_client;
use oauth2::{
    basic::BasicClient, AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken,
    PkceCodeChallenge, RedirectUrl, Scope, TokenResponse as OAuth2TokenResponse, TokenUrl,
};
use secrecy::{ExposeSecret, Secret};
use serde::{Deserialize, Serialize};

#[derive(thiserror::Error, Debug)]
pub enum YouTubeOAuthError {
    #[error("OAuth URL construction failed: {0}")]
    UrlConstruction(String),

    #[error("Token exchange failed: {0}")]
    TokenExchange(String),

    #[error("Token refresh failed: {0}")]
    TokenRefresh(String),

    #[error("Invalid redirect URI: {0}")]
    InvalidRedirectUri(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenData {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub scopes: Vec<String>,
}

/// YouTube OAuth scopes needed for membership verification
/// Note: youtube.force-ssl is required for accessing comments via the API.
/// Despite being a read operation, youtube.readonly is insufficient and returns 403.
pub const YOUTUBE_FORCE_SSL_SCOPE: &str = "https://www.googleapis.com/auth/youtube.force-ssl";

/// Builds the YouTube OAuth client
fn build_oauth_client(
    client_id: &str,
    client_secret: &Secret<String>,
    redirect_uri: &str,
) -> Result<BasicClient, YouTubeOAuthError> {
    let redirect_url = RedirectUrl::new(redirect_uri.to_string())
        .map_err(|e| YouTubeOAuthError::InvalidRedirectUri(e.to_string()))?;

    let client = BasicClient::new(
        ClientId::new(client_id.to_string()),
        Some(ClientSecret::new(client_secret.expose_secret().clone())),
        AuthUrl::new("https://accounts.google.com/o/oauth2/v2/auth".to_string())
            .map_err(|e| YouTubeOAuthError::UrlConstruction(e.to_string()))?,
        Some(
            TokenUrl::new("https://oauth2.googleapis.com/token".to_string())
                .map_err(|e| YouTubeOAuthError::UrlConstruction(e.to_string()))?,
        ),
    )
    .set_redirect_uri(redirect_url);

    Ok(client)
}

/// Generates the authorization URL for YouTube OAuth
/// Returns (auth_url, csrf_token, pkce_verifier)
pub fn build_auth_url(
    client_id: &str,
    client_secret: &Secret<String>,
    redirect_uri: &str,
) -> Result<(String, String, String), YouTubeOAuthError> {
    let client = build_oauth_client(client_id, client_secret, redirect_uri)?;

    // Generate PKCE challenge for additional security
    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new(YOUTUBE_FORCE_SSL_SCOPE.to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    Ok((
        auth_url.to_string(),
        csrf_token.secret().clone(),
        pkce_verifier.secret().clone(),
    ))
}

/// Exchanges an authorization code for access and refresh tokens
pub async fn exchange_code(
    code: &str,
    client_id: &str,
    client_secret: &Secret<String>,
    redirect_uri: &str,
    pkce_verifier: Option<&str>,
) -> Result<TokenData, YouTubeOAuthError> {
    let client = build_oauth_client(client_id, client_secret, redirect_uri)?;

    let mut token_request = client.exchange_code(AuthorizationCode::new(code.to_string()));

    // Add PKCE verifier if provided
    if let Some(verifier) = pkce_verifier {
        use oauth2::PkceCodeVerifier;
        token_request =
            token_request.set_pkce_verifier(PkceCodeVerifier::new(verifier.to_string()));
    }

    let token_response = token_request
        .request_async(async_http_client)
        .await
        .map_err(|e| YouTubeOAuthError::TokenExchange(e.to_string()))?;

    let expires_in = token_response
        .expires_in()
        .unwrap_or(std::time::Duration::from_secs(3600));

    let expires_at = Utc::now() + Duration::seconds(expires_in.as_secs() as i64);

    let scopes = token_response
        .scopes()
        .map(|s| s.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_else(|| vec![YOUTUBE_FORCE_SSL_SCOPE.to_string()]);

    Ok(TokenData {
        access_token: token_response.access_token().secret().clone(),
        refresh_token: token_response.refresh_token().map(|t| t.secret().clone()),
        expires_at,
        scopes,
    })
}

/// Refreshes an access token using a refresh token
pub async fn refresh_access_token(
    refresh_token: &str,
    client_id: &str,
    client_secret: &Secret<String>,
    redirect_uri: &str,
) -> Result<TokenData, YouTubeOAuthError> {
    let client = build_oauth_client(client_id, client_secret, redirect_uri)?;

    use oauth2::RefreshToken;
    let token_response = client
        .exchange_refresh_token(&RefreshToken::new(refresh_token.to_string()))
        .request_async(async_http_client)
        .await
        .map_err(|e| YouTubeOAuthError::TokenRefresh(e.to_string()))?;

    let expires_in = token_response
        .expires_in()
        .unwrap_or(std::time::Duration::from_secs(3600));

    let expires_at = Utc::now() + Duration::seconds(expires_in.as_secs() as i64);

    let scopes = token_response
        .scopes()
        .map(|s| s.iter().map(|scope| scope.to_string()).collect())
        .unwrap_or_else(|| vec![YOUTUBE_FORCE_SSL_SCOPE.to_string()]);

    Ok(TokenData {
        access_token: token_response.access_token().secret().clone(),
        // Refresh token may or may not be returned; keep the old one if not
        refresh_token: token_response
            .refresh_token()
            .map(|t| t.secret().clone())
            .or_else(|| Some(refresh_token.to_string())),
        expires_at,
        scopes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_auth_url() {
        let client_id = "test-client-id";
        let client_secret = Secret::new("test-secret".to_string());
        let redirect_uri = "http://localhost:3000/auth/youtube/callback";

        let result = build_auth_url(client_id, &client_secret, redirect_uri);
        assert!(result.is_ok());

        let (auth_url, csrf_token, pkce_verifier) = result.unwrap();

        // Verify the URL contains expected components
        assert!(auth_url.contains("accounts.google.com"));
        assert!(auth_url.contains("client_id=test-client-id"));
        assert!(auth_url.contains("redirect_uri="));
        assert!(auth_url.contains("youtube.force-ssl"));

        // Verify CSRF token and PKCE verifier are generated
        assert!(!csrf_token.is_empty());
        assert!(!pkce_verifier.is_empty());
    }

    #[test]
    fn test_invalid_redirect_uri() {
        let client_id = "test-client-id";
        let client_secret = Secret::new("test-secret".to_string());
        let invalid_uri = "not a valid uri!!!";

        let result = build_auth_url(client_id, &client_secret, invalid_uri);
        assert!(result.is_err());
    }
}
