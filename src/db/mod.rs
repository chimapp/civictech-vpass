use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub mod schema;

pub async fn create_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    // TODO: T012 - Implement database connection pooling
    // - Configure pool size (10-50 connections)
    // - Set connection timeout
    // - Implement health check
    // - Add connection retry logic
    PgPoolOptions::new()
        .max_connections(20)
        .acquire_timeout(Duration::from_secs(3))
        .connect(database_url)
        .await
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), sqlx::Error> {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .map_err(|e| sqlx::Error::Migrate(Box::new(e)))
}
