use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct WalletQrCode {
    pub id: Uuid,
    pub card_id: Uuid,
    pub transaction_id: String,
    pub qr_code: String,
    pub deep_link: Option<String>,
    pub cid: Option<String>,
    pub scanned_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateWalletQrCodeData {
    pub card_id: Uuid,
    pub transaction_id: String,
    pub qr_code: String,
    pub deep_link: Option<String>,
}

impl WalletQrCode {
    /// Creates a new wallet QR code and deactivates any existing active QR codes for the same card
    pub async fn create(pool: &PgPool, data: CreateWalletQrCodeData) -> Result<Self, sqlx::Error> {
        // Start a transaction
        let mut tx = pool.begin().await?;

        // Deactivate any existing active QR codes for this card
        sqlx::query(
            r#"
            UPDATE wallet_qr_codes
            SET is_active = false, updated_at = NOW()
            WHERE card_id = $1 AND is_active = true
            "#,
        )
        .bind(data.card_id)
        .execute(&mut *tx)
        .await?;

        // Insert the new QR code
        let qr_code = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO wallet_qr_codes (
                card_id, transaction_id, qr_code, deep_link, is_active
            )
            VALUES ($1, $2, $3, $4, true)
            RETURNING *
            "#,
        )
        .bind(data.card_id)
        .bind(&data.transaction_id)
        .bind(&data.qr_code)
        .bind(&data.deep_link)
        .fetch_one(&mut *tx)
        .await?;

        // Commit the transaction
        tx.commit().await?;

        Ok(qr_code)
    }

    /// Finds the active wallet QR code for a card
    pub async fn find_active_by_card_id(
        pool: &PgPool,
        card_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let qr_code = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM wallet_qr_codes
            WHERE card_id = $1 AND is_active = true
            "#,
        )
        .bind(card_id)
        .fetch_optional(pool)
        .await?;

        Ok(qr_code)
    }

    /// Finds a wallet QR code by transaction ID
    pub async fn find_by_transaction_id(
        pool: &PgPool,
        transaction_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let qr_code = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM wallet_qr_codes
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_optional(pool)
        .await?;

        Ok(qr_code)
    }

    /// Updates the scan status with CID
    pub async fn mark_as_scanned(pool: &PgPool, id: Uuid, cid: String) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE wallet_qr_codes
            SET cid = $2, scanned_at = NOW(), updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(cid)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Lists all QR codes for a card (for history)
    pub async fn list_by_card_id(pool: &PgPool, card_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        let qr_codes = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM wallet_qr_codes
            WHERE card_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(card_id)
        .fetch_all(pool)
        .await?;

        Ok(qr_codes)
    }
}
