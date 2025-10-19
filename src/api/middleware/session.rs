use axum::extract::FromRef;
use sqlx::PgPool;
use tower_sessions::{Expiry, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

/// Session keys used in the application
pub const SESSION_KEY_MEMBER_ID: &str = "member_id";
pub const SESSION_KEY_CSRF_TOKEN: &str = "csrf_token";
pub const SESSION_KEY_PKCE_VERIFIER: &str = "pkce_verifier";
pub const SESSION_KEY_SESSION_STARTED_AT: &str = "session_started_at";
pub const SESSION_KEY_RETURN_URL: &str = "return_url";

/// Creates a session layer for Axum
pub async fn create_session_layer(
    pool: PgPool,
    _session_secret: &[u8],
) -> Result<SessionManagerLayer<PostgresStore>, sqlx::Error> {
    // Create the session store backed by PostgreSQL
    let session_store = PostgresStore::new(pool);
    session_store.migrate().await?;

    // Build the session layer
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(true) // Only send over HTTPS in production
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(Expiry::OnInactivity(time::Duration::hours(24)));

    Ok(session_layer)
}

/// Application state that contains the session store
#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: crate::config::Config,
}

impl FromRef<AppState> for PgPool {
    fn from_ref(state: &AppState) -> PgPool {
        state.pool.clone()
    }
}
