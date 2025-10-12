use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MembershipCard {
    pub id: Uuid,
    pub issuer_id: Uuid,
    pub platform: String, // "youtube" or "twitch"
    pub member_platform_id: String,
    pub member_display_name: String,
    pub membership_level: Option<String>,
    pub subscription_start_date: NaiveDate,
    pub subscription_duration_months: Option<i32>,
    pub is_active_member: bool,
    pub supporter_metrics: Option<JsonValue>, // JSONB field
    pub supplementary_data: Option<JsonValue>, // JSONB field
    pub qr_code_payload: String,
    pub qr_code_signature: String, // HMAC-SHA256 hex string
    pub is_revoked: bool,
    pub needs_refresh: bool,
    pub issued_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

// TODO: T021 - Implement full CRUD operations for MembershipCard
// Required functions:
// - create_card(pool: &PgPool, data: CreateCardData) -> Result<MembershipCard>
// - find_by_id(pool: &PgPool, card_id: Uuid) -> Result<Option<MembershipCard>>
// - find_active_cards_for_member(pool: &PgPool, issuer_id: Uuid, platform: &str, member_id: &str) -> Result<Vec<MembershipCard>>
// - list_cards_by_member(pool: &PgPool, platform: &str, member_id: &str, include_revoked: bool) -> Result<Vec<MembershipCard>>
// - mark_as_revoked(pool: &PgPool, card_id: Uuid) -> Result<()>
// - mark_needs_refresh(pool: &PgPool, card_id: Uuid, needs_refresh: bool) -> Result<()>
// - find_all_active_cards(pool: &PgPool) -> Result<Vec<MembershipCard>>
//
// Note: Ensure unique constraint enforcement: only one active card per (issuer_id, platform, member_platform_id)
