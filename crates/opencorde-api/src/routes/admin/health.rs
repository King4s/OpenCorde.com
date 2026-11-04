//! # Admin Health Check
//! Comprehensive instance health check covering all core services.
//!
//! ## Endpoints
//! - GET /api/v1/admin/health — Full service health with latency and colour hints
//!
//! ## Depends On
//! - crate::AppState
//! - crate::middleware::auth::AuthUser
//! - opencorde_db (database layer via PgPool)
//! - redis (optional, for connectivity check)
//! - aws_sdk_s3 (storage health)
//! - reqwest (LiveKit HTTP health)
//! - lettre / tokio::net (SMTP connectivity)
//! - sqlx (bridge mapping counts)

use axum::{Json, extract::State};
use sqlx::Row;

use super::handlers::is_admin;
use super::storage;
use super::types::{InstanceHealth, ServiceHealth};
use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

/// Create a service-health struct for a known-ok service.
fn ok_service(detail: String, latency_ms: Option<u128>) -> ServiceHealth {
    ServiceHealth {
        status: "ok".into(),
        detail,
        latency_ms,
        error: None,
        color: "green".into(),
    }
}

/// Create a service-health struct for a degraded/unconfigured service.
fn amber_service(detail: String) -> ServiceHealth {
    ServiceHealth {
        status: "degraded".into(),
        detail,
        latency_ms: None,
        error: None,
        color: "amber".into(),
    }
}

/// Create a service-health struct for a failed service.
fn error_service(detail: String, error: Option<String>, latency_ms: Option<u128>) -> ServiceHealth {
    ServiceHealth {
        status: "error".into(),
        detail,
        latency_ms,
        error,
        color: "red".into(),
    }
}

/// Create a service-health struct for a disabled/unavailable service.
fn gray_service(detail: String) -> ServiceHealth {
    ServiceHealth {
        status: "disabled".into(),
        detail,
        latency_ms: None,
        error: None,
        color: "gray".into(),
    }
}

/// Check database connectivity with a lightweight SELECT 1 and measure latency.
async fn check_database(db: &sqlx::PgPool) -> ServiceHealth {
    let started = std::time::Instant::now();
    match sqlx::query("SELECT 1").fetch_one(db).await {
        Ok(_) => ok_service("Connected".into(), Some(started.elapsed().as_millis())),
        Err(e) => error_service(
            "Database unreachable".into(),
            Some(e.to_string()),
            Some(started.elapsed().as_millis()),
        ),
    }
}

/// Check Redis connectivity via PING if redis_url is configured.
async fn check_redis(state: &AppState) -> ServiceHealth {
    let redis_url = state.config.redis_url.as_str();
    if redis_url.is_empty() {
        return amber_service("Redis not configured".into());
    }

    let started = std::time::Instant::now();
    match redis::Client::open(redis_url) {
        Ok(client) => match redis::aio::ConnectionManager::new(client).await {
            Ok(mut conn) => {
                let result: redis::RedisResult<String> =
                    redis::cmd("PING").query_async(&mut conn).await;
                let latency = started.elapsed().as_millis();
                match result {
                    Ok(_) => ok_service("Connected".into(), Some(latency)),
                    Err(e) => error_service(
                        "Redis PING failed".into(),
                        Some(e.to_string()),
                        Some(latency),
                    ),
                }
            }
            Err(e) => error_service(
                "Redis connection failed".into(),
                Some(e.to_string()),
                Some(started.elapsed().as_millis()),
            ),
        },
        Err(e) => error_service(
            "Redis URL invalid".into(),
            Some(e.to_string()),
            Some(started.elapsed().as_millis()),
        ),
    }
}

/// Check SMTP connectivity by attempting a TCP connection to the SMTP host:port.
async fn check_smtp(state: &AppState) -> ServiceHealth {
    if !state.email_service.is_configured() {
        return amber_service(
            "SMTP not configured (password resets logged instead of sent)".into(),
        );
    }

    let host = match &state.config.smtp_host {
        Some(h) => h.clone(),
        None => return amber_service("SMTP host not set".into()),
    };
    let port = state.config.smtp_port;

    let addr = format!("{}:{}", host, port);
    let started = std::time::Instant::now();
    match tokio::net::TcpStream::connect(&addr).await {
        Ok(_) => ok_service(
            format!("SMTP reachable at {}", addr),
            Some(started.elapsed().as_millis()),
        ),
        Err(e) => error_service(
            format!("SMTP connection failed: {}", addr),
            Some(e.to_string()),
            Some(started.elapsed().as_millis()),
        ),
    }
}

