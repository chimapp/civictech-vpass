use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    response::{Html, IntoResponse, Response},
    routing::get,
    Form, Router,
};
use chrono::{DateTime, Utc};
use secrecy::ExposeSecret;
use serde::Deserialize;
use tower_sessions::Session;
use uuid::Uuid;

use crate::api::middleware::{
    auth::{get_authenticated_member, AuthError},
    session::{AppState, SESSION_KEY_SESSION_STARTED_AT},
};
use crate::models::{card::MembershipCard, issuer::CardIssuer, oauth_session::OAuthSession};
use crate::services::{card_issuer, qr_generator};

#[derive(Debug)]
pub enum CardsError {
    AuthError(AuthError),
    DatabaseError(sqlx::Error),
    IssuanceError(card_issuer::CardIssuanceError),
    SessionError(String),
    NotFound,
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
        .ok_or(CardsError::AuthError(AuthError::Unauthorized))?;

    let oauth_session = OAuthSession::find_by_member_id(&state.pool, member.member_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::AuthError(AuthError::Unauthorized))?;

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

    let result = card_issuer::issue_card(
        &state.pool,
        &signing_key,
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
            margin-bottom: 40px;
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
            display: inline-block;
            margin-bottom: 32px;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }}
        .qr-code img {{
            display: block;
            max-width: 280px;
            width: 100%;
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

        <div class="card-display">
            <div class="membership-level">{}</div>
            <div class="qr-code">
                <img src="/cards/{}/qr" alt="Membership QR Code" />
            </div>
            <div style="font-size: 11px; color: #666; text-transform: uppercase; letter-spacing: 1px;">
                Scan to verify
            </div>
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

        <div class="actions">
            <a href="/cards/{}/qr" download="vpass-card.svg" class="button">Download QR Code</a>
            <a href="/cards/my-cards" class="button secondary">View All Cards</a>
        </div>
    </div>
</body>
</html>"#,
        card.membership_level_label,
        card.id,
        card.membership_confirmed_at.format("%Y-%m-%d %H:%M UTC"),
        card.issued_at.format("%Y-%m-%d %H:%M UTC"),
        card.id,
        card.id
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

    let qr_payload: qr_generator::MembershipCardPayload = serde_json::from_value(card.qr_payload)
        .map_err(|e| {
        CardsError::IssuanceError(card_issuer::CardIssuanceError::QrGeneration(
            qr_generator::QrGenerationError::SerializationError(e),
        ))
    })?;

    let qr_svg = qr_generator::generate_qr_svg(&qr_payload, &card.qr_signature)
        .map_err(|e| CardsError::IssuanceError(card_issuer::CardIssuanceError::QrGeneration(e)))?;

    Ok((
        StatusCode::OK,
        [(header::CONTENT_TYPE, "image/svg+xml")],
        qr_svg,
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

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cards/my-cards", get(my_cards))
        .route("/cards/:id", get(show_card))
        .route("/cards/:id/qr", get(card_qr))
        .route(
            "/channels/:issuer_id/claim",
            get(claim_page_for_channel).post(claim_card_for_channel),
        )
}
