use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    middleware,
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Router,
};
use chrono::{DateTime, Utc};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::api::middleware::{
    auth::{get_authenticated_member, require_auth, AuthError},
    session::{AppState, SESSION_KEY_SESSION_STARTED_AT},
};
use crate::models::{
    card::MembershipCard, issuer::CardIssuer, oauth_session::OAuthSession,
    wallet_qr_code::WalletQrCode,
};
use crate::services::{card_issuer, wallet_qr};

#[derive(Debug)]
pub enum CardsError {
    AuthError(AuthError),
    DatabaseError(sqlx::Error),
    IssuanceError(card_issuer::CardIssuanceError),
    SessionError(String),
    NotFound,
    WalletQrError(wallet_qr::WalletQrError),
}

impl IntoResponse for CardsError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            CardsError::AuthError(e) => return e.into_response(),
            CardsError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            CardsError::IssuanceError(e) => {
                (StatusCode::BAD_REQUEST, format!("Issuance error: {}", e))
            }
            CardsError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
            CardsError::NotFound => (StatusCode::NOT_FOUND, "Card not found".to_string()),
            CardsError::WalletQrError(e) => {
                // Special handling for CredentialNotReady
                if matches!(e, wallet_qr::WalletQrError::CredentialNotReady) {
                    return (StatusCode::ACCEPTED, "Credential not ready yet").into_response();
                }
                (StatusCode::BAD_REQUEST, format!("Wallet QR error: {}", e))
            }
        };

        (status, message).into_response()
    }
}

