use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Member {
    pub id: Uuid,
    pub youtube_user_id: String,
    pub default_display_name: String,
    pub avatar_url: Option<String>,
    pub locale: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct CreateMemberData {
    pub youtube_user_id: String,
    pub default_display_name: String,
    pub avatar_url: Option<String>,
    pub locale: Option<String>,
}

impl Member {
    /// Creates a new member record
    pub async fn create(pool: &PgPool, data: CreateMemberData) -> Result<Self, sqlx::Error> {
        let member = sqlx::query_as::<_, Self>(
            r#"
            INSERT INTO members (youtube_user_id, default_display_name, avatar_url, locale)
            VALUES ($1, $2, $3, $4)
            RETURNING *
            "#,
        )
        .bind(&data.youtube_user_id)
        .bind(&data.default_display_name)
        .bind(&data.avatar_url)
        .bind(&data.locale)
        .fetch_one(pool)
        .await?;

        Ok(member)
    }

    /// Finds a member by their internal ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let member = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM members WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(member)
    }

    /// Finds a member by their YouTube user ID
    pub async fn find_by_youtube_user_id(
        pool: &PgPool,
        youtube_user_id: &str,
    ) -> Result<Option<Self>, sqlx::Error> {
        let member = sqlx::query_as::<_, Self>(
            r#"
            SELECT * FROM members WHERE youtube_user_id = $1
            "#,
        )
        .bind(youtube_user_id)
        .fetch_optional(pool)
        .await?;

        Ok(member)
    }

    /// Updates member profile information
    pub async fn update_profile(
        pool: &PgPool,
        id: Uuid,
        display_name: Option<String>,
        avatar_url: Option<String>,
        locale: Option<String>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE members
            SET
                default_display_name = COALESCE($2, default_display_name),
                avatar_url = COALESCE($3, avatar_url),
                locale = COALESCE($4, locale),
                updated_at = NOW()
            WHERE id = $1
            "#,
        )
        .bind(id)
        .bind(display_name)
        .bind(avatar_url)
        .bind(locale)
        .execute(pool)
        .await?;

        Ok(())
    }

    /// Finds or creates a member by YouTube user ID
    pub async fn find_or_create(
        pool: &PgPool,
        data: CreateMemberData,
    ) -> Result<Self, sqlx::Error> {
        // First try to find existing member
        if let Some(existing) = Self::find_by_youtube_user_id(pool, &data.youtube_user_id).await? {
            // Update profile if needed
            Self::update_profile(
                pool,
                existing.id,
                Some(data.default_display_name),
                data.avatar_url,
                data.locale,
            )
            .await?;

            // Fetch updated member
            Self::find_by_id(pool, existing.id)
                .await?
                .ok_or(sqlx::Error::RowNotFound)
        } else {
            // Create new member
            Self::create(pool, data).await
        }
    }
}
