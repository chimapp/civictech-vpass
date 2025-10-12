use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Revocation {
    pub id: Uuid,
    pub card_id: Uuid,
    pub reason: String, // "subscription_canceled", "membership_changed", "manual_revocation", "security_issue"
    pub reason_detail: Option<String>,
    pub new_card_id: Option<Uuid>,
    pub revoked_by: String, // "system" or "manual"
    pub revoked_at: DateTime<Utc>,
}

// TODO: T050 - Implement revocation tracking for Revocation
// Required functions:
// - create_revocation(pool: &PgPool, data: CreateRevocationData) -> Result<Revocation>
// - find_by_card_id(pool: &PgPool, card_id: Uuid) -> Result<Vec<Revocation>>
// - find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Revocation>>
