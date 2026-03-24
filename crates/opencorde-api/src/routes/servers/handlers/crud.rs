//! # Server CRUD Operations
//! Create, read, update, and delete handlers for servers.

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use opencorde_core::Snowflake;
use opencorde_db::repos::{channel_repo, member_repo, server_repo};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

use super::super::{
    types::{CreateServerRequest, ServerResponse, UpdateServerRequest},
    validation::validate_server_name,
};

/// Build the servers router with all CRUD endpoints.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/servers", post(create_server).get(list_servers))
        .route(
            "/api/v1/servers/{id}",
            get(get_server).patch(update_server).delete(delete_server),
        )
}

/// Convert ServerRow to ServerResponse.
fn server_row_to_response(row: server_repo::ServerRow) -> ServerResponse {
    ServerResponse {
        id: row.id.to_string(),
        name: row.name,
        owner_id: row.owner_id.to_string(),
        icon_url: row.icon_url,
        banner_url: row.banner_url,
        description: row.description,
        vanity_url: row.vanity_url,
        verification_level: row.verification_level,
        explicit_content_filter: row.explicit_content_filter,
        default_notifications: row.default_notifications,
        system_channel_id: row.system_channel_id.map(|id| id.to_string()),
        rules_channel_id: row.rules_channel_id.map(|id| id.to_string()),
        created_at: row.created_at,
    }
}

/// POST /api/v1/servers — Create a new server.
///
/// Requires authentication. The authenticated user becomes the owner.
/// Auto-adds owner as server member.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<CreateServerRequest>,
) -> Result<(StatusCode, Json<ServerResponse>), ApiError> {
    tracing::info!(name = %req.name, "creating server");

    // Validate server name
    validate_server_name(&req.name)?;

    // Generate Snowflake ID for server
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(1, 0);
    let server_id = generator.next_id();

    tracing::debug!(server_id = server_id.as_i64(), "generated server snowflake");

    // Create server in database
    let server_row = server_repo::create_server(&state.db, server_id, &req.name, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        server_id = server_row.id,
        name = %server_row.name,
        "server created"
    );

    // Auto-add owner as server member
    member_repo::add_member(&state.db, auth.user_id, server_id)
        .await
        .map_err(|e| {
            tracing::error!(
                server_id = server_id.as_i64(),
                user_id = auth.user_id.as_i64(),
                error = %e,
                "failed to add owner as server member"
            );
            ApiError::Database(e)
        })?;

    tracing::debug!(server_id = server_row.id, "owner added as server member");

    // Create default #general text channel
    let mut channel_generator = opencorde_core::snowflake::SnowflakeGenerator::new(2, 0);
    let general_channel_id = channel_generator.next_id();
    channel_repo::create_channel(&state.db, general_channel_id, server_id, "general", 0, false)
        .await
        .map_err(|e| {
            tracing::error!(
                server_id = server_id.as_i64(),
                channel_id = general_channel_id.as_i64(),
                error = %e,
                "failed to create default #general channel"
            );
            ApiError::Database(e)
        })?;

    tracing::info!(
        server_id = server_row.id,
        channel_id = general_channel_id.as_i64(),
        "default #general channel created"
    );

    let response = server_row_to_response(server_row);
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/servers — List all servers the user is a member of.
///
/// Requires authentication.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ServerResponse>>, ApiError> {
    tracing::info!("listing user servers");

    let servers = server_repo::list_by_user(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = servers.len(), "servers fetched");

    let responses: Vec<ServerResponse> = servers.into_iter().map(server_row_to_response).collect();

    Ok(Json(responses))
}

/// GET /api/v1/servers/{id} — Get server details by ID.
///
/// Requires authentication. Returns 404 if server not found.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<Json<ServerResponse>, ApiError> {
    tracing::debug!(id = %id, "fetching server details");

    // Parse path parameter as i64, then convert to Snowflake
    let server_id: Snowflake = id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid server id".into()))
        .and_then(|id| {
            if id > 0 {
                Ok(Snowflake::new(id))
            } else {
                Err(ApiError::BadRequest("server id must be positive".into()))
            }
        })?;

    tracing::debug!(server_id = server_id.as_i64(), "parsed server id");

    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(server_id = server_id.as_i64(), "server not found");
            ApiError::NotFound("server not found".into())
        })?;

    tracing::info!(server_id = server.id, "server fetched");

    Ok(Json(server_row_to_response(server)))
}

