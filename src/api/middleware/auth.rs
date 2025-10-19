use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use tower_sessions::Session;
use uuid::Uuid;

use super::session::{SESSION_KEY_MEMBER_ID, SESSION_KEY_RETURN_URL};

/// Authentication error responses
#[derive(Debug)]
pub enum AuthError {
    Unauthorized(String), // Store the requested path
    SessionError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::Unauthorized(_) => {
                // Redirect to login page instead of showing error message
                // The return URL is already stored in the session by the middleware
                Redirect::to("/auth/youtube/login").into_response()
            }
            AuthError::SessionError => {
                (StatusCode::INTERNAL_SERVER_ERROR, "Session error occurred.").into_response()
            }
        }
    }
}

/// Middleware that requires the user to be authenticated
pub async fn require_auth(
    session: Session,
    request: Request,
    next: Next,
) -> Result<Response, AuthError> {
    // Check if member_id exists in session
    let member_id: Option<Uuid> = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|_| AuthError::SessionError)?;

    if member_id.is_none() {
        // Store the requested URL for redirect after login
        let requested_path = request
            .uri()
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");

        // Only store if it's not already a login/logout/callback URL
        if !requested_path.starts_with("/auth/") {
            let _ = session
                .insert(SESSION_KEY_RETURN_URL, requested_path.to_string())
                .await;
        }

        return Err(AuthError::Unauthorized(requested_path.to_string()));
    }

    Ok(next.run(request).await)
}

/// Extension type that holds the authenticated member ID
#[derive(Debug, Clone)]
pub struct AuthenticatedMember {
    pub member_id: Uuid,
}

/// Extracts the authenticated member ID from the session
pub async fn get_authenticated_member(session: &Session) -> Result<AuthenticatedMember, AuthError> {
    let member_id: Uuid = session
        .get(SESSION_KEY_MEMBER_ID)
        .await
        .map_err(|_| AuthError::SessionError)?
        .ok_or(AuthError::Unauthorized(String::new()))?;

    Ok(AuthenticatedMember { member_id })
}
