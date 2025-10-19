use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Redirect, Response},
    routing::get,
    Router,
};
use chrono::Utc;
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
use crate::services::oauth::youtube;

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
        .map_err(AuthError::OAuthError)?;

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

    // Store OAuth session with plaintext tokens
    // Note: Database encryption at rest is recommended for production
    OAuthSession::create(
        &state.pool,
        CreateSessionData {
            member_id: member.id,
            access_token: token_data.access_token.into_bytes(),
            refresh_token: token_data.refresh_token.map(|rt| rt.into_bytes()),
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

    // Redirect to issuers page
    Ok(Redirect::to("/issuers"))
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

/// Shows the home/login page
async fn home_page(session: Session) -> Result<axum::response::Html<String>, AuthError> {
    // Check if user is already logged in
    let member_id: Option<uuid::Uuid> = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|e| AuthError::SessionError(e.to_string()))?;

    let html = if let Some(_member_id) = member_id {
        // User is logged in - show dashboard with TE design
        r#"<!DOCTYPE html>
<html>
<head>
    <title>VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            display: flex;
            align-items: center;
            justify-content: center;
            padding: 20px;
        }
        .container {
            max-width: 600px;
            width: 100%;
        }
        h1 {
            font-size: 72px;
            font-weight: 300;
            letter-spacing: -2px;
            margin-bottom: 8px;
            color: #1E3A5F;
        }
        .subtitle {
            font-size: 14px;
            font-weight: 400;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
            margin-bottom: 60px;
        }
        .status-bar {
            background: #B8915F;
            color: #000;
            padding: 12px 16px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            margin-bottom: 40px;
            display: inline-block;
        }
        .menu {
            display: flex;
            flex-direction: column;
            gap: 2px;
            margin-bottom: 60px;
        }
        .button {
            background: #F5F3ED;
            color: #000;
            padding: 20px 24px;
            text-decoration: none;
            font-size: 16px;
            font-weight: 500;
            text-align: left;
            transition: background 0.2s, color 0.2s;
            border: none;
            cursor: pointer;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 1px 3px rgba(0,0,0,0.1);
        }
        .button:hover {
            background: #1E3A5F;
            color: #fff;
        }
        .button::after {
            content: '→';
            font-size: 20px;
        }
        .button.danger {
            background: #D5D3CD;
            color: #666;
        }
        .button.danger:hover {
            background: #FF5722;
            color: #fff;
        }
        .footer {
            font-size: 11px;
            color: #999;
            text-align: center;
            margin-top: 40px;
        }
    </style>
</head>
<body>
    <div class="container">
        <h1>VPass</h1>
        <p class="subtitle">Membership System</p>
        <div class="status-bar">● Connected</div>
        <div class="menu">
            <a href="/issuers" class="button">Browse Channels</a>
            <a href="/cards/my-cards" class="button">My Cards</a>
            <a href="/auth/logout" class="button danger">Sign Out</a>
        </div>
        <div class="footer">OP-1 inspired design</div>
    </div>
</body>
</html>"#
            .to_string()
    } else {
        // User is not logged in - show login page with TE design
        r#"<!DOCTYPE html>
<html>
<head>
    <title>VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }
        body {
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }
        .container {
            max-width: 900px;
            margin: 60px auto;
        }
        .hero {
            margin-bottom: 80px;
        }
        h1 {
            font-size: 96px;
            font-weight: 300;
            letter-spacing: -3px;
            margin-bottom: 12px;
            color: #1E3A5F;
            line-height: 0.9;
        }
        .subtitle {
            font-size: 14px;
            font-weight: 400;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
        }
        .description {
            background: #F5F3ED;
            padding: 40px;
            margin: 60px 0;
            border-left: 4px solid #1E3A5F;
            box-shadow: 0 2px 8px rgba(0,0,0,0.08);
        }
        .description h2 {
            font-size: 24px;
            font-weight: 500;
            margin-bottom: 20px;
            color: #1E3A5F;
        }
        .description p {
            font-size: 16px;
            line-height: 1.6;
            color: #444;
            margin-bottom: 24px;
        }
        .features {
            display: grid;
            grid-template-columns: repeat(3, 1fr);
            gap: 2px;
            margin-bottom: 60px;
        }
        .feature {
            background: #F5F3ED;
            padding: 32px 24px;
            text-align: center;
            box-shadow: 0 1px 4px rgba(0,0,0,0.08);
        }
        .feature:nth-child(1) { border-top: 3px solid #1E3A5F; }
        .feature:nth-child(2) { border-top: 3px solid #B8915F; }
        .feature:nth-child(3) { border-top: 3px solid #8C8C88; }
        .feature-icon {
            font-size: 32px;
            margin-bottom: 16px;
        }
        .feature h3 {
            font-size: 12px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1.5px;
            margin-bottom: 12px;
            color: #000;
        }
        .feature:nth-child(1) h3 { color: #1E3A5F; }
        .feature:nth-child(2) h3 { color: #B8915F; }
        .feature:nth-child(3) h3 { color: #8C8C88; }
        .feature p {
            font-size: 13px;
            color: #666;
            line-height: 1.5;
        }
        .cta {
            text-align: center;
            margin: 80px 0;
        }
        .login-button {
            display: inline-block;
            background: #1E3A5F;
            color: #fff;
            padding: 24px 48px;
            text-decoration: none;
            font-size: 16px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            transition: all 0.2s;
            border: 2px solid #1E3A5F;
            box-shadow: 0 2px 8px rgba(0,0,0,0.15);
        }
        .login-button:hover {
            background: #B8915F;
            color: #000;
            border-color: #B8915F;
        }
        .footer {
            text-align: center;
            font-size: 11px;
            color: #999;
            margin-top: 100px;
            padding-top: 40px;
            border-top: 1px solid #333;
        }
        @media (max-width: 768px) {
            h1 { font-size: 56px; }
            .features { grid-template-columns: 1fr; }
            .description { padding: 24px; }
        }
    </style>
</head>
<body>
    <div class="container">
        <div class="hero">
            <h1>VPass</h1>
            <p class="subtitle">Digital Membership Cards</p>
        </div>

        <div class="description">
            <h2>Verify Your Membership</h2>
            <p>A secure system for YouTube channel members to claim verifiable digital membership cards for offline events.</p>
        </div>

        <div class="cta">
            <a href="/auth/youtube/login" class="login-button">Connect YouTube</a>
        </div>

        <div class="footer">
            OP-1 inspired design · Built with Rust + Axum
        </div>
    </div>
</body>
</html>"#.to_string()
    };

    Ok(axum::response::Html(html))
}

/// Creates the auth router
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(home_page))
        .route("/auth/youtube/login", get(youtube_login))
        .route("/auth/youtube/callback", get(youtube_callback))
        .route("/auth/logout", get(logout))
}
