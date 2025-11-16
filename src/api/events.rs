use askama::Template;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Form, Json, Router,
};
use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use tower_sessions::Session;
use uuid::Uuid;

use crate::api::middleware::session::{AppState, SESSION_KEY_MEMBER_ID};
use crate::models::event::{CreateEventData, Event, UpdateEventData};

#[derive(Debug)]
pub enum EventError {
    DatabaseError(sqlx::Error),
    NotFound,
    ValidationError(String),
    SessionError(String),
}

impl IntoResponse for EventError {
    fn into_response(self) -> axum::response::Response {
        let (status, message) = match self {
            EventError::DatabaseError(e) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Database error: {}", e),
            ),
            EventError::NotFound => (StatusCode::NOT_FOUND, "Event not found".to_string()),
            EventError::ValidationError(msg) => (StatusCode::BAD_REQUEST, msg),
            EventError::SessionError(msg) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Session error: {}", msg),
            ),
        };

        (status, message).into_response()
    }
}

// Templates
#[derive(Template)]
#[template(path = "events/list.html")]
struct EventListTemplate {
    events: Vec<Event>,
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "events/new.html")]
struct NewEventTemplate {
    issuers: Vec<crate::models::issuer::CardIssuer>,
    is_authenticated: bool,
}

#[derive(Template)]
#[template(path = "events/show.html")]
struct ShowEventTemplate {
    event: Event,
    issuer: crate::models::issuer::CardIssuer,
    stats: EventStats,
    is_authenticated: bool,
}

