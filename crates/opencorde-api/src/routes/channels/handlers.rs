//! # Channel Route Handlers
//! HTTP request handlers for channel endpoints.
//!
//! ## Endpoints
//! - POST /api/v1/servers/{server_id}/channels — Create channel in server
//! - GET /api/v1/servers/{server_id}/channels — List channels in server
//! - PATCH /api/v1/channels/{id} — Update channel
//! - DELETE /api/v1/channels/{id} — Delete channel
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::channel_repo
//! - opencorde_core::Snowflake
//! - crate::middleware::auth::AuthUser
//! - crate::AppState

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{patch, post},
};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::channel_repo;
use tracing::instrument;

use crate::{
    AppState,
    error::ApiError,
    middleware::auth::AuthUser,
    routes::{moderation::audit_mod::log_mod_action, permission_check},
};
use opencorde_core::Snowflake;
use serde_json::json;

use super::{
    types::{ChannelResponse, CreateChannelRequest, UpdateChannelRequest},
    validation::{parse_snowflake_id, validate_channel_name, validate_channel_type},
};

/// Build the channels router with all CRUD endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/channels",
            post(create_channel).get(list_channels),
        )
        .route(
            "/api/v1/channels/{id}",
            patch(update_channel).delete(delete_channel),
        )
}

/// Convert ChannelRow to ChannelResponse.
fn channel_row_to_response(row: channel_repo::ChannelRow) -> ChannelResponse {
    ChannelResponse {
        id: row.id.to_string(),
        server_id: row.server_id.to_string(),
        name: row.name,
        channel_type: row.channel_type,
        topic: row.topic,
        position: row.position,
        parent_id: row.parent_id.map(|id| id.to_string()),
        nsfw: row.nsfw,
        slowmode_delay: row.slowmode_delay,
        e2ee_enabled: row.e2ee_enabled,
        created_at: row.created_at,
    }
}

/// POST /api/v1/servers/{server_id}/channels — Create a new channel in a server.
///
/// Requires authentication.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateChannelRequest>,
) -> Result<(StatusCode, Json<ChannelResponse>), ApiError> {
    tracing::info!(name = %req.name, "creating channel");

    // Parse server ID
    let server_id_sf = parse_snowflake_id(&server_id)?;
    tracing::debug!(server_id = server_id_sf.as_i64(), "parsed server id");

    // Require MANAGE_CHANNELS permission
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id_sf,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    // Validate channel name
    validate_channel_name(&req.name)?;

    // Default to Text channel if type not specified
    let channel_type = req.channel_type.unwrap_or(0);

    // Validate channel type
    validate_channel_type(channel_type)?;

    // Parse parent_id if provided
    let parent_id = if let Some(parent_str) = &req.parent_id {
        Some(parse_snowflake_id(parent_str)?)
    } else {
        None
    };

    tracing::debug!(
        channel_type = channel_type,
        parent_id = ?parent_id,
        nsfw = req.nsfw.unwrap_or(false),
        "validated channel parameters"
    );

    // Generate Snowflake ID for channel
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(1, 1);
    let channel_id = generator.next_id();

    tracing::debug!(
        channel_id = channel_id.as_i64(),
        "generated channel snowflake"
    );

    // Get nsfw flag, default to false
    let nsfw = req.nsfw.unwrap_or(false);

    // Create channel in database
    let channel_row = channel_repo::create_channel(
        &state.db,
        channel_id,
        server_id_sf,
        &req.name,
        channel_type,
        nsfw,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        channel_id = channel_row.id,
        server_id = channel_row.server_id,
        "channel created"
    );
    log_mod_action(
        &state,
        server_id_sf,
        auth.user_id,
        "channel.create",
        channel_row.id,
    )
    .await;

    let response = channel_row_to_response(channel_row);
    let event = json!({"type":"ChannelCreate","data":{"channel":&response}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for ChannelCreate event");
    }
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/servers/{server_id}/channels — List all channels in a server.
///
/// Requires authentication. Returns channels ordered by position.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_channels(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<ChannelResponse>>, ApiError> {
    tracing::info!("listing server channels");

    // Parse server ID
    let server_id_sf = parse_snowflake_id(&server_id)?;
    tracing::debug!(server_id = server_id_sf.as_i64(), "parsed server id");

    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    let channels = channel_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = channels.len(), "channels fetched");

    let mut responses = Vec::new();
    for channel in channels {
        let channel_id = Snowflake::new(channel.id);
        if permission_check::require_channel_perm(
            &state.db,
            auth.user_id,
            channel_id,
            Permissions::VIEW_CHANNEL,
        )
        .await
        .is_ok()
        {
            responses.push(channel_row_to_response(channel));
        }
    }

    Ok(Json(responses))
}

