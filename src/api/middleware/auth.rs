use axum::{
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use tower_sessions::Session;
use uuid::Uuid;

use super::session::SESSION_KEY_MEMBER_ID;

/// Authentication error responses
#[derive(Debug)]
pub enum AuthError {
    Unauthorized,
    SessionError,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        match self {
            AuthError::Unauthorized => (
                StatusCode::UNAUTHORIZED,
                "Authentication required. Please log in.",
            )
                .into_response(),
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
        return Err(AuthError::Unauthorized);
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
        .ok_or(AuthError::Unauthorized)?;

    Ok(AuthenticatedMember { member_id })
}
