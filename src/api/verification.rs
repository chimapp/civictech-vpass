use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::api::middleware::session::{AppState, SESSION_KEY_MEMBER_ID};
use crate::models::{
    event::Event,
    verification_event::{CreateVerificationEventData, VerificationEvent},
};
use crate::services::oidvp_verifier;

#[derive(Debug)]
pub enum VerificationApiError {
    DatabaseError(sqlx::Error),
    OidvpError(oidvp_verifier::OidvpError),
    EventNotFound,
    ValidationError(String),
    ConfigError(String),
    SessionError(String),
}

impl IntoResponse for VerificationApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            VerificationApiError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            VerificationApiError::OidvpError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("OIDVP error: {}", e),
            ),
            VerificationApiError::EventNotFound => {
                (StatusCode::NOT_FOUND, "Event not found".to_string())
            }
            VerificationApiError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            VerificationApiError::ConfigError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Configuration error: {}", msg),
            ),
            VerificationApiError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
        };

        (status, message).into_response()
    }
}

// Templates
#[derive(Template)]
#[template(path = "verification/home.html")]
struct VerificationHomeTemplate {
    events: Vec<Event>,
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "verification/scanner.html")]
struct ScannerTemplate {
    event: Event,
    issuer: crate::models::issuer::CardIssuer,
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "verification/history.html")]
struct HistoryTemplate {
    event: Event,
    issuer: crate::models::issuer::CardIssuer,
    events: Vec<VerificationEventWithCard>,
    success_count: usize,
    failed_count: usize,
    page: i64,
    per_page: i64,
    total: i64,
    is_authenticated: bool,
}

#[derive(Debug, Serialize)]
struct VerificationEventWithCard {
    #[serde(flatten)]
    event: VerificationEvent,
    card: Option<crate::models::card::MembershipCard>,
    member: Option<crate::models::member::Member>,
}

// Request/Response types
#[derive(Debug, Serialize)]
pub struct RequestQrResponse {
    pub transaction_id: String,
    pub qrcode_image: String, // base64 PNG
    pub auth_uri: String,
    pub expires_in_seconds: i64,
}

#[derive(Debug, Serialize)]
pub struct CheckResultResponse {
    pub status: String, // "pending", "completed", "expired"
    pub verify_result: Option<bool>,
    pub result_description: Option<String>,
    pub member_info: Option<serde_json::Value>,
    pub message: String,
}

#[derive(Debug, Deserialize)]
pub struct PaginationParams {
    pub page: Option<i64>,
    pub per_page: Option<i64>,
}

async fn is_authenticated(session: &Session) -> Result<bool, VerificationApiError> {
    let member_id: Option<Uuid> = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|e| VerificationApiError::SessionError(e.to_string()))?;

    Ok(member_id.is_some())
}

// Handlers

/// Verification home page - shows list of active events
async fn verification_home(
    State(state): State<AppState>,
    session: Session,
) -> Result<VerificationHomeTemplate, VerificationApiError> {
    let events = Event::list_active(&state.pool)
        .await
        .map_err(VerificationApiError::DatabaseError)?;

    let is_authenticated = is_authenticated(&session).await?;

    Ok(VerificationHomeTemplate {
        events,
        is_authenticated,
    })
}

