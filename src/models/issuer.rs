use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct CardIssuer {
    pub id: Uuid,
    pub platform: String, // Always "youtube" for MVP
    pub youtube_channel_id: String,
    pub channel_handle: Option<String>,
    pub channel_name: String,
    pub verification_video_id: String,
    pub default_membership_label: String,
    pub vc_uid: Option<String>, // Taiwan Digital Wallet VC UID
    pub members_only_video_id: Option<String>, // For membership verification
    pub verification_method: String, // "video" or "comment"
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateIssuerData {
    pub youtube_channel_id: String,
    pub channel_handle: Option<String>,
    pub channel_name: String,
    pub verification_video_id: String,
    pub default_membership_label: String,
    pub vc_uid: Option<String>,
}

impl CardIssuer {
    /// Creates a new card issuer (YouTube channel)
    pub async fn create(pool: &PgPool, data: CreateIssuerData) -> Result<Self, sqlx::Error> {
        let issuer = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO card_issuers (
                platform, youtube_channel_id, channel_handle, channel_name,
                verification_video_id, default_membership_label, vc_uid
            )
            VALUES ('youtube', $1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(&data.youtube_channel_id)
        .bind(&data.channel_handle)
        .bind(&data.channel_name)
        .bind(&data.verification_video_id)
        .bind(&data.default_membership_label)
        .bind(&data.vc_uid)
        .fetch_one(pool)
        .await?;

        Ok(issuer)
    }

    /// Finds an issuer by their internal ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let issuer = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM card_issuers WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(issuer)
    }

    /// Finds an issuer by their YouTube channel ID
    pub async fn find_by_youtube_channel_id(
        pool: &PgPool,
        channel_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let issuer = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM card_issuers
            WHERE youtube_channel_id = $1 AND is_active = TRUE
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(channel_id)
        .fetch_optional(pool)
        .await?;

        Ok(issuer)
    }

    /// Lists all active issuers
    pub async fn list_active(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        let issuers = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM card_issuers
            WHERE is_active = true
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(issuers)
    }

    /// Updates the verification video ID for an issuer
    pub async fn update_verification_video(
        pool: &PgPool,
        id: Uuid,
        video_id: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE card_issuers
            SET verification_video_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(video_id)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Updates issuer active status
    pub async fn set_active_status(
        pool: &PgPool,
        id: Uuid,
        is_active: bool,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE card_issuers
            SET is_active = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(is_active)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Updates issuer channel information
    pub async fn update_channel_info(
        pool: &PgPool,
        id: Uuid,
        channel_name: Option<String>,
        channel_handle: Option<String>,
        default_membership_label: Option<String>,
        vc_uid: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE card_issuers
            SET
                channel_name = COALESCE($2, channel_name),
                channel_handle = COALESCE($3, channel_handle),
                default_membership_label = COALESCE($4, default_membership_label),
                vc_uid = COALESCE($5, vc_uid),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(channel_name)
        .bind(channel_handle)
        .bind(default_membership_label)
        .bind(vc_uid)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Updates members-only video ID for background verification
    pub async fn update_members_only_video(
        pool: &PgPool,
        id: Uuid,
        members_only_video_id: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE card_issuers
            SET members_only_video_id = $2, updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(members_only_video_id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
