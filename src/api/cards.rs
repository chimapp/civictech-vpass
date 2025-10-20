use askama::Template;
use axum::{
    extract::{Path, State},
    http::{header, StatusCode},
    middleware,
    response::{IntoResponse, Response},
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

// Template structures
#[derive(Template)]
#[template(path = "cards/show.html")]
struct ShowCardTemplate {
    card: MembershipCard,
    wallet_qr: Option<WalletQrCode>,
}

#[derive(Template)]
#[template(path = "cards/claim.html")]
struct ClaimCardTemplate {
    issuer: CardIssuer,
}

#[derive(Template)]
#[template(path = "cards/list.html")]
struct MyCardsTemplate {
    cards: Vec<MembershipCard>,
}

/// Shows the claim card page for a specific channel/issuer
async fn claim_page_for_channel(
    State(state): State<AppState>,
    Path(issuer_id): Path<Uuid>,
    session: Session,
) -> Result<ClaimCardTemplate, CardsError> {
    let _member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    // Fetch the issuer to display channel information
    let issuer = CardIssuer::find_by_id(&state.pool, issuer_id)
        .await
        .map_err(CardsError::DatabaseError)?
        .ok_or(CardsError::NotFound)?;

    Ok(ClaimCardTemplate { issuer })
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
) -> Result<ShowCardTemplate, CardsError> {
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

    Ok(ShowCardTemplate { card, wallet_qr })
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
) -> Result<MyCardsTemplate, CardsError> {
    let member = get_authenticated_member(&session)
        .await
        .map_err(CardsError::AuthError)?;

    let cards = MembershipCard::list_by_member(&state.pool, member.member_id, true)
        .await
        .map_err(CardsError::DatabaseError)?;

    Ok(MyCardsTemplate { cards })
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
