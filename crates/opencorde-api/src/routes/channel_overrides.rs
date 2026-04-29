//! # Route: Channel Permission Overrides
//! Per-channel permission override management.
//!
//! ## Endpoints
//! - GET    /api/v1/channels/{id}/permissions           — list overrides
//! - PUT    /api/v1/channels/{id}/permissions/{type}/{targetId} — upsert
//! - DELETE /api/v1/channels/{id}/permissions/{type}/{targetId} — remove
//!
//! ## Auth
//! All endpoints require server membership.
//! Write endpoints require MANAGE_CHANNELS (bit 0x10) or ADMINISTRATOR (bit 0x08).
//!
//! ## Depends On
//! - opencorde_db::repos::{channel_override_repo, channel_repo, member_repo, role_repo}
//! - crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers}

use crate::{
    AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers,
    routes::permission_check,
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::{channel_override_repo, channel_repo};
use serde::{Deserialize, Serialize};
use tracing::instrument;

/// Response shape for a single permission override.
#[derive(Debug, Serialize)]
pub struct OverrideResponse {
    pub id: String,
    pub channel_id: String,
    pub target_type: String,
    pub target_id: String,
    pub allow: u64,
    pub deny: u64,
}

/// Request body for upsert.
#[derive(Debug, Deserialize)]
pub struct UpsertOverrideRequest {
    pub allow: u64,
    pub deny: u64,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/channels/{channel_id}/permissions",
            get(list_overrides),
        )
        .route(
            "/api/v1/channels/{channel_id}/permissions/{target_type}/{target_id}",
            put(upsert_override).delete(delete_override),
        )
}

/// Convert a repo row into a JSON-serialisable response.
fn row_to_response(row: channel_override_repo::OverrideRow) -> OverrideResponse {
    OverrideResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        target_type: row.target_type,
        target_id: row.target_id.to_string(),
        allow: row.allow_bits as u64,
        deny: row.deny_bits as u64,
    }
}

/// Validate that target_type is "role" or "member".
fn validate_target_type(t: &str) -> Result<(), ApiError> {
    if t != "role" && t != "member" {
        return Err(ApiError::BadRequest(
            "target_type must be 'role' or 'member'".into(),
        ));
    }
    Ok(())
}

/// Resolve the server_id for a channel, returning NotFound if absent.
async fn get_server_id_for_channel(
    state: &AppState,
    channel_id: opencorde_core::Snowflake,
) -> Result<opencorde_core::Snowflake, ApiError> {
    let channel = channel_repo::get_by_id(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("channel not found".into()))?;
    Ok(opencorde_core::Snowflake::new(channel.server_id))
}

/// GET /api/v1/channels/{channel_id}/permissions
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_overrides(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<OverrideResponse>>, ApiError> {
    tracing::info!("listing channel permission overrides");
    let channel_sf = helpers::parse_snowflake(&channel_id)?;
    get_server_id_for_channel(&state, channel_sf).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    let rows = channel_override_repo::list_for_channel(&state.db, channel_sf.as_i64())
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = rows.len(), "overrides listed");
    Ok(Json(rows.into_iter().map(row_to_response).collect()))
}

/// PUT /api/v1/channels/{channel_id}/permissions/{target_type}/{target_id}
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn upsert_override(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, target_type, target_id)): Path<(String, String, String)>,
    Json(req): Json<UpsertOverrideRequest>,
) -> Result<(StatusCode, Json<OverrideResponse>), ApiError> {
    tracing::info!("upserting channel permission override");
    validate_target_type(&target_type)?;
    let channel_sf = helpers::parse_snowflake(&channel_id)?;
    let target_sf = helpers::parse_snowflake(&target_id)?;
    get_server_id_for_channel(&state, channel_sf).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_sf,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    let row = channel_override_repo::upsert(
        &state.db,
        channel_sf.as_i64(),
        &target_type,
        target_sf.as_i64(),
        req.allow as i64,
        req.deny as i64,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(override_id = row.id, "override upserted");
    Ok((StatusCode::OK, Json(row_to_response(row))))
}

/// DELETE /api/v1/channels/{channel_id}/permissions/{target_type}/{target_id}
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_override(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, target_type, target_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("deleting channel permission override");
    validate_target_type(&target_type)?;
    let channel_sf = helpers::parse_snowflake(&channel_id)?;
    let target_sf = helpers::parse_snowflake(&target_id)?;
    get_server_id_for_channel(&state, channel_sf).await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_sf,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    channel_override_repo::delete(
        &state.db,
        channel_sf.as_i64(),
        &target_type,
        target_sf.as_i64(),
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!("override deleted");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_target_type_valid() {
        assert!(validate_target_type("role").is_ok());
        assert!(validate_target_type("member").is_ok());
    }

    #[test]
    fn test_validate_target_type_invalid() {
        assert!(validate_target_type("user").is_err());
        assert!(validate_target_type("").is_err());
    }
}
