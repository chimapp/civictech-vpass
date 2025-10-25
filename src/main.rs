use axum::{
    routing::{get, get_service},
    Router,
};
use secrecy::ExposeSecret;
use std::{net::SocketAddr, path::Path};
use tower_http::{services::ServeDir, trace::TraceLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use vpass::api::middleware::session::{create_session_layer, AppState};
use vpass::config::Config;
use vpass::db;

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

    // Load configuration
    let config = Config::from_env()?;
    tracing::info!("Configuration loaded successfully");

    // Create database pool
    let pool = db::create_pool(&config.database_url).await?;
    tracing::info!("Database pool created");

    // Run migrations
    db::run_migrations(&pool).await?;
    tracing::info!("Database migrations completed");

    // Create session layer
    let session_secret = config.session_secret.expose_secret().as_bytes();
    let session_layer = create_session_layer(pool.clone(), session_secret, &config.base_url).await?;
    tracing::info!("Session layer initialized");

    // Build application state
    let state = AppState {
        pool: pool.clone(),
        config: config.clone(),
    };

    // Serve static assets from web/static
    let static_routes = Router::new().nest_service(
        "/static",
        get_service(ServeDir::new(Path::new("web").join("static"))),
    );

    // Build router
    let app = Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(vpass::api::auth::router())
        .merge(vpass::api::cards::router())
        .merge(vpass::api::issuers::router())
        .merge(static_routes)
        .layer(session_layer)
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = SocketAddr::from(([127, 0, 0, 1], config.port));
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
