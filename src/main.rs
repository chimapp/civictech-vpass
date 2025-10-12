use axum::{routing::get, Router};
use std::net::SocketAddr;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "vpass=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    tracing::info!("Starting VPass server...");

    // TODO: T011 - Load configuration from environment
    // let config = vpass::config::Config::from_env()?;

    // TODO: T012 - Create database pool
    // let pool = vpass::db::create_pool(&config.database_url).await?;

    // TODO: T009 - Run database migrations
    // vpass::db::run_migrations(&pool).await?;

    // TODO: T015 - Build Axum application with router
    let app = Router::new()
        .route("/", get(|| async { "VPass API - Coming Soon" }))
        .route("/health", get(|| async { "OK" }));

    // TODO: T016 - Add session middleware
    // TODO: T017 - Add authentication middleware
    // TODO: T041 - Serve static files from web/static
    // TODO: T053 - Setup cron scheduler

    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    tracing::info!("Listening on {}", addr);

    // Start server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C signal handler");
    tracing::info!("Shutdown signal received, cleaning up...");
}

// TODO: T015 - Implement full router setup with all endpoints
// - /auth/{platform}/login
// - /auth/{platform}/callback
// - /auth/logout
// - /auth/session
// - /cards/claim
// - /cards/{card_id}
// - /cards/my-cards
// - /cards/{card_id}/qr
// - /verify/scan
// - /verify/history
// - /issuers