/// PATCH /api/v1/servers/{id} — Update server details (owner only).
///
/// Requires authentication and ownership of the server.
/// Returns 403 if not the owner.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateServerRequest>,
) -> Result<Json<ServerResponse>, ApiError> {
    tracing::debug!(id = %id, "updating server");

    // Parse path parameter as i64, then convert to Snowflake
    let server_id: Snowflake = id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid server id".into()))
        .and_then(|id| {
            if id > 0 {
                Ok(Snowflake::new(id))
            } else {
                Err(ApiError::BadRequest("server id must be positive".into()))
            }
        })?;

    // Fetch server from database
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(server_id = server_id.as_i64(), "server not found");
            ApiError::NotFound("server not found".into())
        })?;

    // Check ownership
    if server.owner_id != auth.user_id.as_i64() {
        tracing::warn!(
            server_id = server_id.as_i64(),
            user_id = auth.user_id.as_i64(),
            "user is not server owner"
        );
        return Err(ApiError::Forbidden);
    }

    tracing::debug!(server_id = server.id, "ownership verified");

    // Determine which fields to update (fall back to current values if not provided)
    let update_name = req.name.as_deref().unwrap_or(&server.name);
    let update_description = req.description.as_deref().or(server.description.as_deref());
    let update_verification  = req.verification_level.unwrap_or(server.verification_level);
    let update_content_filter = req.explicit_content_filter.unwrap_or(server.explicit_content_filter);
    let update_notifications = req.default_notifications.unwrap_or(server.default_notifications);
    let update_vanity = req.vanity_url.as_deref().or(server.vanity_url.as_deref());
    let update_system_ch = req.system_channel_id
        .as_deref()
        .map(|s| s.parse::<i64>().ok())
        .unwrap_or(server.system_channel_id);
    let update_rules_ch = req.rules_channel_id
        .as_deref()
        .map(|s| s.parse::<i64>().ok())
        .unwrap_or(server.rules_channel_id);

    // Validate name if provided
    if let Some(name) = &req.name {
        validate_server_name(name)?;
    }

    // Validate moderation ranges
    if !(0..=4).contains(&update_verification) {
        return Err(ApiError::BadRequest("verification_level must be 0–4".into()));
    }
    if !(0..=2).contains(&update_content_filter) {
        return Err(ApiError::BadRequest("explicit_content_filter must be 0–2".into()));
    }

    // Update server
    server_repo::update_server(
        &state.db, server_id, update_name, update_description,
        update_verification, update_content_filter, update_notifications,
        update_vanity, update_system_ch, update_rules_ch,
    )
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        server_id = server.id,
        name = %update_name,
        "server updated"
    );

    // Fetch updated server
    let updated = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::error!(
                server_id = server_id.as_i64(),
                "server disappeared after update"
            );
            ApiError::Internal(anyhow::anyhow!("server disappeared after update"))
        })?;

    Ok(Json(server_row_to_response(updated)))
}

/// DELETE /api/v1/servers/{id} — Delete a server (owner only).
///
/// Requires authentication and ownership of the server.
/// Returns 403 if not the owner.
/// Returns 204 No Content on success.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!(id = %id, "deleting server");

    // Parse path parameter as i64, then convert to Snowflake
    let server_id: Snowflake = id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid server id".into()))
        .and_then(|id| {
            if id > 0 {
                Ok(Snowflake::new(id))
            } else {
                Err(ApiError::BadRequest("server id must be positive".into()))
            }
        })?;

    // Fetch server from database
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| {
            tracing::warn!(server_id = server_id.as_i64(), "server not found");
            ApiError::NotFound("server not found".into())
        })?;

    if server.owner_id != auth.user_id.as_i64() {
        tracing::warn!(server_id = server.id, "delete forbidden: not owner");
        return Err(ApiError::Forbidden);
    }

    server_repo::delete_server(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(server_id = server.id, "server deleted");
    Ok(StatusCode::NO_CONTENT)
}
