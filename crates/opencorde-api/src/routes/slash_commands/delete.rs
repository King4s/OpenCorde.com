//! DELETE /api/v1/commands/{command_id} handler.

use axum::{extract::{State, Path}, http::StatusCode};
use opencorde_core::snowflake::Snowflake;
use opencorde_db::repos::{server_repo, slash_command_repo};
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use super::helpers::parse_snowflake;

/// DELETE /api/v1/commands/{command_id} — Delete a slash command.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn delete_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(command_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("deleting slash command");

    // Parse command ID
    let command_id_sf = parse_snowflake(&command_id)?;

    // Fetch command to verify ownership
    let mut rows: Vec<slash_command_repo::SlashCommandRow> = sqlx::query_as(
        "SELECT * FROM slash_commands WHERE id = $1",
    )
    .bind(command_id_sf.as_i64())
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to fetch slash command");
        ApiError::Database(e)
    })?;

    let command = rows.pop().ok_or(ApiError::NotFound("command not found".to_string()))?;

    // Verify user is server owner
    let server = server_repo::get_by_id(&state.db, Snowflake::new(command.server_id))
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch server");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("server not found".to_string()))?;

    if server.owner_id != auth.user_id.as_i64() {
        tracing::warn!("user is not server owner");
        return Err(ApiError::Forbidden);
    }

    // Delete command
    slash_command_repo::delete_command(&state.db, command_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to delete slash command");
            ApiError::Database(e)
        })?;

    tracing::info!(command_id = command_id_sf.as_i64(), "slash command deleted successfully");

    Ok(StatusCode::NO_CONTENT)
}
