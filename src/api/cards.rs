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
use crate::models::{card::MembershipCard, oauth_session::OAuthSession};
use crate::services::{card_issuer, encryption, qr_generator};

#[derive(Debug)]
pub enum CardsError {
    AuthError(AuthError),
    DatabaseError(sqlx::Error),
    IssuanceError(card_issuer::CardIssuanceError),
    EncryptionError(String),
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
            CardsError::EncryptionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Encryption error: {}", msg),
            ),
            CardsError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
            CardsError::NotFound => (StatusCode::NOT_FOUND, "Card not found".to_string()),
        };

        (status, message).into_response()
    }
}

/// Shows the claim card page
async fn claim_page(
    State(_state): State<AppState>,
    session: Session,
) -> Result<Html<String>, CardsError> {
    let _member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let html = r#"
<!DOCTYPE html>
<html>
<head>
    <title>Claim Membership Card - VPass</title>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <style>
        body { font-family: Arial, sans-serif; max-width: 600px; margin: 50px auto; padding: 20px; }
        h1 { color: #333; }
        form { margin-top: 30px; }
        label { display: block; margin-bottom: 5px; font-weight: bold; }
        input, select { width: 100%; padding: 10px; margin-bottom: 20px; border: 1px solid #ddd; border-radius: 4px; box-sizing: border-box; }
        button { background-color: #4CAF50; color: white; padding: 12px 20px; border: none; border-radius: 4px; cursor: pointer; font-size: 16px; width: 100%; }
        button:hover { background-color: #45a049; }
        .info { background-color: #e7f3fe; padding: 15px; border-left: 4px solid #2196F3; margin-bottom: 20px; }
    </style>
</head>
<body>
    <h1>Claim Your Membership Card</h1>
    <div class="info">
        <p><strong>Instructions:</strong></p>
        <ol>
            <li>Go to the channel's members-only verification video</li>
            <li>Post a comment (any text is fine)</li>
            <li>Copy the comment URL or ID</li>
            <li>Paste it below and submit</li>
        </ol>
    </div>
    <form action="/cards/claim" method="POST">
        <label for="issuer_id">Issuer Channel ID:</label>
        <input type="text" name="issuer_id" id="issuer_id" required placeholder="Enter issuer UUID">
        <label for="comment_link">Comment URL or ID:</label>
        <input type="text" name="comment_link" id="comment_link" required
               placeholder="https://www.youtube.com/watch?v=...&lc=... or just the comment ID">
        <button type="submit">Claim Card</button>
    </form>
</body>
</html>
    "#;

    Ok(Html(html.to_string()))
}

#[derive(Deserialize)]
struct ClaimCardForm {
    issuer_id: Uuid,
    comment_link: String,
}

async fn claim_card(
    State(state): State<AppState>,
    session: Session,
    Form(form): Form<ClaimCardForm>,
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

    let encryption_key = encryption::derive_key(state.config.encryption_key.expose_secret());
    let access_token = encryption::decrypt(&oauth_session.access_token, &encryption_key)
        .map_err(|e| CardsError::EncryptionError(e.to_string()))?;

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

    let signing_key = encryption::derive_key(state.config.session_secret.expose_secret());

    let result = card_issuer::issue_card(
        &state.pool,
        &signing_key,
        card_issuer::IssueCardRequest {
            issuer_id: form.issuer_id,
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
        r#"<!DOCTYPE html><html><head><title>Your Membership Card - VPass</title><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>body{{font-family:Arial,sans-serif;max-width:800px;margin:50px auto;padding:20px;text-align:center}}h1{{color:#333}}.card-info{{background-color:#f9f9f9;padding:20px;border-radius:8px;margin:20px 0}}.qr-container{{margin:30px 0}}.success{{color:#4CAF50;font-size:18px;font-weight:bold}}.details{{text-align:left;margin:20px auto;max-width:400px}}.details dt{{font-weight:bold;margin-top:10px}}.details dd{{margin-left:0;color:#666}}.actions{{margin-top:30px}}.button{{display:inline-block;background-color:#2196F3;color:white;padding:12px 24px;text-decoration:none;border-radius:4px;margin:5px}}.button:hover{{background-color:#0b7dda}}</style></head><body><h1>Your Membership Card</h1><p class="success">✓ Card issued successfully!</p><div class="card-info"><dl class="details"><dt>Membership Level:</dt><dd>{}</dd><dt>Confirmed At:</dt><dd>{}</dd><dt>Issued At:</dt><dd>{}</dd><dt>Card ID:</dt><dd>{}</dd></dl></div><div class="qr-container"><p><strong>Scan this QR code to import into 數位皮夾:</strong></p><a href="/cards/{}/qr" download="membership-card.svg"><img src="/cards/{}/qr" alt="QR Code" style="max-width:300px;border:2px solid #ddd;padding:10px;background:white;"/></a></div><div class="actions"><a href="/cards/{}/qr" download="membership-card.svg" class="button">Download QR Code</a><a href="/cards/my-cards" class="button">View All My Cards</a></div></body></html>"#,
        card.membership_level_label,
        card.membership_confirmed_at.format("%Y-%m-%d %H:%M UTC"),
        card.issued_at.format("%Y-%m-%d %H:%M UTC"),
        card.id,
        card.id,
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
        .map(|card| format!(
            r#"<div class="card"><h3>{}</h3><p>Confirmed: {}</p><p>Issued: {}</p><a href="/cards/{}" class="button">View Card</a></div>"#,
            card.membership_level_label,
            card.membership_confirmed_at.format("%Y-%m-%d"),
            card.issued_at.format("%Y-%m-%d"),
            card.id
        ))
        .collect();

    let html = format!(
        r#"<!DOCTYPE html><html><head><title>My Cards - VPass</title><meta charset="UTF-8"><meta name="viewport" content="width=device-width, initial-scale=1.0"><style>body{{font-family:Arial,sans-serif;max-width:1000px;margin:50px auto;padding:20px}}h1{{color:#333}}.cards{{display:grid;grid-template-columns:repeat(auto-fill,minmax(300px,1fr));gap:20px;margin-top:30px}}.card{{background-color:#f9f9f9;padding:20px;border-radius:8px;border:1px solid #ddd}}.card h3{{margin-top:0;color:#2196F3}}.button{{display:inline-block;background-color:#4CAF50;color:white;padding:10px 20px;text-decoration:none;border-radius:4px;margin-top:10px}}.button:hover{{background-color:#45a049}}.empty{{text-align:center;color:#666;margin-top:50px}}</style></head><body><h1>My Membership Cards</h1>{}</body></html>"#,
        if cards.is_empty() {
            r#"<div class="empty"><p>You don't have any cards yet.</p><a href="/cards/claim" class="button">Claim a Card</a></div>"#.to_string()
        } else {
            format!(r#"<div class="cards">{}</div>"#, cards_html)
        }
    );

    Ok(Html(html))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/cards/claim", get(claim_page).post(claim_card))
        .route("/cards/my-cards", get(my_cards))
        .route("/cards/:id", get(show_card))
        .route("/cards/:id/qr", get(card_qr))
}
