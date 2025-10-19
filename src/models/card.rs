use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct MembershipCard {
    pub id: Uuid,
    pub issuer_id: Uuid,
    pub member_id: Uuid,
    pub membership_level_label: String,
    pub membership_confirmed_at: DateTime<Utc>,
    pub verification_comment_id: String,
    pub verification_video_id: String,
    pub snapshot_json: JsonValue,
    pub qr_payload: JsonValue,
    pub qr_signature: String,
    pub is_primary: bool,
    pub issued_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateCardData {
    pub issuer_id: Uuid,
    pub member_id: Uuid,
    pub membership_level_label: String,
    pub membership_confirmed_at: DateTime<Utc>,
    pub verification_comment_id: String,
    pub verification_video_id: String,
    pub snapshot_json: JsonValue,
    pub qr_payload: JsonValue,
    pub qr_signature: String,
}

impl MembershipCard {
    /// Creates a new membership card
    /// Automatically marks it as primary and deactivates any existing primary cards for the same issuer/member pair
    pub async fn create(pool: &PgPool, data: CreateCardData) -> Result<Self, sqlx::Error> {
        // Start a transaction
        let mut tx = pool.begin().await?;

        // Deactivate any existing primary cards for this issuer/member combination
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET is_primary = false
            WHERE issuer_id = $1 AND member_id = $2 AND is_primary = true
            "#,
        )
        .bind(data.issuer_id)
        .bind(data.member_id)
        .execute(&mut *tx)
        .await?;

        // Insert the new card
        let card = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO membership_cards (
                issuer_id, member_id, membership_level_label, membership_confirmed_at,
                verification_comment_id, verification_video_id, snapshot_json,
                qr_payload, qr_signature, is_primary
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, true)
            RETURNING *
            "#,
        )
        .bind(data.issuer_id)
        .bind(data.member_id)
        .bind(&data.membership_level_label)
        .bind(data.membership_confirmed_at)
        .bind(&data.verification_comment_id)
        .bind(&data.verification_video_id)
        .bind(&data.snapshot_json)
        .bind(&data.qr_payload)
        .bind(&data.qr_signature)
        .fetch_one(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(card)
    }

    /// Finds a card by its ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let card = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(card)
    }

    /// Finds the primary (active) card for a member at a specific issuer
    pub async fn find_primary_for_member(
        pool: &PgPool,
        issuer_id: Uuid,
        member_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let card = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1 AND member_id = $2 AND is_primary = true
            "#,
        )
        .bind(issuer_id)
        .bind(member_id)
        .fetch_optional(pool)
        .await?;

        Ok(card)
    }

    /// Lists all cards for a member (across all issuers)
    pub async fn list_by_member(
        pool: &PgPool,
        member_id: Uuid,
        primary_only: bool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let query = if primary_only {
            r#"
            SELECT * FROM membership_cards
            WHERE member_id = $1 AND is_primary = true
            ORDER BY issued_at DESC
            "#
        } else {
            r#"
            SELECT * FROM membership_cards
            WHERE member_id = $1
            ORDER BY issued_at DESC
            "#
        };

        let cards = sqlx::query_as::<_, Self>(query)
            .bind(member_id)
            .fetch_all(pool)
            .await?;

        Ok(cards)
    }

    /// Lists all cards issued by a specific issuer
    pub async fn list_by_issuer(
        pool: &PgPool,
        issuer_id: Uuid,
        primary_only: bool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let query = if primary_only {
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1 AND is_primary = true
            ORDER BY issued_at DESC
            "#
        } else {
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1
            ORDER BY issued_at DESC
            "#
        };

        let cards = sqlx::query_as::<_, Self>(query)
            .bind(issuer_id)
            .fetch_all(pool)
            .await?;

        Ok(cards)
    }

    /// Marks a card as non-primary (effectively deactivating it)
    pub async fn set_primary_status(
        pool: &PgPool,
        id: Uuid,
        is_primary: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET is_primary = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(is_primary)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Counts total cards issued by an issuer
    pub async fn count_by_issuer(pool: &PgPool, issuer_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM membership_cards WHERE issuer_id = $1 AND is_primary = true
            "#,
        )
        .bind(issuer_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }
}