// Request/Response types
#[derive(Debug, Deserialize)]
pub struct ListEventsQuery {
    pub issuer_id: Option<Uuid>,
    pub active_only: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub issuer_id: Uuid,
    pub event_name: String,
    pub event_description: Option<String>,
    pub event_date: NaiveDate,
    pub event_location: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateEventRequest {
    pub event_name: Option<String>,
    pub event_description: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub event_location: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventStats {
    pub total_scans: i64,
    pub successful_scans: i64,
    pub failed_scans: i64,
    pub unique_cards: i64,
}

impl EventStats {
    pub fn success_rate_label(&self) -> Option<String> {
        if self.total_scans > 0 {
            let rate = self.successful_scans as f64 * 100.0 / self.total_scans as f64;
            Some(format!("{:.1}", rate))
        } else {
            None
        }
    }
}

async fn is_authenticated(session: &Session) -> Result<bool, EventError> {
    let member_id: Option<Uuid> = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|e| EventError::SessionError(e.to_string()))?;

    Ok(member_id.is_some())
}

// Handlers

/// List events (HTML)
async fn list_events_page(
    State(state): State<AppState>,
    Query(params): Query<ListEventsQuery>,
    session: Session,
) -> Result<EventListTemplate, EventError> {
    let events = if let Some(issuer_id) = params.issuer_id {
        Event::list_by_issuer(&state.pool, issuer_id, params.active_only.unwrap_or(false))
            .await
            .map_err(EventError::DatabaseError)?
    } else {
        Event::list_active(&state.pool)
            .await
            .map_err(EventError::DatabaseError)?
    };

    let is_authenticated = is_authenticated(&session).await?;

    Ok(EventListTemplate {
        events,
        is_authenticated,
    })
}

/// List events (JSON API)
async fn list_events_json(
    State(state): State<AppState>,
    Query(params): Query<ListEventsQuery>,
) -> Result<Json<Vec<Event>>, EventError> {
    let events = if let Some(issuer_id) = params.issuer_id {
        Event::list_by_issuer(&state.pool, issuer_id, params.active_only.unwrap_or(false))
            .await
            .map_err(EventError::DatabaseError)?
    } else {
        Event::list_active(&state.pool)
            .await
            .map_err(EventError::DatabaseError)?
    };

    Ok(Json(events))
}

/// New event page
async fn new_event_page(
    State(state): State<AppState>,
    session: Session,
) -> Result<NewEventTemplate, EventError> {
    let issuers = crate::models::issuer::CardIssuer::list_active(&state.pool)
        .await
        .map_err(EventError::DatabaseError)?;

    let is_authenticated = is_authenticated(&session).await?;

    Ok(NewEventTemplate {
        issuers,
        is_authenticated,
    })
}

/// Create event (HTML form)
async fn create_event_form(
    State(state): State<AppState>,
    Form(req): Form<CreateEventRequest>,
) -> Result<axum::response::Redirect, EventError> {
    // Validate
    if req.event_name.trim().is_empty() {
        return Err(EventError::ValidationError(
            "Event name is required".to_string(),
        ));
    }

    let event = Event::create(
        &state.pool,
        CreateEventData {
            issuer_id: req.issuer_id,
            event_name: req.event_name,
            event_description: req.event_description,
            event_date: req.event_date,
            event_location: req.event_location,
        },
    )
    .await
    .map_err(EventError::DatabaseError)?;

    tracing::info!(event_id = %event.id, event_name = %event.event_name, "Event created");

    Ok(axum::response::Redirect::to(&format!(
        "/events/{}",
        event.id
    )))
}

/// Create event (JSON API)
async fn create_event_json(
    State(state): State<AppState>,
    Json(req): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<Event>), EventError> {
    // Validate
    if req.event_name.trim().is_empty() {
        return Err(EventError::ValidationError(
            "Event name is required".to_string(),
        ));
    }

    let event = Event::create(
        &state.pool,
        CreateEventData {
            issuer_id: req.issuer_id,
            event_name: req.event_name,
            event_description: req.event_description,
            event_date: req.event_date,
            event_location: req.event_location,
        },
    )
    .await
    .map_err(EventError::DatabaseError)?;

    tracing::info!(event_id = %event.id, event_name = %event.event_name, "Event created");

    Ok((StatusCode::CREATED, Json(event)))
}

/// Get event details
async fn show_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    session: Session,
) -> Result<ShowEventTemplate, EventError> {
    let event = Event::find_by_id(&state.pool, id)
        .await
        .map_err(EventError::DatabaseError)?
        .ok_or(EventError::NotFound)?;

    let issuer = crate::models::issuer::CardIssuer::find_by_id(&state.pool, event.issuer_id)
        .await
        .map_err(EventError::DatabaseError)?
        .ok_or(EventError::NotFound)?;

    // Calculate stats
    let total_scans =
        crate::models::verification_event::VerificationEvent::count_by_event_and_result(
            &state.pool,
            id,
            None,
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let successful_scans =
        crate::models::verification_event::VerificationEvent::count_by_event_and_result(
            &state.pool,
            id,
            Some("success"),
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let unique_cards =
        crate::models::verification_event::VerificationEvent::count_unique_cards_by_event(
            &state.pool,
            id,
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let stats = EventStats {
        total_scans,
        successful_scans,
        failed_scans: total_scans - successful_scans,
        unique_cards,
    };

    let is_authenticated = is_authenticated(&session).await?;

    Ok(ShowEventTemplate {
        event,
        issuer,
        stats,
        is_authenticated,
    })
}

/// Get event (JSON)
async fn get_event_json(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Event>, EventError> {
    let event = Event::find_by_id(&state.pool, id)
        .await
        .map_err(EventError::DatabaseError)?
        .ok_or(EventError::NotFound)?;

    Ok(Json(event))
}

/// Update event
async fn update_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(req): Json<UpdateEventRequest>,
) -> Result<Json<Event>, EventError> {
    let event = Event::update(
        &state.pool,
        id,
        UpdateEventData {
            event_name: req.event_name,
            event_description: req.event_description,
            event_date: req.event_date,
            event_location: req.event_location,
        },
    )
    .await
    .map_err(EventError::DatabaseError)?;

    tracing::info!(event_id = %event.id, "Event updated");

    Ok(Json(event))
}

/// Deactivate event
async fn deactivate_event(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, EventError> {
    Event::deactivate(&state.pool, id)
        .await
        .map_err(EventError::DatabaseError)?;

    tracing::info!(event_id = %id, "Event deactivated");

    Ok(StatusCode::NO_CONTENT)
}

/// Get event stats (JSON)
async fn event_stats(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<EventStats>, EventError> {
    // Verify event exists
    Event::find_by_id(&state.pool, id)
        .await
        .map_err(EventError::DatabaseError)?
        .ok_or(EventError::NotFound)?;

    let total_scans =
        crate::models::verification_event::VerificationEvent::count_by_event_and_result(
            &state.pool,
            id,
            None,
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let successful_scans =
        crate::models::verification_event::VerificationEvent::count_by_event_and_result(
            &state.pool,
            id,
            Some("success"),
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let unique_cards =
        crate::models::verification_event::VerificationEvent::count_unique_cards_by_event(
            &state.pool,
            id,
        )
        .await
        .map_err(EventError::DatabaseError)?;

    let stats = EventStats {
        total_scans,
        successful_scans,
        failed_scans: total_scans - successful_scans,
        unique_cards,
    };

    Ok(Json(stats))
}

pub fn router() -> Router<AppState> {
    Router::new()
        // HTML routes
        .route("/events", get(list_events_page))
        .route("/events/new", get(new_event_page))
        .route("/events/create", post(create_event_form))
        .route("/events/:id", get(show_event))
        // JSON API routes
        .route("/api/events", get(list_events_json).post(create_event_json))
        .route(
            "/api/events/:id",
            get(get_event_json)
                .put(update_event)
                .delete(deactivate_event),
        )
        .route("/api/events/:id/stats", get(event_stats))
}
