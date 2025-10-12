use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationEvent {
    pub id: Uuid,
    pub card_id: Uuid,
    pub verifier_issuer_id: Uuid,
    pub verification_result: String, // "success", "revoked", "expired", "invalid_signature", "wrong_issuer"
    pub verification_context: Option<JsonValue>, // JSONB field
    pub verified_at: DateTime<Utc>,
}

// TODO: T042 - Implement verification event logging for VerificationEvent
// Required functions:
// - create_event(pool: &PgPool, data: CreateEventData) -> Result<VerificationEvent>
// - list_events_by_issuer(pool: &PgPool, issuer_id: Uuid, limit: i64, offset: i64) -> Result<Vec<VerificationEvent>>
// - count_events_by_issuer(pool: &PgPool, issuer_id: Uuid) -> Result<i64>
// - list_events_by_card(pool: &PgPool, card_id: Uuid) -> Result<Vec<VerificationEvent>>
