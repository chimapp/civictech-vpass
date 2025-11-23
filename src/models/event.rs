use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Event {
    pub id: Uuid,
    pub issuer_id: Uuid,
    pub event_name: String,
    pub event_description: Option<String>,
    pub event_date: NaiveDate,
    pub event_location: Option<String>,
    pub verifier_ref: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateEventData {
    pub issuer_id: Uuid,
    pub event_name: String,
    pub event_description: Option<String>,
    pub event_date: NaiveDate,
    pub event_location: Option<String>,
    pub verifier_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateEventData {
    pub event_name: Option<String>,
    pub event_description: Option<String>,
    pub event_date: Option<NaiveDate>,
    pub event_location: Option<String>,
    pub verifier_ref: Option<String>,
}

impl Event {
    /// Create a new event
    pub async fn create(pool: &PgPool, data: CreateEventData) -> Result<Self, sqlx::Error> {
        let event = sqlx::query_as::<_, Event>(
            r#"
            INSERT INTO events (issuer_id, event_name, event_description, event_date, event_location, verifier_ref)
            VALUES ($1, $2, $3, $4, $5, $6)
            RETURNING *
            "#,
        )
        .bind(data.issuer_id)
        .bind(data.event_name)
        .bind(data.event_description)
        .bind(data.event_date)
        .bind(data.event_location)
        .bind(data.verifier_ref)
        .fetch_one(pool)
        .await?;

        Ok(event)
    }

    /// Find event by ID
    pub async fn find_by_id(pool: &PgPool, id: Uuid) -> Result<Option<Self>, sqlx::Error> {
        let event = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(pool)
        .await?;

        Ok(event)
    }

    /// List events by issuer
    pub async fn list_by_issuer(
        pool: &PgPool,
        issuer_id: Uuid,
        active_only: bool,
    ) -> Result<Vec<Self>, sqlx::Error> {
        let query = if active_only {
            r#"
            SELECT * FROM events
            WHERE issuer_id = $1 AND is_active = TRUE
            ORDER BY event_date DESC
            "#
        } else {
            r#"
            SELECT * FROM events
            WHERE issuer_id = $1
            ORDER BY event_date DESC
            "#
        };

        let events = sqlx::query_as::<_, Event>(query)
            .bind(issuer_id)
            .fetch_all(pool)
            .await?;

        Ok(events)
    }

    /// List all active events (across all issuers)
    pub async fn list_active(pool: &PgPool) -> Result<Vec<Self>, sqlx::Error> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE is_active = TRUE
            ORDER BY event_date DESC
            "#,
        )
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// List upcoming events for an issuer
    pub async fn list_upcoming(pool: &PgPool, issuer_id: Uuid) -> Result<Vec<Self>, sqlx::Error> {
        let events = sqlx::query_as::<_, Event>(
            r#"
            SELECT * FROM events
            WHERE issuer_id = $1
              AND is_active = TRUE
              AND event_date >= CURRENT_DATE
            ORDER BY event_date ASC
            "#,
        )
        .bind(issuer_id)
        .fetch_all(pool)
        .await?;

        Ok(events)
    }

    /// Update an event
    pub async fn update(
        pool: &PgPool,
        id: Uuid,
        data: UpdateEventData,
    ) -> Result<Self, sqlx::Error> {
        // Build dynamic update query based on which fields are provided
        let mut query = String::from("UPDATE events SET ");
        let mut updates = Vec::new();
        let mut bind_count = 1;

        if data.event_name.is_some() {
            updates.push(format!("event_name = ${}", bind_count));
            bind_count += 1;
        }
        if data.event_description.is_some() {
            updates.push(format!("event_description = ${}", bind_count));
            bind_count += 1;
        }
        if data.event_date.is_some() {
            updates.push(format!("event_date = ${}", bind_count));
            bind_count += 1;
        }
        if data.event_location.is_some() {
            updates.push(format!("event_location = ${}", bind_count));
            bind_count += 1;
        }
        if data.verifier_ref.is_some() {
            updates.push(format!("verifier_ref = ${}", bind_count));
            bind_count += 1;
        }

        if updates.is_empty() {
            // No fields to update, just return existing event
            return Self::find_by_id(pool, id)
                .await?
                .ok_or(sqlx::Error::RowNotFound);
        }

        query.push_str(&updates.join(", "));
        query.push_str(&format!(" WHERE id = ${} RETURNING *", bind_count));

        let mut query_builder = sqlx::query_as::<_, Event>(&query);

        if let Some(name) = data.event_name {
            query_builder = query_builder.bind(name);
        }
        if let Some(desc) = data.event_description {
            query_builder = query_builder.bind(desc);
        }
        if let Some(date) = data.event_date {
            query_builder = query_builder.bind(date);
        }
        if let Some(location) = data.event_location {
            query_builder = query_builder.bind(location);
        }
        if let Some(verifier_ref) = data.verifier_ref {
            query_builder = query_builder.bind(verifier_ref);
        }

        query_builder = query_builder.bind(id);

        let event = query_builder.fetch_one(pool).await?;

        Ok(event)
    }

    /// Deactivate an event (soft delete)
    pub async fn deactivate(pool: &PgPool, id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE events
            SET is_active = FALSE
            WHERE id = $1
            "#,
        )
        .bind(id)
        .execute(pool)
        .await?;

        Ok(())
    }
}
