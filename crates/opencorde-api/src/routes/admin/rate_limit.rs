//! # Admin Rate Limit Handlers
//! Endpoints for viewing and updating runtime rate limit configuration.
//!
//! ## Endpoints
//! - GET  /api/v1/admin/rate-limits — Return current rate limit config
//! - PUT  /api/v1/admin/rate-limits — Update rate limit config (live, no restart)
//!
//! ## Depends On
//! - crate::middleware::rate_limit::{RateLimitConfig, RateLimitState}
//! - crate::middleware::auth::AuthUser
//! - crate::AppState

use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

use super::handlers::is_admin;

/// Request body for PUT /api/v1/admin/rate-limits.
#[derive(Debug, Deserialize)]
pub struct UpdateRateLimitRequest {
    /// New sustained requests per second per IP (min 1)
    pub requests_per_second: u32,
    /// New burst capacity per IP (min = requests_per_second)
    pub burst_size: u32,
    /// Whether to enable or disable rate limiting
    pub enabled: bool,
}

/// Response for GET and PUT /api/v1/admin/rate-limits.
#[derive(Debug, Serialize)]
pub struct RateLimitResponse {
    pub requests_per_second: u32,
    pub burst_size: u32,
    pub enabled: bool,
}

/// GET /api/v1/admin/rate-limits — Return current rate limit settings.
///
/// Requires admin role.
#[tracing::instrument(skip(state, auth))]
pub async fn get_rate_limits(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<RateLimitResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching rate limit config");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted rate limit read");
        return Err(ApiError::Forbidden);
    }

    let config = state.rate_limit_state.config.read().await;
    let resp = RateLimitResponse {
        requests_per_second: config.requests_per_second,
        burst_size: config.burst_size,
        enabled: config.enabled,
    };
    tracing::info!(
        rps = resp.requests_per_second,
        burst = resp.burst_size,
        enabled = resp.enabled,
        "admin: rate limit config fetched"
    );
    Ok(Json(resp))
}

/// PUT /api/v1/admin/rate-limits — Update rate limit settings at runtime.
///
/// Requires admin role. Changes take effect immediately without restart.
#[tracing::instrument(skip(state, auth, body))]
pub async fn update_rate_limits(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateRateLimitRequest>,
) -> Result<Json<RateLimitResponse>, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        rps = body.requests_per_second,
        burst = body.burst_size,
        enabled = body.enabled,
        "admin: updating rate limit config"
    );

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted rate limit update");
        return Err(ApiError::Forbidden);
    }

    if body.requests_per_second == 0 {
        return Err(ApiError::BadRequest("requests_per_second must be >= 1".into()));
    }

    {
        let mut config = state.rate_limit_state.config.write().await;
        config.requests_per_second = body.requests_per_second;
        config.burst_size = body.burst_size.max(body.requests_per_second);
        config.enabled = body.enabled;
    }
    state.rate_limit_state.rebuild_limiter().await;

    let config = state.rate_limit_state.config.read().await;
    let resp = RateLimitResponse {
        requests_per_second: config.requests_per_second,
        burst_size: config.burst_size,
        enabled: config.enabled,
    };
    tracing::info!(
        rps = resp.requests_per_second,
        burst = resp.burst_size,
        enabled = resp.enabled,
        "admin: rate limit config updated"
    );
    Ok(Json(resp))
}
