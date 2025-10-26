use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct VerificationSession {
    pub id: Uuid,
    pub event_id: Uuid,
    pub transaction_id: String,
    pub qrcode_image: String, // base64 PNG
    pub auth_uri: String,
    pub status: String, // 'pending', 'completed', 'expired', 'failed'
    pub verify_result: Option<bool>,
    pub result_description: Option<String>,
    pub result_data: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateVerificationSessionData {
    pub event_id: Uuid,
    pub transaction_id: String,
    pub qrcode_image: String,
    pub auth_uri: String,
}

impl VerificationSession {
    /// Creates a new verification session
    ///
    /// QR code expires after 5 minutes
    pub async fn create(
        pool: &PgPool,
        data: CreateVerificationSessionData,
    ) -> Result<Self, sqlx::Error> {
        let now = Utc::now();
        let expires_at = now + Duration::minutes(5);

        let session = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO verification_sessions (
                event_id, transaction_id, qrcode_image, auth_uri,
                status, expires_at
            )
            VALUES ($1, $2, $3, $4, 'pending', $5)
            RETURNING *
            "#,
        )
        .bind(data.event_id)
        .bind(&data.transaction_id)
        .bind(&data.qrcode_image)
        .bind(&data.auth_uri)
        .bind(expires_at)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Finds a session by transaction ID
    pub async fn find_by_transaction_id(
        pool: &PgPool,
        transaction_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let session = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM verification_sessions
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Finds sessions by event ID
    pub async fn find_by_event_id(
        pool: &PgPool,
        event_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let sessions = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM verification_sessions
            WHERE event_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(event_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(sessions)
    }

    /// Updates session with verification result
    pub async fn update_result(
        pool: &PgPool,
        transaction_id: &str,
        verify_result: bool,
        result_description: String,
        result_data: Option<serde_json::Value>,
    ) -> Result<(), sqlx::Error> {
        let now = Utc::now();

        sqlx::query(
            r#"
            UPDATE verification_sessions
            SET
                status = 'completed',
                verify_result = $2,
                result_description = $3,
                result_data = $4,
                completed_at = $5
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(verify_result)
        .bind(result_description)
        .bind(result_data)
        .bind(now)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Marks a session as expired
    pub async fn mark_expired(pool: &PgPool, transaction_id: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE verification_sessions
            SET status = 'expired'
            WHERE transaction_id = $1 AND status = 'pending'
            "#,
        )
        .bind(transaction_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Marks a session as failed
    pub async fn mark_failed(
        pool: &PgPool,
        transaction_id: &str,
        error_message: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE verification_sessions
            SET
                status = 'failed',
                result_description = $2
            WHERE transaction_id = $1
            "#,
        )
        .bind(transaction_id)
        .bind(error_message)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Checks if session is expired
    pub fn is_expired(&self) -> bool {
        Utc::now() > self.expires_at
    }

    /// Checks if session is still pending
    pub fn is_pending(&self) -> bool {
        self.status == "pending" && !self.is_expired()
    }

    /// Counts sessions by event and status
    pub async fn count_by_event_and_status(
        pool: &PgPool,
        event_id: Uuid,
        status: Option<&str>,
    ) -> Result<i64, sqlx::Error> {
        let count = if let Some(status_filter) = status {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM verification_sessions
                WHERE event_id = $1 AND status = $2
                "#,
            )
            .bind(event_id)
            .bind(status_filter)
            .fetch_one(pool)
            .await?
        } else {
            sqlx::query_scalar::<_, i64>(
                r#"
                SELECT COUNT(*) FROM verification_sessions
                WHERE event_id = $1
                "#,
            )
            .bind(event_id)
            .fetch_one(pool)
            .await?
        };

        Ok(count)
    }

    /// Cleanup old expired sessions (older than 24 hours)
    pub async fn cleanup_old_sessions(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let cutoff = Utc::now() - Duration::hours(24);

        let result = sqlx::query(
            r#"
            DELETE FROM verification_sessions
            WHERE created_at < $1 AND status IN ('expired', 'failed')
            "#,
        )
        .bind(cutoff)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }
}
