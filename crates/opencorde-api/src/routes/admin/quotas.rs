//! # Admin Quota Handlers
//! Endpoints for managing per-user and instance-wide storage quotas.
//!
//! ## Endpoints
//! - GET    /api/v1/admin/quotas                  — Instance quota settings + per-user overrides
//! - PUT    /api/v1/admin/quotas                  — Update instance-wide quota defaults
//! - PUT    /api/v1/admin/users/{user_id}/quota   — Set per-user override
//! - DELETE /api/v1/admin/users/{user_id}/quota   — Clear per-user override
//!
//! ## Depends On
//! - crate::middleware::auth::AuthUser
//! - crate::AppState
//! - opencorde_db::repos::user_repo

use axum::{
    Json,
    extract::{Path, State},
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::user_repo;

use super::handlers::is_admin;

// ------------------------------------------------------------------
// Types
// ------------------------------------------------------------------

/// Instance-wide storage quota settings.
#[derive(Debug, Serialize, Deserialize)]
pub struct InstanceQuotaSettings {
    /// Default per-user storage quota in bytes (e.g. 104857600 = 100 MB).
    pub default_user_quota_bytes: i64,
    /// Instance-wide total storage cap in bytes (null = no cap).
    pub instance_total_storage_cap_bytes: Option<i64>,
}

/// Per-user quota override entry.
#[derive(Debug, Serialize)]
pub struct UserQuotaOverride {
    pub user_id: String,
    pub username: String,
    pub quota_bytes: Option<i64>,
    pub used_bytes: i64,
}

/// Full quota config response for GET /api/v1/admin/quotas.
#[derive(Debug, Serialize)]
pub struct AdminQuotasResponse {
    /// Instance-wide settings.
    pub instance: InstanceQuotaSettings,
    /// Users with quota overrides (non-NULL storage_quota_bytes).
    pub overrides: Vec<UserQuotaOverride>,
}

/// Request body for PUT /api/v1/admin/quotas.
#[derive(Debug, Deserialize)]
pub struct UpdateInstanceQuotaRequest {
    pub default_user_quota_bytes: i64,
    pub instance_total_storage_cap_bytes: Option<i64>,
}

/// Request body for PUT /api/v1/admin/users/{user_id}/quota.
#[derive(Debug, Deserialize)]
pub struct SetUserQuotaRequest {
    /// Quota in bytes. Pass null to clear the override.
    pub quota_bytes: Option<i64>,
}

// ------------------------------------------------------------------
// GET /api/v1/admin/quotas
// ------------------------------------------------------------------

/// GET /api/v1/admin/quotas — Return instance quota settings and per-user overrides.
///
/// Requires admin role.
#[tracing::instrument(skip(state, auth))]
pub async fn get_quotas(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<AdminQuotasResponse>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching quota settings");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted quota read");
        return Err(ApiError::Forbidden);
    }

    let settings = user_repo::get_instance_settings(&state.db)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let instance = match settings {
        Some(s) => InstanceQuotaSettings {
            default_user_quota_bytes: s.default_user_quota_bytes,
            instance_total_storage_cap_bytes: s.instance_total_storage_cap_bytes,
        },
        None => InstanceQuotaSettings {
            default_user_quota_bytes: 104_857_600, // 100 MB default
            instance_total_storage_cap_bytes: None,
        },
    };

    // List users with explicit quota overrides (storage_quota_bytes IS NOT NULL).
    let rows = sqlx::query(
        "SELECT id, username, storage_quota_bytes, storage_used_bytes_cache \
         FROM users WHERE storage_quota_bytes IS NOT NULL \
         ORDER BY username",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let overrides: Vec<UserQuotaOverride> = rows
        .iter()
        .map(|r| {
            let id: i64 = r.get("id");
            UserQuotaOverride {
                user_id: id.to_string(),
                username: r.get("username"),
                quota_bytes: r.get("storage_quota_bytes"),
                used_bytes: r.get("storage_used_bytes_cache"),
            }
        })
        .collect();

    tracing::info!(
        default_quota = instance.default_user_quota_bytes,
        override_count = overrides.len(),
        "admin: quota settings fetched"
    );

    Ok(Json(AdminQuotasResponse {
        instance,
        overrides,
    }))
}

// ------------------------------------------------------------------
// PUT /api/v1/admin/quotas
// ------------------------------------------------------------------

