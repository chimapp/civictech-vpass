use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct OAuthSession {
    pub id: Uuid,
    pub member_id: Uuid,
    pub access_token: Vec<u8>,          // BYTEA - encrypted
    pub refresh_token: Option<Vec<u8>>, // BYTEA - encrypted
    pub token_scope: String,
    pub token_expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateSessionData {
    pub member_id: Uuid,
    pub access_token: Vec<u8>,
    pub refresh_token: Option<Vec<u8>>,
    pub token_scope: String,
    pub token_expires_at: DateTime<Utc>,
}

impl OAuthSession {
    /// Creates a new OAuth session with encrypted tokens
    pub async fn create(pool: &PgPool, data: CreateSessionData) -> Result<Self, sqlx::Error> {
        let session = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO oauth_sessions (
                member_id, access_token, refresh_token, token_scope, token_expires_at
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING *
            "#,
        )
        .bind(data.member_id)
        .bind(&data.access_token)
        .bind(&data.refresh_token)
        .bind(&data.token_scope)
        .bind(data.token_expires_at)
        .fetch_one(pool)
        .await?;

        Ok(session)
    }

    /// Finds a session by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let session = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM oauth_sessions WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Finds the most recent session for a member
    pub async fn find_by_member_id(
        pool: &PgPool,
        member_id: Uuid,
    ) -> Result<Option<Self>, sqlx::Error> {
        let session = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM oauth_sessions
            WHERE member_id = $1
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(member_id)
        .fetch_optional(pool)
        .await?;

        Ok(session)
    }

    /// Updates tokens for a session (e.g., after refresh)
    pub async fn update_tokens(
        pool: &PgPool,
        id: Uuid,
        access_token: Vec<u8>,
        refresh_token: Option<Vec<u8>>,
        expires_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE oauth_sessions
            SET
                access_token = $2,
                refresh_token = $3,
                token_expires_at = $4,
                last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(&access_token)
        .bind(&refresh_token)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Updates the last_used_at timestamp
    pub async fn touch(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE oauth_sessions
            SET last_used_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Deletes a session
    pub async fn delete(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM oauth_sessions WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Deletes all sessions for a member
    pub async fn delete_by_member_id(pool: &PgPool, member_id: Uuid) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_sessions WHERE member_id = $1
            "#,
        )
        .bind(member_id)
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Deletes expired sessions (cleanup task)
    pub async fn delete_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            DELETE FROM oauth_sessions WHERE token_expires_at < NOW()
            "#,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Checks if the session is expired
    pub fn is_expired(&self) -> bool {
        self.token_expires_at < Utc::now()
    }
}
