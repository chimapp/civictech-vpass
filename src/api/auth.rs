use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use chrono::Utc;
use secrecy::ExposeSecret;
use serde::Deserialize;
use tower_sessions::Session;

use crate::api::middleware::session::{
    AppState, SESSION_KEY_CSRF_TOKEN, SESSION_KEY_MEMBER_ID, SESSION_KEY_PKCE_VERIFIER,
    SESSION_KEY_SESSION_STARTED_AT,
};
use crate::models::{
    member::{CreateMemberData, Member},
    oauth_session::{CreateSessionData, OAuthSession},
};
use crate::services::{encryption, oauth::youtube};

#[derive(Debug)]
pub enum AuthError {
    OAuthError(String),
    DatabaseError(sqlx::Error),
    SessionError(String),
    EncryptionError(String),
    CsrfMismatch,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AuthError::OAuthError(msg) => {
                (StatusCode::BAD_REQUEST, format!("OAuth error: {}", msg))
            }
            AuthError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            AuthError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
            AuthError::EncryptionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Encryption error: {}", msg),
            ),
            AuthError::CsrfMismatch => (StatusCode::BAD_REQUEST, "CSRF token mismatch".to_string()),
        };

        (status, message).into_response()
    }
}

/// Initiates YouTube OAuth flow
async fn youtube_login(
    State(state): State<AppState>,
    session: Session,
) -> Result<Redirect, AuthError> {
    let redirect_uri = format!("{}/auth/youtube/callback", state.config.base_url);

    // Build OAuth URL
    let (auth_url, csrf_token, pkce_verifier) = youtube::build_auth_url(
        &state.config.youtube_client_id,
        &state.config.youtube_client_secret,
        &redirect_uri,
    )
    .map_err(|e| AuthError::OAuthError(e.to_string()))?;

    // Store CSRF token and PKCE verifier in session
    session
        .insert(SESSION_KEY_CSRF_TOKEN, csrf_token)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    session
        .insert(SESSION_KEY_PKCE_VERIFIER, pkce_verifier)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    // Store when the session started (for comment verification)
    session
        .insert(SESSION_KEY_SESSION_STARTED_AT, Utc::now().to_rfc3339())
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    tracing::info!("Redirecting to YouTube OAuth");

    Ok(Redirect::to(&auth_url))
}

#[derive(Deserialize)]
struct OAuthCallback {
    code: String,
    state: String,
}

/// Handles OAuth callback from YouTube
async fn youtube_callback(
    State(state): State<AppState>,
    Query(params): Query<OAuthCallback>,
    session: Session,
) -> Result<Redirect, AuthError> {
    // Verify CSRF token
    let stored_csrf: Option<String> = session
        .get(SESSION_KEY_CSRF_TOKEN)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    if stored_csrf.as_ref() != Some(&params.state) {
        return Err(AuthError::CsrfMismatch);
    }

    // Get PKCE verifier
    let pkce_verifier: Option<String> = session
        .get(SESSION_KEY_PKCE_VERIFIER)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    let redirect_uri = format!("{}/auth/youtube/callback", state.config.base_url);

    // Exchange code for tokens
    let token_data = youtube::exchange_code(
        &params.code,
        &state.config.youtube_client_id,
        &state.config.youtube_client_secret,
        &redirect_uri,
        pkce_verifier.as_deref(),
    )
    .await
    .map_err(|e| AuthError::OAuthError(e.to_string()))?;

    tracing::info!("Successfully exchanged OAuth code for tokens");

    // Get user info from YouTube to get channel ID
    let user_info = get_youtube_user_info(&token_data.access_token)
        .await
        .map_err(|e| AuthError::OAuthError(e))?;

    // Create or find member
    let member = Member::find_or_create(
        &state.pool,
        CreateMemberData {
            youtube_user_id: user_info.channel_id.clone(),
            default_display_name: user_info.display_name.clone(),
            avatar_url: user_info.avatar_url.clone(),
            locale: None,
        },
    )
    .await
    .map_err(AuthError::DatabaseError)?;

    // Encrypt tokens
    let encryption_key = encryption::derive_key(state.config.encryption_key.expose_secret());
    let encrypted_access_token = encryption::encrypt(&token_data.access_token, &encryption_key)
        .map_err(|e| AuthError::EncryptionError(e.to_string()))?;

    let encrypted_refresh_token = token_data
        .refresh_token
        .as_ref()
        .map(|rt| encryption::encrypt(rt, &encryption_key))
        .transpose()
        .map_err(|e| AuthError::EncryptionError(e.to_string()))?;

    // Store OAuth session
    OAuthSession::create(
        &state.pool,
        CreateSessionData {
            member_id: member.id,
            access_token: encrypted_access_token,
            refresh_token: encrypted_refresh_token,
            token_scope: token_data.scopes.join(" "),
            token_expires_at: token_data.expires_at,
        },
    )
    .await
    .map_err(AuthError::DatabaseError)?;

    // Store member ID in session
    session
        .insert(SESSION_KEY_MEMBER_ID, member.id)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    tracing::info!(member_id = %member.id, "Member authenticated successfully");

    // Redirect to claim page
    Ok(Redirect::to("/cards/claim"))
}

/// Logs out the user
async fn logout(session: Session) -> Result<Redirect, AuthError> {
    session
        .flush()
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    Ok(Redirect::to("/"))
}

#[derive(Deserialize)]
struct YouTubeUserInfo {
    #[serde(rename = "id")]
    channel_id: String,
    snippet: YouTubeSnippet,
}

#[derive(Deserialize)]
struct YouTubeSnippet {
    title: String,
    thumbnails: YouTubeThumbnails,
}

#[derive(Deserialize)]
struct YouTubeThumbnails {
    default: YouTubeThumbnail,
}

#[derive(Deserialize)]
struct YouTubeThumbnail {
    url: String,
}

struct UserInfo {
    channel_id: String,
    display_name: String,
    avatar_url: Option<String>,
}

/// Fetches user info from YouTube API
async fn get_youtube_user_info(access_token: &str) -> Result<UserInfo, String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://www.googleapis.com/youtube/v3/channels?part=snippet&mine=true")
        .header("Authorization", format!("Bearer {}", access_token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !response.status().is_success() {
        return Err(format!("YouTube API error: {}", response.status()));
    }

    #[derive(Deserialize)]
    struct ChannelsResponse {
        items: Vec<YouTubeUserInfo>,
    }

    let channels: ChannelsResponse = response.json().await.map_err(|e| e.to_string())?;

    let channel = channels.items.first().ok_or("No channel found")?;

    Ok(UserInfo {
        channel_id: channel.channel_id.clone(),
        display_name: channel.snippet.title.clone(),
        avatar_url: Some(channel.snippet.thumbnails.default.url.clone()),
    })
}

/// Creates the auth router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/auth/youtube/login", get(youtube_login))
        .route("/auth/youtube/callback", get(youtube_callback))
        .route("/auth/logout", get(logout))
}