/// PUT /api/v1/admin/quotas — Update instance-wide quota defaults.
///
/// Requires admin role. Changes take effect immediately for new upload checks.
#[tracing::instrument(skip(state, auth, body))]
pub async fn update_quotas(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(body): Json<UpdateInstanceQuotaRequest>,
) -> Result<Json<InstanceQuotaSettings>, ApiError> {
    tracing::info!(
        user_id = %auth.user_id,
        default = body.default_user_quota_bytes,
        cap = ?body.instance_total_storage_cap_bytes,
        "admin: updating quota settings"
    );

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted quota update");
        return Err(ApiError::Forbidden);
    }

    if body.default_user_quota_bytes <= 0 {
        return Err(ApiError::BadRequest(
            "default_user_quota_bytes must be a positive integer".into(),
        ));
    }

    user_repo::upsert_instance_quota_settings(
        &state.db,
        body.default_user_quota_bytes,
        body.instance_total_storage_cap_bytes,
    )
    .await
    .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let resp = InstanceQuotaSettings {
        default_user_quota_bytes: body.default_user_quota_bytes,
        instance_total_storage_cap_bytes: body.instance_total_storage_cap_bytes,
    };

    tracing::info!("admin: quota settings updated");
    Ok(Json(resp))
}

// ------------------------------------------------------------------
// PUT /api/v1/admin/users/{user_id}/quota
// ------------------------------------------------------------------

/// PUT /api/v1/admin/users/{user_id}/quota — Set a per-user storage quota override.
///
/// Requires admin role. Setting quota_bytes to null clears the override.
#[tracing::instrument(skip(state, auth, body))]
pub async fn set_user_quota(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
    Json(body): Json<SetUserQuotaRequest>,
) -> Result<Json<UserQuotaOverride>, ApiError> {
    tracing::info!(
        admin_id = %auth.user_id,
        target_user_id = %user_id,
        quota = ?body.quota_bytes,
        "admin: setting user quota override"
    );

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted user quota set");
        return Err(ApiError::Forbidden);
    }

    let uid: i64 = user_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid user_id".into()))?;
    let target_sf = Snowflake::new(uid);

    // Verify user exists
    let user = user_repo::get_by_id(&state.db, target_sf)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;

    user_repo::set_storage_quota_override(&state.db, target_sf, body.quota_bytes)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    let resp = UserQuotaOverride {
        user_id: user.id.to_string(),
        username: user.username,
        quota_bytes: body.quota_bytes,
        used_bytes: user.storage_used_bytes_cache,
    };

    tracing::info!(target_user_id = %user_id, "admin: user quota override set");
    Ok(Json(resp))
}

// ------------------------------------------------------------------
// DELETE /api/v1/admin/users/{user_id}/quota
// ------------------------------------------------------------------

/// DELETE /api/v1/admin/users/{user_id}/quota — Clear a per-user storage quota override.
///
/// Requires admin role. User will fall back to the instance default after this.
#[tracing::instrument(skip(state, auth))]
pub async fn clear_user_quota(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> Result<(), ApiError> {
    tracing::info!(
        admin_id = %auth.user_id,
        target_user_id = %user_id,
        "admin: clearing user quota override"
    );

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted user quota clear");
        return Err(ApiError::Forbidden);
    }

    let uid: i64 = user_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid user_id".into()))?;
    let target_sf = Snowflake::new(uid);

    user_repo::set_storage_quota_override(&state.db, target_sf, None)
        .await
        .map_err(|e| ApiError::InternalServerError(e.to_string()))?;

    tracing::info!(target_user_id = %user_id, "admin: user quota override cleared");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instance_quota_settings_serialization() {
        let settings = InstanceQuotaSettings {
            default_user_quota_bytes: 104_857_600,
            instance_total_storage_cap_bytes: Some(10_737_418_240),
        };
        let json = serde_json::to_string(&settings).unwrap();
        assert!(json.contains("104857600"));
        assert!(json.contains("10737418240"));
    }

    #[test]
    fn test_user_quota_override_serialization() {
        let entry = UserQuotaOverride {
            user_id: "12345".to_string(),
            username: "testuser".to_string(),
            quota_bytes: Some(524_288_000),
            used_bytes: 104_857_600,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("testuser"));
        assert!(json.contains("524288000"));
    }
}
