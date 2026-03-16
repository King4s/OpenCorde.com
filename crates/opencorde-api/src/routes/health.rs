//! # Route: Health Check
//! Simple health check endpoint for liveness/readiness probes.
//!
//! ## Endpoints
//! - GET /api/v1/health — System and database health status
//!
//! ## Features
//! - Database connectivity check
//! - Version information
//! - Structured logging of health check results
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db (database layer)

use axum::extract::State;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;

use crate::AppState;

/// Health check response body.
#[derive(Serialize, Debug)]
pub struct HealthResponse {
    /// Overall system status ("ok" or error description)
    pub status: String,
    /// Database connectivity status
    pub database: String,
    /// API version from Cargo.toml
    pub version: String,
}

/// Create the health check router.
pub fn router() -> Router<AppState> {
    Router::new().route("/api/v1/health", get(health_check))
}

/// Health check endpoint handler.
///
/// Checks:
/// 1. Database connectivity via simple SELECT 1 query
/// 2. Returns version from Cargo.toml
///
/// Returns 200 OK with health details regardless of database state,
/// but logs database errors for debugging.
#[tracing::instrument(skip(state))]
async fn health_check(State(state): State<AppState>) -> Json<HealthResponse> {
    tracing::debug!("processing health check");

    // Check database connectivity
    let db_status = match opencorde_db::health_check(&state.db).await {
        Ok(()) => {
            tracing::debug!("database health check passed");
            "connected".to_string()
        }
        Err(e) => {
            tracing::warn!(error = %e, "database health check failed");
            format!("error: {}", e)
        }
    };

    let response = HealthResponse {
        status: "ok".to_string(),
        database: db_status,
        version: env!("CARGO_PKG_VERSION").to_string(),
    };

    tracing::debug!(database_status = %response.database, "health check response");
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response_serialization() {
        let response = HealthResponse {
            status: "ok".to_string(),
            database: "connected".to_string(),
            version: "0.1.0".to_string(),
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("ok"));
        assert!(json.contains("connected"));
        assert!(json.contains("0.1.0"));
    }
}
