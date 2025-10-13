use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthSession {
    pub id: Uuid,
    pub user_role: String, // "member" or "organizer"
    pub platform: String,  // "youtube" or "twitch"
    pub platform_user_id: String,
    pub access_token: String,          // Stored encrypted
    pub refresh_token: Option<String>, // Stored encrypted
    pub token_expires_at: DateTime<Utc>,
    pub scope: String,
    pub issuer_id: Option<Uuid>, // Only for organizer sessions
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

// TODO: T020 - Implement full session management for OAuthSession
// Required functions:
// - create_session(pool: &PgPool, data: CreateSessionData) -> Result<OAuthSession>
// - find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<OAuthSession>>
// - find_by_platform_user(pool: &PgPool, platform: &str, user_id: &str) -> Result<Option<OAuthSession>>
// - update_tokens(pool: &PgPool, id: Uuid, access_token: &str, refresh_token: Option<&str>, expires_at: DateTime<Utc>) -> Result<()>
// - update_last_used(pool: &PgPool, id: Uuid) -> Result<()>
// - delete_session(pool: &PgPool, id: Uuid) -> Result<()>
// - delete_expired_sessions(pool: &PgPool) -> Result<u64>
//
// Note: access_token and refresh_token must be encrypted before storing (see T013)
