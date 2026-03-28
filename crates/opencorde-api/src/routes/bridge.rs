//! # Route: Discord Bridge Mappings
//! CRUD for `bridge_channel_mappings` — links OpenCorde channels to Discord channels.
//!
//! ## Endpoints
//! - GET  /api/v1/servers/{server_id}/bridge/mappings — List all mappings for a server
//! - POST /api/v1/servers/{server_id}/bridge/mappings — Create a new mapping
//! - PATCH /api/v1/servers/{server_id}/bridge/mappings/{id} — Enable / disable mapping
//! - DELETE /api/v1/servers/{server_id}/bridge/mappings/{id} — Remove a mapping
//!
//! ## Permissions
//! All endpoints require the caller to be the server owner.
//!
//! ## Depends On
//! - sqlx (database queries against bridge_channel_mappings)
//! - crate::middleware::auth::AuthUser
//! - opencorde_db::repos::server_repo (owner check)
//! - crate::routes::helpers::{parse_snowflake, check_server_owner}

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, patch, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::helpers::{check_server_owner, parse_snowflake};
use opencorde_db::repos::server_repo;

/// Serialized bridge mapping returned to the client.
#[derive(Debug, Serialize)]
pub struct BridgeMappingResponse {
    pub id: i64,
    pub discord_guild_id: String,
    pub discord_channel_id: String,
    pub discord_webhook_id: Option<String>,
    pub opencorde_channel_id: String,
    pub enabled: bool,
    pub last_discord_msg_id: i64,
    pub last_opencorde_msg_id: i64,
    pub created_at: DateTime<Utc>,
}

/// Request body to create a new bridge mapping.
#[derive(Debug, Deserialize)]
pub struct CreateBridgeMappingRequest {
    /// Discord guild (server) ID as a string
    pub discord_guild_id: String,
    /// Discord channel ID to bridge
    pub discord_channel_id: String,
    /// Discord webhook ID for OpenCorde→Discord direction (optional)
    pub discord_webhook_id: Option<String>,
    /// Discord webhook token (required if webhook_id is set)
    pub discord_webhook_token: Option<String>,
    /// OpenCorde channel ID to bridge to
    pub opencorde_channel_id: String,
}

/// Request body to update a mapping (enable/disable only).
#[derive(Debug, Deserialize)]
pub struct UpdateBridgeMappingRequest {
    pub enabled: bool,
}

/// Build the bridge router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/bridge/mappings",
            get(list_mappings).post(create_mapping),
        )
        .route(
            "/api/v1/servers/{server_id}/bridge/mappings/{mapping_id}",
            patch(update_mapping).delete(delete_mapping),
        )
}

/// GET /api/v1/servers/{server_id}/bridge/mappings — List bridge mappings for this server.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_mappings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<BridgeMappingResponse>>, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_server(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or(ApiError::NotFound)?;
    check_server_owner(auth.user_id, server.owner_id)?;

    let rows = sqlx::query(
        "SELECT id, discord_guild_id, discord_channel_id, discord_webhook_id,
                discord_webhook_token, opencorde_channel_id, enabled,
                last_discord_msg_id, last_opencorde_msg_id, created_at
         FROM bridge_channel_mappings
         WHERE opencorde_server_id = $1
         ORDER BY created_at ASC",
    )
    .bind(sid.as_i64())
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    let mappings = rows
        .iter()
        .map(|r| BridgeMappingResponse {
            id: r.get::<i64, _>("id"),
            discord_guild_id: r.get::<i64, _>("discord_guild_id").to_string(),
            discord_channel_id: r.get::<i64, _>("discord_channel_id").to_string(),
            discord_webhook_id: r
                .get::<Option<i64>, _>("discord_webhook_id")
                .map(|v| v.to_string()),
            opencorde_channel_id: r.get::<i64, _>("opencorde_channel_id").to_string(),
            enabled: r.get("enabled"),
            last_discord_msg_id: r.get("last_discord_msg_id"),
            last_opencorde_msg_id: r.get("last_opencorde_msg_id"),
            created_at: r.get("created_at"),
        })
        .collect();

    Ok(Json(mappings))
}

