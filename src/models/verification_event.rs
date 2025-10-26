use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationEvent {
    pub id: Uuid,
    pub event_id: Uuid,
    pub card_id: Option<Uuid>, // nullable: failed scans may not have valid card_id
    pub verification_result: String, // "success", "invalid_signature", "card_not_found", "invalid_payload"
    pub verification_context: Option<JsonValue>, // JSONB field for extra metadata
    pub raw_payload: Option<String>, // Original QR payload for debugging
    pub verified_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateVerificationEventData {
    pub event_id: Uuid,
    pub card_id: Option<Uuid>,
    pub verification_result: String,
    pub verification_context: Option<JsonValue>,
    pub raw_payload: Option<String>,
}

impl VerificationEvent {
    /// Create a new verification event
    pub async fn create_event(
        pool: &PgPool,
        data: CreateVerificationEventData,
    ) -> Result<Self, sqlx::Error> {
        let event = sqlx::query_as::<_, VerificationEvent>(
            r#"
            INSERT INTO verification_events (event_id, card_id, verification_result, verification_context, raw_payload)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.event_id)
        .bind(data.card_id)
        .bind(data.verification_result)
        .bind(data.verification_context)
        .bind(data.raw_payload)
        .fetch_one(pool)
        .await?;

        Ok(event)
    }

    /// List verification events for a specific event
    pub async fn list_by_event(
        pool: &PgPool,
        event_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let events = sqlx::query_as::<_, VerificationEvent>(
            r#"
            SELECT * FROM verification_events
            WHERE event_id = $1
            ORDER BY verified_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(event_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Count verification events by event and result
    pub async fn count_by_event_and_result(
        pool: &PgPool,
        event_id: Uuid,
        result: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let count = if let Some(result) = result {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM verification_events
                WHERE event_id = $1 AND verification_result = $2
                "#,
            )
            .bind(event_id)
            .bind(result)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM verification_events
                WHERE event_id = $1
                "#,
            )
            .bind(event_id)
            .fetch_one(pool)
            .await?
        };

        Ok(count)
    }

    /// Count unique cards verified at an event
    pub async fn count_unique_cards_by_event(
        pool: &PgPool,
        event_id: Uuid,
    ) -> Result<i64, sqlx::Error> {
        let count = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT COUNT(DISTINCT card_id)
            FROM verification_events
            WHERE event_id = $1 AND card_id IS NOT NULL
            "#,
        )
        .bind(event_id)
        .fetch_one(pool)
        .await?;

        Ok(count)
    }

    /// List verification events for a specific card
    pub async fn list_by_card(pool: &PgPool, card_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        let events = sqlx::query_as::<_, VerificationEvent>(
            r#"
            SELECT * FROM verification_events
            WHERE card_id = $1
            ORDER BY verified_at DESC
            "#,
        )
        .bind(card_id)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// List recent verification events across all events
    pub async fn list_recent(
        pool: &PgPool,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let events = sqlx::query_as::<_, VerificationEvent>(
            r#"
            SELECT * FROM verification_events
            ORDER BY verified_at DESC
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }
}
