//! GET /api/v1/users/@me/storage — Authenticated user's storage usage and quota.
//!
//! Shows the user their current storage consumption, applicable quota,
//! and percentage used. Requires authentication.

use axum::{Json, extract::State};
use serde::Serialize;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use opencorde_db::repos::user_repo;

/// Storage usage response for an authenticated user.
#[derive(Debug, Serialize)]
pub struct UserStorageInfo {
    /// Total bytes stored across all uploads.
    pub used_bytes: i64,
    /// Applicable quota in bytes (per-user override or instance default).
    pub quota_bytes: i64,
    /// True if the user has a per-user override (not the instance default).
    pub quota_is_override: bool,
    /// Percentage used (0–100). Clamped; 100+ means quota is exceeded.
    pub used_percent: f64,
    /// True if the user has hit or exceeded their quota.
    pub quota_exceeded: bool,
}

/// GET /api/v1/users/@me/storage — Get authenticated user's storage usage and quota.
///
/// Requires valid access token. Returns current storage usage, applicable quota
/// (per-user override or instance default), and percentage used.
#[tracing::instrument(skip(state, auth))]
pub async fn get_my_storage(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<UserStorageInfo>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "user: fetching storage usage");

    // Get fresh usage from files table
    let used_bytes = user_repo::refresh_storage_usage_cache(&state.db, auth.user_id)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    // Get user row to check for per-user override
    let user = user_repo::get_by_id(&state.db, auth.user_id)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    // Determine effective quota: per-user override > instance default
    let (quota_bytes, quota_is_override) = if let Some(override_bytes) = user.storage_quota_bytes {
        (override_bytes, true)
    } else {
        let settings = user_repo::get_instance_settings(&state.db)
            .await
            .map_err(|e| ApiError::InternalServerError(e.to_string()))?;
        let default_quota = settings
            .map(|s| s.default_user_quota_bytes)
            .unwrap_or(104_857_600); // 100 MB fallback
        (default_quota, false)
    };

    let used_percent = if quota_bytes > 0 {
        ((used_bytes as f64 / quota_bytes as f64) * 100.0).min(100.0)
    } else {
        0.0
    };

    let quota_exceeded = used_bytes >= quota_bytes && quota_bytes > 0;

    let info = UserStorageInfo {
        used_bytes,
        quota_bytes,
        quota_is_override,
        used_percent,
        quota_exceeded,
    };

    tracing::info!(
        user_id = %auth.user_id,
        used = used_bytes,
        quota = quota_bytes,
        percent = used_percent,
        "user: storage info fetched"
    );

    Ok(Json(info))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_storage_info_serialization() {
        let info = UserStorageInfo {
            used_bytes: 52_428_800,
            quota_bytes: 104_857_600,
            quota_is_override: false,
            used_percent: 50.0,
            quota_exceeded: false,
        };
        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("52428800"));
        assert!(json.contains("104857600"));
        assert!(json.contains("50.0"));
    }
}