/// Shows the claim card page for a specific channel/issuer
async fn claim_page_for_channel(
    State(state): State<AppState>,
    Path(issuer_id): Path<Uuid>,
    session: Session,
) -> Result<Html<String>, CardsError> {
    let _member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    // Fetch the issuer to display channel information
    let issuer = CardIssuer::find_by_id(&state.pool, issuer_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::NotFound)?;

    let html = format!(
        r#"
<!DOCTYPE html>
<html>
<head>
    <title>Claim Card - {} - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 700px;
            margin: 40px auto;
        }}
        .header {{
            margin-bottom: 60px;
        }}
        h1 {{
            font-size: 48px;
            font-weight: 300;
            letter-spacing: -1px;
            color: #1E3A5F;
            margin-bottom: 8px;
        }}
        .subtitle {{
            font-size: 14px;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
        }}
        .back {{
            display: inline-block;
            color: #666;
            text-decoration: none;
            font-size: 13px;
            margin-bottom: 40px;
            transition: color 0.2s;
        }}
        .back:hover {{ color: #1E3A5F; }}
        .back::before {{ content: '← '; }}
        .channel-info {{
            background: #F5F3ED;
            padding: 32px;
            margin-bottom: 40px;
            border-left: 3px solid #1E3A5F;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .channel-info h2 {{
            font-size: 24px;
            font-weight: 500;
            color: #1E3A5F;
            margin-bottom: 16px;
        }}
        .channel-info .channel-id {{
            font-size: 12px;
            font-family: 'Courier New', monospace;
            color: #666;
        }}
        .info-box {{
            background: #F5F3ED;
            padding: 32px;
            margin-bottom: 40px;
            border-left: 3px solid #B8915F;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .info-box h2 {{
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #B8915F;
            margin-bottom: 20px;
        }}
        .info-box ol {{
            margin-left: 20px;
            line-height: 1.8;
            color: #444;
        }}
        .info-box li {{
            font-size: 14px;
            margin-bottom: 8px;
        }}
        form {{
            background: #F5F3ED;
            padding: 40px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .form-group {{
            margin-bottom: 32px;
        }}
        label {{
            display: block;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #666;
            margin-bottom: 12px;
        }}
        input {{
            width: 100%;
            padding: 16px;
            background: #fff;
            border: 1px solid #CCC;
            color: #000;
            font-size: 14px;
            font-family: 'Courier New', monospace;
            transition: border-color 0.2s;
        }}
        input:focus {{
            outline: none;
            border-color: #1E3A5F;
        }}
        input::placeholder {{
            color: #999;
        }}
        button {{
            width: 100%;
            padding: 20px;
            background: #B8915F;
            color: #000;
            border: none;
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            cursor: pointer;
            transition: all 0.2s;
            box-shadow: 0 2px 4px rgba(0,0,0,0.15);
        }}
        button:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/issuers" class="back">Back to Channels</a>
        <div class="header">
            <h1>Claim Card</h1>
            <p class="subtitle">Membership Verification</p>
        </div>

        <div class="channel-info">
            <h2>{}</h2>
            <p class="channel-id">{}</p>
        </div>

        <div class="info-box">
            <h2>Instructions</h2>
            <ol>
                <li>Go to the channel's members-only verification video</li>
                <li>Post a comment (any text is fine)</li>
                <li>Copy the comment URL or ID</li>
                <li>Paste it below and submit</li>
            </ol>
        </div>

        <form action="/channels/{}/claim" method="POST">
            <div class="form-group">
                <label for="comment_link">Comment URL or ID</label>
                <input type="text" name="comment_link" id="comment_link" required
                       placeholder="https://www.youtube.com/watch?v=...&lc=...">
            </div>
            <button type="submit">Issue Card</button>
        </form>
    </div>
</body>
</html>
    "#,
        issuer.channel_name,
        issuer.channel_name,
        issuer
            .channel_handle
            .as_deref()
            .unwrap_or(&issuer.youtube_channel_id),
        issuer_id
    );

    Ok(Html(html))
}

#[derive(Deserialize)]
struct ClaimCardFormForChannel {
    comment_link: String,
}

async fn claim_card_for_channel(
    State(state): State<AppState>,
    Path(issuer_id): Path<Uuid>,
    session: Session,
    Form(form): Form<ClaimCardFormForChannel>,
) -> Result<Response, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let member_record = crate::models::member::Member::find_by_id(&state.pool, member.member_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::AuthError(
            AuthError::Unauthorized(String::new()),
        ))?;

    let mut oauth_session = OAuthSession::find_by_member_id(&state.pool, member.member_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::AuthError(
            AuthError::Unauthorized(String::new()),
        ))?;

    // Check if token is expired and refresh if needed
    if oauth_session.is_expired() {
        tracing::info!("Access token expired, attempting to refresh");

        let refresh_token = oauth_session
            .refresh_token
            .as_ref()
            .and_then(|t| String::from_utf8(t.clone()).ok())
            .ok_or(CardsError::SessionError(
                "No refresh token available".to_string(),
            ))?;

        let token_data = crate::services::oauth::youtube::refresh_access_token(
            &refresh_token,
            &state.config.youtube_client_id,
            &state.config.youtube_client_secret,
            &format!("{}/auth/youtube/callback", state.config.base_url),
        )
        .await
        .map_err(|e| CardsError::SessionError(format!("Token refresh failed: {}", e)))?;

        // Update the session with new tokens
        OAuthSession::update_tokens(
            &state.pool,
            oauth_session.id,
            token_data.access_token.as_bytes().to_vec(),
            token_data.refresh_token.map(|t| t.as_bytes().to_vec()),
            token_data.expires_at,
        )
        .await
        .map_err(CardsError::DatabaseError)?;

        tracing::info!("Access token refreshed successfully");

        // Update our local copy
        oauth_session.access_token = token_data.access_token.as_bytes().to_vec();
        oauth_session.token_expires_at = token_data.expires_at;
    }

    let access_token = String::from_utf8(oauth_session.access_token)
        .map_err(|_| CardsError::SessionError("Invalid access token encoding".to_string()))?;

    let session_started_str: String = session
        .get(SESSION_KEY_SESSION_STARTED_AT)
        .await
        .map_err(|e| CardsError::SessionError(e.to_string()))?
        .ok_or(CardsError::SessionError(
            "No session start time".to_string(),
        ))?;

    let session_started_at = DateTime::parse_from_rfc3339(&session_started_str)
        .map_err(|e| CardsError::SessionError(e.to_string()))?
        .with_timezone(&Utc);

    let signing_key = {
        use ring::digest;
        let hash = digest::digest(
            &digest::SHA256,
            state.config.session_secret.expose_secret().as_bytes(),
        );
        let mut key = [0u8; 32];
        key.copy_from_slice(hash.as_ref());
        key
    };

    // Prepare wallet API configuration if available
    let issuer_api_config = state.config.issuer_api_url.as_ref().and_then(|url| {
        state
            .config
            .issuer_access_token
            .as_ref()
            .map(|token| (url.as_str(), token.expose_secret().as_str()))
    });

    let result = card_issuer::issue_card(
        &state.pool,
        &signing_key,
        issuer_api_config,
        card_issuer::IssueCardRequest {
            issuer_id,
            member_youtube_user_id: member_record.youtube_user_id,
            member_display_name: member_record.default_display_name,
            member_avatar_url: member_record.avatar_url,
            comment_link_or_id: form.comment_link,
            session_started_at,
            access_token,
        },
    )
    .await
    .map_err(CardsError::IssuanceError)?;

    tracing::info!(card_id = %result.card.id, "Card issued successfully");

    Ok(axum::response::Redirect::to(&format!("/cards/{}", result.card.id)).into_response())
}

async fn show_card(
    State(state): State<AppState>,
    Path(card_id): Path<Uuid>,
    session: Session,
) -> Result<Html<String>, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let card = MembershipCard::find_by_id(&state.pool, card_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::NotFound)?;

    if card.member_id != member.member_id {
        return Err(CardsError::NotFound);
    }

    // Load the active wallet QR code for this card
    let wallet_qr =
        crate::models::wallet_qr_code::WalletQrCode::find_active_by_card_id(&state.pool, card.id)
            .await
            .map_err(CardsError::DatabaseError)?;

    let credential_status_block = wallet_qr
        .as_ref()
        .map(|qr| {
            let cid_present = qr.cid.is_some();
            let status_text = if cid_present {
                "Credential issued! ✓"
            } else {
                "Waiting for wallet scan..."
            };
            let instructions_text = if cid_present {
                "Credential is ready in your Taiwan Digital Wallet."
            } else {
                "Please scan the QR code with the Taiwan Digital Wallet app."
            };
            let poll_info = if cid_present {
                "Credential is ready."
            } else {
                "Checking status..."
            };
            let spinner_classes = if cid_present {
                "spinner-dot is-hidden"
            } else {
                "spinner-dot"
            };

            format!(
                r#"<section class="credential-status" data-credential-status data-poll-url="/cards/{card_id}/poll-credential" data-cid-present="{cid_present}" data-max-polls="150">
        <div class="status-indicator">
            <span class="{spinner_classes}" data-role="spinner"></span>
            <span class="status-text" data-role="status-text">{status_text}</span>
        </div>
        <div class="status-details">
            <p class="status-instructions">{instructions_text}</p>
            <p class="poll-info" data-role="poll-info">{poll_info}</p>
        </div>
    </section>"#,
                card_id = card.id,
                cid_present = cid_present,
                spinner_classes = spinner_classes,
                status_text = status_text,
                instructions_text = instructions_text,
                poll_info = poll_info,
            )
        })
        .unwrap_or_default();

    let qr_available = wallet_qr
        .as_ref()
        .map(|qr| qr.cid.is_none())
        .unwrap_or(false);

    let qr_markup = wallet_qr
        .as_ref()
        .map(|qr| {
            if qr.cid.is_none() {
                format!(
                    r#"<div class="qr-code">
                <img src="{}" alt="Taiwan Digital Wallet QR Code" />
            </div>
            <div class="scan-instruction">Scan with Taiwan Digital Wallet</div>"#,
                    qr.qr_code
                )
            } else {
                r#"<div class="qr-code placeholder">
                <div class="placeholder-icon">✓</div>
                <div class="placeholder-text">Credential already issued</div>
            </div>"#
                    .to_string()
            }
        })
        .unwrap_or_else(|| {
            r#"<div class="qr-code placeholder">
                <div class="placeholder-text">QR code not available</div>
            </div>"#
                .to_string()
        });

    let deep_link = wallet_qr
        .as_ref()
        .and_then(|qr| qr.deep_link.as_deref())
        .map(|link| html_escape::encode_double_quoted_attribute(link).to_string());

    let actions_markup = if qr_available {
        if let Some(link) = deep_link {
            format!(
                r#"<div class="actions">
            <a href="{}" class="button">Open in Taiwan Wallet App</a>
            <a href="/cards/my-cards" class="button secondary">View All Cards</a>
        </div>"#,
                link
            )
        } else {
            r#"<div class="actions">
            <a href="/cards/my-cards" class="button secondary">View All Cards</a>
        </div>"#
                .to_string()
        }
    } else {
        r#"<div class="actions">
            <a href="/cards/my-cards" class="button secondary">View All Cards</a>
        </div>"#
            .to_string()
    };

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>Card - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 700px;
            margin: 40px auto;
        }}
        .back {{
            display: inline-block;
            color: #666;
            text-decoration: none;
            font-size: 13px;
            margin-bottom: 40px;
            transition: color 0.2s;
        }}
        .back:hover {{ color: #1E3A5F; }}
        .back::before {{ content: '← '; }}
        .status {{
            background: #B8915F;
            color: #000;
            padding: 8px 16px;
            font-size: 11px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            display: inline-block;
            margin-bottom: 24px;
        }}
        .credential-status {{
            background: #F5F3ED;
            border-left: 4px solid #B8915F;
            padding: 24px;
            margin-bottom: 24px;
            box-shadow: 0 2px 6px rgba(0,0,0,0.08);
        }}
        .status-indicator {{
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 10px;
            font-size: 16px;
            font-weight: 500;
            color: #1E3A5F;
        }}
        .spinner-dot {{
            width: 16px;
            height: 16px;
            border-radius: 50%;
            background: #1E3A5F;
            position: relative;
            animation: pulse 1s ease-in-out infinite;
        }}
        .spinner-dot::after {{
            content: '';
            position: absolute;
            top: -6px;
            left: -6px;
            right: -6px;
            bottom: -6px;
            border-radius: 50%;
            border: 2px solid rgba(30,58,95,0.35);
        }}
        @keyframes pulse {{
            0% {{ transform: scale(1); opacity: 1; }}
            50% {{ transform: scale(1.25); opacity: 0.65; }}
            100% {{ transform: scale(1); opacity: 1; }}
        }}
        .status-text {{
            letter-spacing: -0.2px;
        }}
        .status-details {{
            display: flex;
            flex-direction: column;
            gap: 6px;
            font-size: 13px;
            color: #666;
        }}
        .status-instructions {{
            letter-spacing: 0.5px;
        }}
        .poll-info {{
            font-family: 'Courier New', monospace;
            color: #1E3A5F;
        }}
        .status-refresh-button {{
            padding: 10px 18px;
            background: #B8915F;
            border: none;
            color: #000;
            cursor: pointer;
            text-transform: uppercase;
            letter-spacing: 1px;
            font-size: 11px;
            transition: background 0.2s, color 0.2s;
        }}
        .status-refresh-button:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
        .status-refresh-button:focus {{
            outline: 2px solid #1E3A5F;
            outline-offset: 2px;
        }}
        .is-hidden {{
            display: none !important;
        }}
        .toast-container {{
            position: fixed;
            top: 20px;
            right: 20px;
            display: flex;
            flex-direction: column;
            gap: 10px;
            z-index: 2000;
        }}
        .toast {{
            background: #1E3A5F;
            color: #fff;
            padding: 12px 18px;
            border-radius: 4px;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
            opacity: 0;
            transform: translateY(-10px);
            transition: opacity 0.25s ease, transform 0.25s ease;
            font-size: 13px;
            max-width: 280px;
        }}
        .toast.visible {{
            opacity: 1;
            transform: translateY(0);
        }}
        .toast-error {{
            background: #FF5722;
            color: #000;
        }}
        .toast-success {{
            background: #B8915F;
            color: #000;
        }}
        .card-display {{
            background: #F5F3ED;
            padding: 48px;
            text-align: center;
            margin-bottom: 40px;
            border-top: 4px solid #1E3A5F;
            box-shadow: 0 2px 8px rgba(0,0,0,0.1);
        }}
        .membership-level {{
            font-size: 36px;
            font-weight: 500;
            color: #1E3A5F;
            margin-bottom: 40px;
            letter-spacing: -1px;
        }}
        .qr-code {{
            background: #fff;
            padding: 24px;
            display: inline-flex;
            align-items: center;
            justify-content: center;
            margin: 0 auto 32px auto;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .qr-code img {{
            display: block;
            max-width: 280px;
            width: 100%;
        }}
        .qr-code.placeholder {{
            width: 100%;
            max-width: 280px;
            min-height: 220px;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            gap: 12px;
            background: #E8E6E0;
            border: 2px dashed rgba(30,58,95,0.35);
        }}
        .placeholder-icon {{
            font-size: 36px;
            color: #1E3A5F;
        }}
        .placeholder-text {{
            font-size: 14px;
            font-weight: 500;
            color: #1E3A5F;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        .scan-instruction {{
            font-size: 11px;
            color: #666;
            text-transform: uppercase;
            letter-spacing: 1px;
        }}
        .info-grid {{
            display: grid;
            grid-template-columns: repeat(2, 1fr);
            gap: 2px;
            background: #D5D3CD;
            margin-bottom: 40px;
        }}
        .info-item {{
            background: #F5F3ED;
            padding: 24px;
        }}
        .info-label {{
            font-size: 10px;
            text-transform: uppercase;
            letter-spacing: 1px;
            color: #999;
            margin-bottom: 8px;
        }}
        .info-value {{
            font-size: 13px;
            font-family: 'Courier New', monospace;
            color: #000;
            word-break: break-all;
        }}
        .actions {{
            display: flex;
            flex-direction: column;
            gap: 2px;
        }}
        .button {{
            background: #B8915F;
            color: #000;
            padding: 20px 24px;
            text-decoration: none;
            font-size: 14px;
            font-weight: 500;
            text-align: left;
            transition: all 0.2s;
            display: flex;
            justify-content: space-between;
            align-items: center;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .button:hover {{
            background: #FF5722;
            color: #fff;
        }}
        .button::after {{
            content: '↓';
            font-size: 20px;
        }}
        .button.secondary {{
            background: #F5F3ED;
            color: #000;
        }}
        .button.secondary:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
        .button.secondary::after {{
            content: '→';
        }}
        @media (max-width: 600px) {{
            .info-grid {{
                grid-template-columns: 1fr;
            }}
        }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/cards/my-cards" class="back">Back to My Cards</a>

        <div class="status">● Active Card</div>

        {}

        <div class="card-display">
            <div class="membership-level">{}</div>
            {}
        </div>

        <div class="info-grid">
            <div class="info-item">
                <div class="info-label">Confirmed</div>
                <div class="info-value">{}</div>
            </div>
            <div class="info-item">
                <div class="info-label">Issued</div>
                <div class="info-value">{}</div>
            </div>
            <div class="info-item" style="grid-column: 1 / -1;">
                <div class="info-label">Card ID</div>
                <div class="info-value">{}</div>
            </div>
        </div>

        {}
    </div>
    <div class="toast-container" data-role="toast-root"></div>
    <script src="/static/js/credential-polling.js" defer></script>
</body>
</html>"#,
        credential_status_block,
        card.membership_level_label,
        qr_markup,
        card.membership_confirmed_at.format("%Y-%m-%d %H:%M UTC"),
        card.issued_at.format("%Y-%m-%d %H:%M UTC"),
        card.id,
        actions_markup
    );

    Ok(Html(html))
}

async fn card_qr(
    State(state): State<AppState>,
    Path(card_id): Path<Uuid>,
    session: Session,
) -> Result<Response, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let card = MembershipCard::find_by_id(&state.pool, card_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::NotFound)?;

    if card.member_id != member.member_id {
        return Err(CardsError::NotFound);
    }

    // Load the active wallet QR code for this card
    let wallet_qr =
        crate::models::wallet_qr_code::WalletQrCode::find_active_by_card_id(&state.pool, card.id)
            .await
            .map_err(CardsError::DatabaseError)?;

    let wallet_qr_data = wallet_qr
        .map(|qr| qr.qr_code)
        .unwrap_or_else(|| "Not available".to_string());

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "text/plain")],
        wallet_qr_data,
    )
        .into_response())
}

