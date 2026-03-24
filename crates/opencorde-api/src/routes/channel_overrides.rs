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

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use opencorde_db::repos::{channel_override_repo, channel_repo, member_repo, role_repo};
use serde::{Deserialize, Serialize};
use tracing::instrument;

const PERM_ADMINISTRATOR: i64 = 0x08;
const PERM_MANAGE_CHANNELS: i64 = 0x10;

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

/// Check that the calling user is a member of `server_id`.
async fn require_member(
    state: &AppState,
    user_id: opencorde_core::Snowflake,
    server_id: opencorde_core::Snowflake,
) -> Result<(), ApiError> {
    member_repo::get_member(&state.db, user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Forbidden)?;
    Ok(())
}

/// Check that the calling user has MANAGE_CHANNELS or ADMINISTRATOR on the server.
async fn require_manage_channels(
    state: &AppState,
    user_id: opencorde_core::Snowflake,
    server_id: opencorde_core::Snowflake,
) -> Result<(), ApiError> {
    let member_roles = member_repo::list_member_roles(&state.db, user_id, server_id)
        .await
        .map_err(ApiError::Database)?;

    for mr in &member_roles {
        let role_sf = opencorde_core::Snowflake::new(mr.role_id);
        if let Some(role) = role_repo::get_by_id(&state.db, role_sf)
            .await
            .map_err(ApiError::Database)?
        {
            let p = role.permissions;
            if (p & PERM_ADMINISTRATOR) != 0 || (p & PERM_MANAGE_CHANNELS) != 0 {
                return Ok(());
            }
        }
    }

    tracing::warn!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        "user lacks MANAGE_CHANNELS permission"
    );
    Err(ApiError::Forbidden)
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
    let server_sf = get_server_id_for_channel(&state, channel_sf).await?;
    require_member(&state, auth.user_id, server_sf).await?;

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
    let server_sf = get_server_id_for_channel(&state, channel_sf).await?;
    require_manage_channels(&state, auth.user_id, server_sf).await?;

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
    let server_sf = get_server_id_for_channel(&state, channel_sf).await?;
    require_manage_channels(&state, auth.user_id, server_sf).await?;

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
