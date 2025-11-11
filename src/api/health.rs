use axum::{extract::State, http::StatusCode, Json};
use secrecy::ExposeSecret;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::time::Instant;

use crate::api::middleware::session::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub timestamp: String,
    pub version: String,
    pub dependencies: DependencyStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DependencyStatus {
    pub database: ServiceHealth,
    pub wallet_api: ServiceHealth,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ServiceHealth {
    pub status: String,
    pub response_time_ms: u128,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Health check endpoint
/// Returns 200 if all dependencies are healthy, 503 if any are down
pub async fn health_check(State(state): State<AppState>) -> (StatusCode, Json<HealthResponse>) {
    let start = Instant::now();

    // Check database connectivity
    let db_health = check_database(&state.pool).await;

    // Check wallet API availability (if configured)
    let wallet_health = if let (Some(issuer_api_url), Some(issuer_access_token)) =
        (&state.config.issuer_api_url, &state.config.issuer_access_token)
    {
        check_wallet_api(issuer_api_url, issuer_access_token.expose_secret()).await
    } else {
        ServiceHealth {
            status: "not_configured".to_string(),
            response_time_ms: 0,
            error: Some("Wallet API credentials not configured".to_string()),
        }
    };

    // Determine overall health status
    let all_healthy = db_health.status == "healthy"
        && (wallet_health.status == "healthy" || wallet_health.status == "not_configured");

    let status_code = if all_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = HealthResponse {
        status: if all_healthy {
            "healthy".to_string()
        } else {
            "unhealthy".to_string()
        },
        timestamp: chrono::Utc::now().to_rfc3339(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        dependencies: DependencyStatus {
            database: db_health,
            wallet_api: wallet_health,
        },
    };

    tracing::info!(
        status = %response.status,
        duration_ms = start.elapsed().as_millis(),
        "Health check completed"
    );

    (status_code, Json(response))
}

/// Check database connectivity
async fn check_database(pool: &PgPool) -> ServiceHealth {
    let start = Instant::now();

    match sqlx::query("SELECT 1").fetch_one(pool).await {
        Ok(_) => ServiceHealth {
            status: "healthy".to_string(),
            response_time_ms: start.elapsed().as_millis(),
            error: None,
        },
        Err(e) => ServiceHealth {
            status: "unhealthy".to_string(),
            response_time_ms: start.elapsed().as_millis(),
            error: Some(format!("Database error: {}", e)),
        },
    }
}

/// Check wallet API availability
async fn check_wallet_api(api_base_url: &str, access_token: &str) -> ServiceHealth {
    let start = Instant::now();

    match crate::services::wallet_qr::check_wallet_health(api_base_url, access_token).await {
        Ok(_) => ServiceHealth {
            status: "healthy".to_string(),
            response_time_ms: start.elapsed().as_millis(),
            error: None,
        },
        Err(e) => ServiceHealth {
            status: "unhealthy".to_string(),
            response_time_ms: start.elapsed().as_millis(),
            error: Some(format!("Wallet API error: {}", e)),
        },
    }
}