async fn my_cards(
    State(state): State<AppState>,
    session: Session,
) -> Result<Html<String>, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let cards = MembershipCard::list_by_member(&state.pool, member.member_id, true)
        .await
        .map_err(CardsError::DatabaseError)?;

    let cards_html: String = cards
        .iter()
        .map(|card| {
            format!(
                r#"<a href="/cards/{}" class="card">
                <div class="card-level">{}</div>
                <div class="card-dates">
                    <div class="date-item">
                        <span class="label">Confirmed</span>
                        <span class="value">{}</span>
                    </div>
                    <div class="date-item">
                        <span class="label">Issued</span>
                        <span class="value">{}</span>
                    </div>
                </div>
                <div class="card-arrow">→</div>
            </a>"#,
                card.id,
                card.membership_level_label,
                card.membership_confirmed_at.format("%Y-%m-%d"),
                card.issued_at.format("%Y-%m-%d")
            )
        })
        .collect();

    let html = format!(
        r#"<!DOCTYPE html>
<html>
<head>
    <title>My Cards - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{
            font-family: 'Helvetica Neue', Arial, sans-serif;
            background: #E8E6E0;
            color: #000;
            min-height: 100vh;
            padding: 20px;
        }}
        .container {{
            max-width: 1000px;
            margin: 40px auto;
        }}
        .header {{
            margin-bottom: 60px;
        }}
        h1 {{
            font-size: 48px;
            font-weight: 300;
            letter-spacing: -1px;
            color: #1E3A5F;
            margin-bottom: 8px;
        }}
        .subtitle {{
            font-size: 14px;
            text-transform: uppercase;
            letter-spacing: 2px;
            color: #666;
        }}
        .back {{
            display: inline-block;
            color: #666;
            text-decoration: none;
            font-size: 13px;
            margin-bottom: 40px;
            transition: color 0.2s;
        }}
        .back:hover {{ color: #1E3A5F; }}
        .back::before {{ content: '← '; }}
        .cards {{
            display: grid;
            grid-template-columns: repeat(auto-fill, minmax(320px, 1fr));
            gap: 2px;
        }}
        .card {{
            background: #F5F3ED;
            padding: 32px;
            text-decoration: none;
            color: #000;
            display: flex;
            flex-direction: column;
            transition: all 0.2s;
            position: relative;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .card:hover {{
            background: #1E3A5F;
            color: #fff;
            box-shadow: 0 4px 12px rgba(0,0,0,0.15);
        }}
        .card:hover .card-arrow {{
            color: #fff;
        }}
        .card-level {{
            font-size: 20px;
            font-weight: 500;
            margin-bottom: 24px;
        }}
        .card-dates {{
            display: flex;
            gap: 32px;
            margin-bottom: 20px;
        }}
        .date-item {{
            display: flex;
            flex-direction: column;
            gap: 4px;
        }}
        .label {{
            font-size: 10px;
            text-transform: uppercase;
            letter-spacing: 1px;
            opacity: 0.6;
        }}
        .value {{
            font-size: 13px;
            font-family: 'Courier New', monospace;
        }}
        .card-arrow {{
            position: absolute;
            top: 32px;
            right: 32px;
            font-size: 24px;
            color: #999;
            transition: color 0.2s;
        }}
        .empty {{
            text-align: center;
            padding: 80px 20px;
            background: #F5F3ED;
            box-shadow: 0 2px 4px rgba(0,0,0,0.08);
        }}
        .empty p {{
            font-size: 16px;
            color: #666;
            margin-bottom: 32px;
        }}
        .button {{
            display: inline-block;
            background: #B8915F;
            color: #000;
            padding: 16px 32px;
            text-decoration: none;
            font-size: 14px;
            font-weight: 600;
            text-transform: uppercase;
            letter-spacing: 1px;
            transition: all 0.2s;
            box-shadow: 0 2px 4px rgba(0,0,0,0.15);
        }}
        .button:hover {{
            background: #1E3A5F;
            color: #fff;
        }}
    </style>
</head>
<body>
    <div class="container">
        <a href="/" class="back">Back to Dashboard</a>
        <div class="header">
            <h1>My Cards</h1>
            <p class="subtitle">Membership Collection</p>
        </div>
        {}
    </div>
</body>
</html>"#,
        if cards.is_empty() {
            r#"<div class="empty"><p>You don't have any cards yet.</p><a href="/issuers" class="button">Browse Channels</a></div>"#.to_string()
        } else {
            format!(r#"<div class="cards">{}</div>"#, cards_html)
        }
    );

    Ok(Html(html))
}

#[derive(Debug, Serialize)]
struct PollCredentialResponse {
    status: String,
    cid: Option<String>,
    message: String,
}

/// Polls the Taiwan Digital Wallet API to check credential status and store CID
async fn poll_credential(
    State(state): State<AppState>,
    Path(card_id): Path<Uuid>,
    session: Session,
) -> Result<axum::Json<PollCredentialResponse>, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    // Verify the card belongs to the member
    let card = MembershipCard::find_by_id(&state.pool, card_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::NotFound)?;

    if card.member_id != member.member_id {
        return Err(CardsError::NotFound);
    }

    // Get the active wallet QR code for this card
    let wallet_qr = WalletQrCode::find_active_by_card_id(&state.pool, card.id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or_else(|| {
            CardsError::WalletQrError(wallet_qr::WalletQrError::ApiError(
                "No active wallet QR code found".to_string(),
            ))
        })?;

    // Check if we already have a CID
    if let Some(cid) = wallet_qr.cid {
        tracing::info!(card_id = %card_id, cid = %cid, "CID already stored");
        return Ok(axum::Json(PollCredentialResponse {
            status: "ready".to_string(),
            cid: Some(cid),
            message: "Credential already issued".to_string(),
        }));
    }

    // Poll the wallet API
    let issuer_api_url = state.config.issuer_api_url.as_deref().ok_or_else(|| {
        CardsError::WalletQrError(wallet_qr::WalletQrError::ApiError(
            "Issuer API URL not configured. Set ISSUER_API_URL.".to_string(),
        ))
    })?;

    let issuer_access_token = state
        .config
        .issuer_access_token
        .as_ref()
        .map(|token| token.expose_secret().as_str());

    let credential_response = wallet_qr::poll_credential_status(
        issuer_api_url,
        issuer_access_token,
        &wallet_qr.transaction_id,
    )
    .await
    .map_err(CardsError::WalletQrError)?;

    // Extract CID from JWT
    let cid = wallet_qr::extract_cid_from_jwt(&credential_response.credential)
        .map_err(CardsError::WalletQrError)?;

    // Store the CID in the database
    WalletQrCode::mark_as_scanned(&state.pool, wallet_qr.id, cid.clone())
        .await
        .map_err(CardsError::DatabaseError)?;

    tracing::info!(
        card_id = %card_id,
        transaction_id = %wallet_qr.transaction_id,
        cid = %cid,
        "Credential CID stored successfully"
    );

    Ok(axum::Json(PollCredentialResponse {
        status: "ready".to_string(),
        cid: Some(cid),
        message: "Credential issued and CID stored".to_string(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cards/my-cards", get(my_cards))
        .route("/cards/:id", get(show_card))
        .route("/cards/:id/qr", get(card_qr))
        .route("/cards/:id/poll-credential", get(poll_credential))
        .route(
            "/channels/:issuer_id/claim",
            get(claim_page_for_channel).post(claim_card_for_channel),
        )
        .layer(middleware::from_fn(require_auth))
}
