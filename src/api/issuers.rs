use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
    Form, Json, Router,
};
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::api::middleware::session::{AppState, SESSION_KEY_MEMBER_ID};
use crate::models::issuer::{CardIssuer, CreateIssuerData};
use crate::services::youtube_channel;

#[derive(Debug)]
pub enum IssuersError {
    DatabaseError(sqlx::Error),
    NotFound,
    ValidationError(String),
    YouTubeApiError(youtube_channel::YouTubeChannelError),
    SessionError(String),
}

impl IntoResponse for IssuersError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            IssuersError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            IssuersError::NotFound => (StatusCode::NOT_FOUND, "Issuer not found".to_string()),
            IssuersError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            IssuersError::YouTubeApiError(e) => {
                (StatusCode::BAD_REQUEST, format!("YouTube API error: {}", e))
            }
            IssuersError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
        };

        (status, message).into_response()
    }
}

// Template structures
#[derive(Template)]
#[template(path = "issuers/list.html")]
struct ListIssuersTemplate {
    issuers: Vec<CardIssuer>,
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "issuers/new.html")]
struct NewIssuerTemplate {
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "issuers/edit.html")]
struct EditIssuerTemplate {
    issuer: CardIssuer,
    is_authenticated: bool,
}

async fn is_authenticated(session: &Session) -> Result<bool, IssuersError> {
    let member_id: Option<Uuid> = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|e| IssuersError::SessionError(e.to_string()))?;

    Ok(member_id.is_some())
}

/// List all issuers
async fn list_issuers(
    State(state): State<AppState>,
    session: Session,
) -> Result<ListIssuersTemplate, IssuersError> {
    let issuers = CardIssuer::list_active(&state.pool)
        .await
        .map_err(IssuersError::DatabaseError)?;

    let is_authenticated = is_authenticated(&session).await?;

    Ok(ListIssuersTemplate {
        issuers,
        is_authenticated,
    })
}

/// Show create form
async fn new_issuer_form(session: Session) -> Result<NewIssuerTemplate, IssuersError> {
    let is_authenticated = is_authenticated(&session).await?;
    Ok(NewIssuerTemplate { is_authenticated })
}

#[derive(Deserialize)]
struct CreateIssuerForm {
    youtube_channel_id: String,
    channel_name: String,
    channel_handle: Option<String>,
    verification_video_id: String,
    default_membership_label: String,
    vc_uid: Option<String>,
}

/// Create a new issuer
async fn create_issuer(
    State(state): State<AppState>,
    Form(form): Form<CreateIssuerForm>,
) -> Result<Response, IssuersError> {
    // Basic validation
    if form.youtube_channel_id.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "YouTube Channel ID is required".to_string(),
        ));
    }
    if form.channel_name.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "Channel Name is required".to_string(),
        ));
    }
    if form.verification_video_id.trim().is_empty() {
        return Err(IssuersError::ValidationError(
            "Verification Video ID is required".to_string(),
        ));
    }

    let channel_handle = form.channel_handle.and_then(|h| {
        let trimmed = h.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let vc_uid = form.vc_uid.and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    });

    let issuer = CardIssuer::create(
        &state.pool,
        CreateIssuerData {
            youtube_channel_id: form.youtube_channel_id.trim().to_string(),
            channel_handle,
            channel_name: form.channel_name.trim().to_string(),
            verification_video_id: form.verification_video_id.trim().to_string(),
            default_membership_label: form.default_membership_label.trim().to_string(),
            vc_uid,
        },
    )
    .await
    .map_err(IssuersError::DatabaseError)?;

    tracing::info!(issuer_id = %issuer.id, "Created new issuer");

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

/// Show edit form
async fn edit_issuer_form(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    session: Session,
) -> Result<EditIssuerTemplate, IssuersError> {
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    let is_authenticated = is_authenticated(&session).await?;

    Ok(EditIssuerTemplate {
        issuer,
        is_authenticated,
    })
}

#[derive(Deserialize)]
struct UpdateIssuerForm {
    channel_name: Option<String>,
    channel_handle: Option<String>,
    verification_video_id: Option<String>,
    default_membership_label: Option<String>,
    vc_uid: Option<String>,
}

/// Update an existing issuer
async fn update_issuer(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Form(form): Form<UpdateIssuerForm>,
) -> Result<Response, IssuersError> {
    // Verify issuer exists
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    // Update channel info if provided
    let channel_name = form.channel_name.filter(|s| !s.trim().is_empty());
    let channel_handle = form.channel_handle.filter(|s| !s.trim().is_empty());
    let default_membership_label = form
        .default_membership_label
        .filter(|s| !s.trim().is_empty());
    let vc_uid = form.vc_uid.filter(|s| !s.trim().is_empty());

    CardIssuer::update_channel_info(
        &state.pool,
        id,
        channel_name,
        channel_handle,
        default_membership_label,
        vc_uid,
    )
    .await
    .map_err(IssuersError::DatabaseError)?;

    // Update verification video if provided
    if let Some(video_id) = form.verification_video_id.filter(|s| !s.trim().is_empty()) {
        CardIssuer::update_verification_video(&state.pool, id, &video_id)
            .await
            .map_err(IssuersError::DatabaseError)?;
    }

    tracing::info!(issuer_id = %issuer.id, "Updated issuer");

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

/// Toggle issuer active status
async fn toggle_issuer_status(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Response, IssuersError> {
    let issuer = CardIssuer::find_by_id(&state.pool, id)
        .await
        .map_err(IssuersError::DatabaseError)?
        .ok_or(IssuersError::NotFound)?;

    let new_status = !issuer.is_active;

    CardIssuer::set_active_status(&state.pool, id, new_status)
        .await
        .map_err(IssuersError::DatabaseError)?;

    tracing::info!(
        issuer_id = %issuer.id,
        is_active = new_status,
        "Toggled issuer status"
    );

    Ok(axum::response::Redirect::to("/issuers").into_response())
}

#[derive(Deserialize)]
struct AutoFillQuery {
    url: String,
}

#[derive(Serialize)]
struct AutoFillResponse {
    channel_id: String,
    channel_name: String,
    channel_handle: String,
}

/// Auto-fill channel information from YouTube URL
async fn autofill_channel(
    State(state): State<AppState>,
    Query(query): Query<AutoFillQuery>,
) -> Result<Json<AutoFillResponse>, IssuersError> {
    // Check if we have a YouTube API key configured
    let api_key = state.config.youtube_api_key.as_ref().ok_or_else(|| {
        IssuersError::ValidationError("YouTube API key not configured".to_string())
    })?;

    tracing::info!(url = %query.url, "Auto-filling channel info");

    let channel_info = youtube_channel::fetch_channel_info(&query.url, api_key)
        .await
        .map_err(IssuersError::YouTubeApiError)?;

    Ok(Json(AutoFillResponse {
        channel_id: channel_info.channel_id,
        channel_name: channel_info.channel_name,
        channel_handle: channel_info.channel_handle.unwrap_or_default(),
    }))
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/issuers", get(list_issuers).post(create_issuer))
        .route("/issuers/new", get(new_issuer_form))
        .route("/issuers/autofill", get(autofill_channel))
        .route("/issuers/:id/edit", get(edit_issuer_form))
        .route("/issuers/:id", post(update_issuer))
        .route("/issuers/:id/toggle", post(toggle_issuer_status))
}