/// POST /api/v1/servers/{server_id}/bridge/mappings — Create a new bridge mapping.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn create_mapping(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateBridgeMappingRequest>,
) -> Result<(StatusCode, Json<BridgeMappingResponse>), ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_server(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or(ApiError::NotFound)?;
    check_server_owner(auth.user_id, server.owner_id)?;

    // Parse Discord IDs from strings
    let discord_guild_id: i64 = req
        .discord_guild_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid discord_guild_id".into()))?;
    let discord_channel_id: i64 = req
        .discord_channel_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid discord_channel_id".into()))?;
    let opencorde_channel_id = parse_snowflake(&req.opencorde_channel_id)?;

    let discord_webhook_id: Option<i64> = req
        .discord_webhook_id
        .as_deref()
        .map(|s| s.parse::<i64>().map_err(|_| ApiError::BadRequest("invalid discord_webhook_id".into())))
        .transpose()?;

    let row = sqlx::query(
        "INSERT INTO bridge_channel_mappings
            (discord_guild_id, discord_channel_id, discord_webhook_id, discord_webhook_token,
             opencorde_server_id, opencorde_channel_id, enabled)
         VALUES ($1, $2, $3, $4, $5, $6, TRUE)
         RETURNING id, discord_guild_id, discord_channel_id, discord_webhook_id,
                   discord_webhook_token, opencorde_channel_id, enabled,
                   last_discord_msg_id, last_opencorde_msg_id, created_at",
    )
    .bind(discord_guild_id)
    .bind(discord_channel_id)
    .bind(discord_webhook_id)
    .bind(&req.discord_webhook_token)
    .bind(sid.as_i64())
    .bind(opencorde_channel_id.as_i64())
    .fetch_one(&state.db)
    .await
    .map_err(|e| {
        if e.to_string().contains("unique") {
            ApiError::Conflict("channel already bridged".into())
        } else {
            ApiError::Internal(e.into())
        }
    })?;

    let mapping = BridgeMappingResponse {
        id: row.get::<i64, _>("id"),
        discord_guild_id: row.get::<i64, _>("discord_guild_id").to_string(),
        discord_channel_id: row.get::<i64, _>("discord_channel_id").to_string(),
        discord_webhook_id: row
            .get::<Option<i64>, _>("discord_webhook_id")
            .map(|v| v.to_string()),
        opencorde_channel_id: row.get::<i64, _>("opencorde_channel_id").to_string(),
        enabled: row.get("enabled"),
        last_discord_msg_id: row.get("last_discord_msg_id"),
        last_opencorde_msg_id: row.get("last_opencorde_msg_id"),
        created_at: row.get("created_at"),
    };

    tracing::info!(
        server_id = sid.as_i64(),
        discord_channel_id,
        "bridge mapping created"
    );

    Ok((StatusCode::CREATED, Json(mapping)))
}

/// PATCH /api/v1/servers/{server_id}/bridge/mappings/{mapping_id} — Enable or disable a mapping.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_mapping(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, mapping_id)): Path<(String, i64)>,
    Json(req): Json<UpdateBridgeMappingRequest>,
) -> Result<Json<BridgeMappingResponse>, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_server(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or(ApiError::NotFound)?;
    check_server_owner(auth.user_id, server.owner_id)?;

    let row = sqlx::query(
        "UPDATE bridge_channel_mappings
         SET enabled = $1
         WHERE id = $2 AND opencorde_server_id = $3
         RETURNING id, discord_guild_id, discord_channel_id, discord_webhook_id,
                   discord_webhook_token, opencorde_channel_id, enabled,
                   last_discord_msg_id, last_opencorde_msg_id, created_at",
    )
    .bind(req.enabled)
    .bind(mapping_id)
    .bind(sid.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?
    .ok_or(ApiError::NotFound)?;

    Ok(Json(BridgeMappingResponse {
        id: row.get::<i64, _>("id"),
        discord_guild_id: row.get::<i64, _>("discord_guild_id").to_string(),
        discord_channel_id: row.get::<i64, _>("discord_channel_id").to_string(),
        discord_webhook_id: row
            .get::<Option<i64>, _>("discord_webhook_id")
            .map(|v| v.to_string()),
        opencorde_channel_id: row.get::<i64, _>("opencorde_channel_id").to_string(),
        enabled: row.get("enabled"),
        last_discord_msg_id: row.get("last_discord_msg_id"),
        last_opencorde_msg_id: row.get("last_opencorde_msg_id"),
        created_at: row.get("created_at"),
    }))
}

/// DELETE /api/v1/servers/{server_id}/bridge/mappings/{mapping_id} — Remove a bridge mapping.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_mapping(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, mapping_id)): Path<(String, i64)>,
) -> Result<StatusCode, ApiError> {
    let sid = parse_snowflake(&server_id)?;
    let server = server_repo::get_server(&state.db, sid)
        .await
        .map_err(ApiError::Database)?
        .ok_or(ApiError::NotFound)?;
    check_server_owner(auth.user_id, server.owner_id)?;

    let result = sqlx::query(
        "DELETE FROM bridge_channel_mappings WHERE id = $1 AND opencorde_server_id = $2",
    )
    .bind(mapping_id)
    .bind(sid.as_i64())
    .execute(&state.db)
    .await
    .map_err(|e| ApiError::Internal(e.into()))?;

    if result.rows_affected() == 0 {
        return Err(ApiError::NotFound);
    }

    tracing::info!(mapping_id, server_id = sid.as_i64(), "bridge mapping deleted");
    Ok(StatusCode::NO_CONTENT)
}