/// PATCH /api/v1/channels/{id} — Update a channel.
///
/// Requires authentication.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateChannelRequest>,
) -> Result<Json<ChannelResponse>, ApiError> {
    tracing::debug!(id = %id, "updating channel");

    // Parse channel ID
    let channel_id = parse_snowflake_id(&id)?;
    tracing::debug!(channel_id = channel_id.as_i64(), "parsed channel id");

    // Require MANAGE_CHANNELS permission (channel-level check covers server owner bypass)
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    // Fetch current channel
    let channel = channel_repo::get_by_id(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(channel_id = channel_id.as_i64(), "channel not found");
            ApiError::NotFound("channel not found".into())
        })?;

    tracing::debug!(channel_id = channel.id, "channel fetched");

    // Determine which fields to update
    let update_name = req.name.as_deref().unwrap_or(&channel.name);

    // Validate name if provided
    if let Some(name) = &req.name {
        validate_channel_name(name)?;
    }

    let update_topic = req.topic.as_deref().or(channel.topic.as_deref());

    tracing::debug!(
        channel_id = channel_id.as_i64(),
        name = %update_name,
        nsfw = ?req.nsfw,
        "validated channel updates"
    );

    // Clamp slowmode_delay to valid range (0-21600 seconds = 6 hours)
    let slowmode_delay = req.slowmode_delay.map(|d| d.clamp(0, 21600));

    // Update channel
    channel_repo::update_channel(
        &state.db,
        channel_id,
        update_name,
        channel_repo::ChannelUpdate {
            topic: update_topic,
            parent_id: None,
            nsfw: req.nsfw,
            slowmode_delay,
            e2ee_enabled: req.e2ee_enabled,
        },
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(channel_id = channel.id, "channel updated");
    log_mod_action(
        &state,
        Snowflake::new(channel.server_id),
        auth.user_id,
        "channel.update",
        channel_id.as_i64(),
    )
    .await;

    // Update position if provided
    if let Some(position) = req.position {
        channel_repo::update_position(&state.db, channel_id, position)
            .await
            .map_err(ApiError::Database)?;

        tracing::debug!(
            channel_id = channel.id,
            position = position,
            "position updated"
        );
    }

    // Fetch updated channel
    let updated = channel_repo::get_by_id(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::error!(
                channel_id = channel_id.as_i64(),
                "channel disappeared after update"
            );
            ApiError::Internal(anyhow::anyhow!("channel disappeared after update"))
        })?;

    let response = channel_row_to_response(updated);
    let event = json!({"type":"ChannelUpdate","data":{"channel":&response}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for ChannelUpdate event");
    }
    Ok(Json(response))
}

/// DELETE /api/v1/channels/{id} — Delete a channel.
///
/// Requires authentication.
/// Returns 204 No Content on success.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_channel(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!(id = %id, "deleting channel");

    // Parse channel ID
    let channel_id = parse_snowflake_id(&id)?;
    tracing::debug!(channel_id = channel_id.as_i64(), "parsed channel id");

    // Require MANAGE_CHANNELS permission
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id,
        Permissions::MANAGE_CHANNELS,
    )
    .await?;

    // Fetch channel to verify it exists
    let channel = channel_repo::get_by_id(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(channel_id = channel_id.as_i64(), "channel not found");
            ApiError::NotFound("channel not found".into())
        })?;

    tracing::debug!(channel_id = channel.id, "channel fetched");

    // Delete channel (cascades to messages)
    channel_repo::delete_channel(&state.db, channel_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(channel_id = channel.id, "channel deleted");
    log_mod_action(
        &state,
        Snowflake::new(channel.server_id),
        auth.user_id,
        "channel.delete",
        channel_id.as_i64(),
    )
    .await;
    let event = json!({"type":"ChannelDelete","data":{"server_id":channel.server_id.to_string(),"channel_id":channel_id.as_i64().to_string()}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for ChannelDelete event");
    }
    Ok(StatusCode::NO_CONTENT)
}
