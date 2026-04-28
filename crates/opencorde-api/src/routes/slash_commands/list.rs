//! GET /api/v1/servers/{id}/commands handler.

use axum::{Json, extract::{State, Path}};
use opencorde_db::repos::server_repo;
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, routes::permission_check, AppState};
use opencorde_core::permissions::Permissions;
use super::helpers::parse_snowflake;
use super::types::{SlashCommandResponse, row_to_response};
use opencorde_db::repos::slash_command_repo;

/// GET /api/v1/servers/{id}/commands — List all commands for a server.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_commands(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<SlashCommandResponse>>, ApiError> {
    tracing::info!("listing slash commands");

    // Parse server ID
    let server_id_sf = parse_snowflake(&server_id)?;

    // Verify server exists
    let _server = server_repo::get_by_id(&state.db, server_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id_sf, Permissions::VIEW_CHANNEL).await?;

    // Fetch commands
    let rows = slash_command_repo::list_commands(&state.db, server_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list slash commands");
            ApiError::Database(e)
        })?;

    let commands: Vec<SlashCommandResponse> = rows.into_iter().map(row_to_response).collect();

    tracing::info!(count = commands.len(), "slash commands fetched successfully");

    Ok(Json(commands))
}