/// Scanner page for a specific event
async fn scanner_page(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    session: Session,
) -> Result<ScannerTemplate, VerificationApiError> {
    let event = Event::find_by_id(&state.pool, event_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    let issuer = crate::models::issuer::CardIssuer::find_by_id(&state.pool, event.issuer_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    let is_authenticated = is_authenticated(&session).await?;

    Ok(ScannerTemplate {
        event,
        issuer,
        is_authenticated,
    })
}

/// Request verification QR code
///
/// Generates a new QR code via OIDVP API (no database storage - frontend manages state)
async fn request_qr(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<RequestQrResponse>, VerificationApiError> {
    // Verify event exists and get verifier_ref from event
    let event = Event::find_by_id(&state.pool, event_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    // Get verifier config
    let verifier_api_url = state
        .config
        .verifier_api_url
        .as_ref()
        .ok_or_else(|| VerificationApiError::ConfigError("VERIFIER_API_URL not configured".to_string()))?;

    let verifier_access_token = state
        .config
        .verifier_access_token
        .as_ref()
        .ok_or_else(|| VerificationApiError::ConfigError("VERIFIER_ACCESS_TOKEN not configured".to_string()))?;

    tracing::info!(event_id = %event_id, verifier_ref = %event.verifier_ref, "Requesting verification QR code");

    // Call OIDVP API to generate QR code using event's verifier_ref
    let qr_response = oidvp_verifier::request_verification_qr(
        verifier_api_url,
        verifier_access_token.expose_secret(),
        &event.verifier_ref,
    )
    .await
    .map_err(VerificationApiError::OidvpError)?;

    // Strip data URL prefix if present, as frontend will add it
    let qrcode_image = qr_response
        .qrcode_image
        .strip_prefix("data:image/png;base64,")
        .unwrap_or(&qr_response.qrcode_image)
        .to_string();

    tracing::info!(
        transaction_id = %qr_response.transaction_id,
        "Verification QR generated (frontend will manage state)"
    );

    // Return directly to frontend - no database storage
    // Frontend manages the pending state in JavaScript
    Ok(Json(RequestQrResponse {
        transaction_id: qr_response.transaction_id,
        qrcode_image,
        auth_uri: qr_response.auth_uri,
        expires_in_seconds: 300, // 5 minutes
    }))
}

/// Check verification result
///
/// Polls OIDVP API for verification result (frontend-managed state)
async fn check_result(
    State(state): State<AppState>,
    Path((event_id, transaction_id)): Path<(Uuid, String)>,
) -> Result<Json<CheckResultResponse>, VerificationApiError> {
    // Verify event exists
    Event::find_by_id(&state.pool, event_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    // Get verifier config
    let verifier_api_url = state
        .config
        .verifier_api_url
        .as_ref()
        .ok_or_else(|| VerificationApiError::ConfigError("VERIFIER_API_URL not configured".to_string()))?;

    let verifier_access_token = state
        .config
        .verifier_access_token
        .as_ref()
        .ok_or_else(|| VerificationApiError::ConfigError("VERIFIER_ACCESS_TOKEN not configured".to_string()))?;

    tracing::debug!(transaction_id = %transaction_id, "Polling OIDVP result");

    // Poll OIDVP API directly (no database session tracking)
    match oidvp_verifier::poll_verification_result(
        verifier_api_url,
        verifier_access_token.expose_secret(),
        &transaction_id,
    )
    .await
    {
        Ok(result) => {
            // Extract member info from claims
            let member_info = if let Some(ref data) = result.data {
                oidvp_verifier::extract_member_info(data)
            } else {
                None
            };

            // If successful, create verification event record (audit log)
            if result.verify_result {
                // Try to extract card_id from member_info if available
                let card_id = member_info
                    .as_ref()
                    .and_then(|info| info.get("cardId"))
                    .and_then(|v| v.as_str())
                    .and_then(|s| Uuid::parse_str(s).ok());

                let _ = VerificationEvent::create_event(
                    &state.pool,
                    CreateVerificationEventData {
                        event_id,
                        card_id,
                        verification_result: "success".to_string(),
                        verification_context: Some(serde_json::json!({
                            "transaction_id": transaction_id,
                            "method": "oidvp"
                        })),
                        raw_payload: Some(serde_json::to_string(&result).unwrap_or_default()),
                    },
                )
                .await
                .map_err(VerificationApiError::DatabaseError)?;
            }

            tracing::info!(
                transaction_id = %transaction_id,
                verify_result = result.verify_result,
                "Verification completed"
            );

            Ok(Json(CheckResultResponse {
                status: "completed".to_string(),
                verify_result: Some(result.verify_result),
                result_description: Some(result.result_description.clone()),
                member_info,
                message: if result.verify_result {
                    "Verification successful!".to_string()
                } else {
                    format!("Verification failed: {}", result.result_description)
                },
            }))
        }
        Err(oidvp_verifier::OidvpError::NotReady) => {
            // Still waiting for user to scan (frontend will keep polling)
            Ok(Json(CheckResultResponse {
                status: "pending".to_string(),
                verify_result: None,
                result_description: None,
                member_info: None,
                message: "Waiting for user to scan QR code...".to_string(),
            }))
        }
        Err(e) => {
            tracing::error!(error = ?e, "Failed to poll verification result");
            Err(VerificationApiError::OidvpError(e))
        }
    }
}

/// Verification history for an event
async fn verification_history(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Query(params): Query<PaginationParams>,
    session: Session,
) -> Result<HistoryTemplate, VerificationApiError> {
    let event = Event::find_by_id(&state.pool, event_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    let issuer = crate::models::issuer::CardIssuer::find_by_id(&state.pool, event.issuer_id)
        .await
        .map_err(VerificationApiError::DatabaseError)?
        .ok_or(VerificationApiError::EventNotFound)?;

    let page = params.page.unwrap_or(1);
    let per_page = params.per_page.unwrap_or(50);
    let offset = (page - 1) * per_page;

    let verification_events =
        VerificationEvent::list_by_event(&state.pool, event_id, per_page, offset)
            .await
            .map_err(VerificationApiError::DatabaseError)?;

    let total = VerificationEvent::count_by_event_and_result(&state.pool, event_id, None)
        .await
        .map_err(VerificationApiError::DatabaseError)?;

    // Enrich with card and member data
    let mut events_with_cards = Vec::new();
    for ve in verification_events {
        let (card, member) = if let Some(card_id) = ve.card_id {
            let card = crate::models::card::MembershipCard::find_by_id(&state.pool, card_id)
                .await
                .map_err(VerificationApiError::DatabaseError)?;

            let member = if let Some(ref c) = card {
                crate::models::member::Member::find_by_id(&state.pool, c.member_id)
                    .await
                    .map_err(VerificationApiError::DatabaseError)?
            } else {
                None
            };

            (card, member)
        } else {
            (None, None)
        };

        events_with_cards.push(VerificationEventWithCard {
            event: ve,
            card,
            member,
        });
    }

    let success_count = events_with_cards
        .iter()
        .filter(|ve| ve.event.verification_result == "success")
        .count();
    let failed_count = events_with_cards
        .iter()
        .filter(|ve| ve.event.verification_result == "failed")
        .count();

    let is_authenticated = is_authenticated(&session).await?;

    Ok(HistoryTemplate {
        event,
        issuer,
        events: events_with_cards,
        success_count,
        failed_count,
        page,
        per_page,
        total,
        is_authenticated,
    })
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/verify", get(verification_home))
        .route("/verify/:event_id/scanner", get(scanner_page))
        .route("/verify/:event_id/request-qr", post(request_qr))
        .route("/verify/:event_id/check-result/:transaction_id", get(check_result))
        .route("/verify/:event_id/history", get(verification_history))
}