/// Check bridge status by counting enabled/disabled bridge mappings.
async fn check_bridge(db: &sqlx::PgPool) -> ServiceHealth {
    let started = std::time::Instant::now();
    let row = match sqlx::query(
        "SELECT COUNT(*) FILTER (WHERE enabled) as enabled_count, COUNT(*) as total FROM bridge_channel_mappings",
    )
    .fetch_one(db)
    .await
    {
        Ok(r) => r,
        Err(e) => {
            // Table may not exist yet — treat as disabled.
            return gray_service(format!("No bridge mappings (table may not exist: {})", e));
        }
    };

    let enabled: i64 = row.get("enabled_count");
    let total: i64 = row.get("total");
    let latency = Some(started.elapsed().as_millis());

    if total == 0 {
        gray_service("No bridge mappings configured".into())
    } else if enabled == 0 {
        amber_service(format!(
            "{} bridge mapping(s) configured, none enabled",
            total
        ))
    } else {
        ok_service(
            format!("{} bridge mapping(s) enabled ({} total)", enabled, total),
            latency,
        )
    }
}

/// Check background job queue health.
/// Reports real job metrics from the registry.
async fn check_background_jobs(state: &AppState) -> ServiceHealth {
    let metrics = state.job_registry.metrics().await;
    if metrics.total_jobs == 0 {
        gray_service("No background jobs registered".into())
    } else if metrics.failed > 0 {
        amber_service(format!(
            "{} jobs registered, {} running, {} failed",
            metrics.total_jobs, metrics.running, metrics.failed
        ))
    } else {
        ok_service(
            format!(
                "{} jobs registered, {} running, {} idle",
                metrics.total_jobs, metrics.running, metrics.idle
            ),
            None,
        )
    }
}

/// Build a full InstanceHealth from the AppState.
/// Calls all service checks concurrently where possible.
pub async fn get_instance_health(state: &AppState) -> InstanceHealth {
    use std::time::Instant;

    // API is always up if this endpoint responds.
    let api_start = Instant::now();
    let api_health = ok_service(
        "API server is running".into(),
        None, // The overall request latency is measured later.
    );
    let _api_latency = api_start.elapsed();

    // Run all checks concurrently.
    let (db, redis, storage_result, livekit, smtp, bridge, bg_jobs) = tokio::join!(
        check_database(&state.db),
        check_redis(state),
        storage::get_storage_health(state),
        super::handlers::get_livekit_health(state),
        check_smtp(state),
        check_bridge(&state.db),
        check_background_jobs(state),
    );

    // Map storage health into ServiceHealth.
    let storage_health = if storage_result.ok {
        ok_service(
            format!("Storage bucket '{}' reachable", storage_result.bucket),
            storage_result.latency_ms,
        )
    } else {
        error_service(
            format!("Storage bucket '{}' unreachable", storage_result.bucket),
            storage_result.error,
            storage_result.latency_ms,
        )
    };

    let livekit_health = if livekit.ok {
        ok_service("LiveKit local and public endpoints healthy".into(), None)
    } else {
        let mut issues = Vec::new();
        if !livekit.local.ok {
            issues.push(format!(
                "local: {}",
                livekit.local.error.as_deref().unwrap_or("no response")
            ));
        }
        if !livekit.public.ok {
            issues.push(format!(
                "public: {}",
                livekit.public.error.as_deref().unwrap_or("no response")
            ));
        }
        error_service(
            format!("LiveKit degraded: {}", issues.join(", ")),
            None,
            None,
        )
    };

    InstanceHealth {
        api: api_health,
        database: db,
        redis,
        storage: storage_health,
        livekit: livekit_health,
        smtp,
        bridge,
        background_jobs: bg_jobs,
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

/// GET /api/v1/admin/health — Full instance health check.
///
/// Requires admin role. Returns health status for every core service with
/// latency measurements and colour-coding hints for the dashboard.
#[tracing::instrument(skip(state, auth))]
pub async fn admin_health(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<InstanceHealth>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: instance health check");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted health check");
        return Err(ApiError::Forbidden);
    }

    let started = std::time::Instant::now();
    let mut health = get_instance_health(&state).await;
    // Record the actual API response latency.
    health.api.latency_ms = Some(started.elapsed().as_millis());

    tracing::info!(
        api = %health.api.status,
        db = %health.database.status,
        redis = %health.redis.status,
        storage = %health.storage.status,
        livekit = %health.livekit.status,
        smtp = %health.smtp.status,
        bridge = %health.bridge.status,
        bg_jobs = %health.background_jobs.status,
        latency_ms = ?health.api.latency_ms,
        "admin: health check complete"
    );

    Ok(Json(health))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ok_service() {
        let s = ok_service("test ok".into(), Some(5));
        assert_eq!(s.status, "ok");
        assert_eq!(s.detail, "test ok");
        assert_eq!(s.latency_ms, Some(5));
        assert_eq!(s.color, "green");
    }

    #[test]
    fn test_amber_service() {
        let s = amber_service("not configured".into());
        assert_eq!(s.status, "degraded");
        assert_eq!(s.color, "amber");
    }

    #[test]
    fn test_error_service() {
        let s = error_service("fail".into(), Some("reason".into()), Some(12));
        assert_eq!(s.status, "error");
        assert_eq!(s.color, "red");
    }

    #[test]
    fn test_gray_service() {
        let s = gray_service("n/a".into());
        assert_eq!(s.status, "disabled");
        assert_eq!(s.color, "gray");
    }
}
