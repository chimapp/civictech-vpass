use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CardIssuer {
    pub id: Uuid,
    pub issuer_type: String, // "official_channel" or "community"
    pub platform: String,     // "youtube" or "twitch"
    pub platform_channel_id: String,
    pub channel_name: String,
    pub is_verified: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// TODO: T019 - Implement full CRUD operations for CardIssuer
// Required functions:
// - create_issuer(pool: &PgPool, data: CreateIssuerData) -> Result<CardIssuer>
// - find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<CardIssuer>>
// - find_by_platform_channel_id(pool: &PgPool, platform: &str, channel_id: &str) -> Result<Option<CardIssuer>>
// - update_verified_status(pool: &PgPool, id: Uuid, is_verified: bool) -> Result<()>
// - list_issuers(pool: &PgPool, platform: Option<&str>, verified_only: bool) -> Result<Vec<CardIssuer>>
