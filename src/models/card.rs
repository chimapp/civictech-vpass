use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use sqlx::{FromRow, PgPool, Type};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, Type, PartialEq)]
#[sqlx(type_name = "card_status", rename_all = "lowercase")]
pub enum CardStatus {
    Active,
    Expired,
    Revoked,
    Suspended,
    Deleted,
}

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
    pub status: CardStatus,
    pub expires_at: Option<DateTime<Utc>>,
    pub last_verified_at: Option<DateTime<Utc>>,
    pub verification_failures: i32,
    pub deleted_at: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,

    // Taiwan Digital Wallet integration (merged from wallet_qr_codes table)
    pub wallet_transaction_id: Option<String>,
    pub wallet_qr_code: Option<String>,
    pub wallet_deep_link: Option<String>,
    pub wallet_cid: Option<String>,
    pub wallet_scanned_at: Option<DateTime<Utc>>,
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
}

impl MembershipCard {
    /// Creates a new membership card
    /// Automatically marks existing cards as deleted for the same issuer/member pair
    /// and sets expiration (30 days from now)
    pub async fn create(pool: &PgPool, data: CreateCardData) -> Result<Self, sqlx::Error> {
        use chrono::Duration;

        // Start a transaction
        let mut tx = pool.begin().await?;

        // Mark any existing non-deleted cards as deleted for this issuer/member combination
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET status = 'deleted', deleted_at = NOW()
            WHERE issuer_id = $1 AND member_id = $2 AND status != 'deleted'
            "#,
        )
        .bind(data.issuer_id)
        .bind(data.member_id)
        .execute(&mut *tx)
        .await?;

        // Calculate initial expiration (30 days from now)
        let expires_at = chrono::Utc::now() + Duration::days(30);

        // Insert the new card
        let card = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO membership_cards (
                issuer_id, member_id, membership_level_label, membership_confirmed_at,
                verification_comment_id, verification_video_id, snapshot_json,
                status, expires_at, verification_failures
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, 'active', $8, 0)
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
        .bind(expires_at)
        .fetch_one(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(card)
    }

    /// Checks if the card has expired
    /// Returns true if expires_at exists and is in the past
    pub fn is_expired(&self) -> bool {
        match self.expires_at {
            Some(expiration) => expiration < Utc::now(),
            None => false,
        }
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

    /// Finds the active card for a member at a specific issuer
    pub async fn find_active_for_member(
        pool: &PgPool,
        issuer_id: Uuid,
        member_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let card = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1 AND member_id = $2
              AND status = 'active'
            ORDER BY issued_at DESC
            LIMIT 1
            "#,
        )
        .bind(issuer_id)
        .bind(member_id)
        .fetch_optional(pool)
        .await?;

        Ok(card)
    }

    /// Finds active AND unexpired cards for a member at a specific issuer
    /// Used for duplicate card prevention (FR-006 + FR-006a)
    pub async fn find_active_unexpired_cards(
        pool: &PgPool,
        issuer_id: Uuid,
        member_id: Uuid,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let cards = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1 AND member_id = $2
              AND status = 'active'
              AND (expires_at IS NULL OR expires_at > NOW())
            ORDER BY issued_at DESC
            "#,
        )
        .bind(issuer_id)
        .bind(member_id)
        .fetch_all(pool)
        .await?;

        Ok(cards)
    }

    /// Lists all non-deleted cards for a member (across all issuers)
    pub async fn list_by_member(pool: &PgPool, member_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        let cards = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE member_id = $1 AND status != 'deleted'
            ORDER BY issued_at DESC
            "#,
        )
        .bind(member_id)
        .fetch_all(pool)
        .await?;

        Ok(cards)
    }

    /// Lists all non-deleted cards issued by a specific issuer
    pub async fn list_by_issuer(pool: &PgPool, issuer_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        let cards = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE issuer_id = $1 AND status != 'deleted'
            ORDER BY issued_at DESC
            "#,
        )
        .bind(issuer_id)
        .fetch_all(pool)
        .await?;

        Ok(cards)
    }

    /// Updates card status
    pub async fn set_status(
        pool: &PgPool,
        id: Uuid,
        status: CardStatus,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET status = $2
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(status)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Soft deletes a card by setting status to 'deleted' and recording deletion timestamp
    pub async fn soft_delete(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET status = 'deleted', deleted_at = NOW()
            WHERE id = $1 AND status != 'deleted'
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Extends card expiration and resets verification failures
    pub async fn extend_expiration(pool: &PgPool, id: Uuid, days: i64) -> Result<(), sqlx::Error> {
        use chrono::Duration;

        let new_expires_at = chrono::Utc::now() + Duration::days(days);

        sqlx::query(
            r#"
            UPDATE membership_cards
            SET expires_at = $2,
                last_verified_at = NOW(),
                verification_failures = 0
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(new_expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Increments verification failure count and updates last_verified_at
    pub async fn increment_verification_failure(
        pool: &PgPool,
        id: Uuid,
    ) -> Result<i32, sqlx::Error> {
        let result: (i32,) = sqlx::query_as(
            r#"
            UPDATE membership_cards
            SET verification_failures = verification_failures + 1,
                last_verified_at = NOW()
            WHERE id = $1
            RETURNING verification_failures
            "#,
        )
        .bind(id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }

    /// Counts total active cards issued by an issuer
    pub async fn count_by_issuer(pool: &PgPool, issuer_id: Uuid) -> Result<i64, sqlx::Error> {
        let result: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM membership_cards
            WHERE issuer_id = $1 AND status = 'active'
            "#,
        )
        .bind(issuer_id)
        .fetch_one(pool)
        .await?;

        Ok(result.0)
    }

    /// Finds cards that need verification (active cards not verified in last 24 hours)
    pub async fn find_cards_needing_verification(
        pool: &PgPool,
        limit: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let cards = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE status = 'active'
              AND (last_verified_at IS NULL OR last_verified_at < NOW() - INTERVAL '24 hours')
            ORDER BY last_verified_at ASC NULLS FIRST
            LIMIT $1
            "#,
        )
        .bind(limit)
        .fetch_all(pool)
        .await?;

        Ok(cards)
    }

    // ========== Taiwan Digital Wallet Operations ==========

    /// Updates wallet QR data for this card
    pub async fn set_wallet_qr(
        pool: &PgPool,
        card_id: Uuid,
        transaction_id: String,
        qr_code: String,
        deep_link: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET wallet_transaction_id = $2,
                wallet_qr_code = $3,
                wallet_deep_link = $4
            WHERE id = $1
            "#,
        )
        .bind(card_id)
        .bind(transaction_id)
        .bind(qr_code)
        .bind(deep_link)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Finds a card by wallet transaction ID
    pub async fn find_by_wallet_transaction_id(
        pool: &PgPool,
        transaction_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let card = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM membership_cards
            WHERE wallet_transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_optional(pool)
        .await?;

        Ok(card)
    }

    /// Marks wallet as scanned with CID
    pub async fn mark_wallet_scanned(
        pool: &PgPool,
        card_id: Uuid,
        cid: String,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE membership_cards
            SET wallet_cid = $2,
                wallet_scanned_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(card_id)
        .bind(cid)
        .execute(pool)
        .await?;

        Ok(())
    }
}
