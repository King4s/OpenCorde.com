//! POST /api/v1/servers/{id}/commands handler.

use axum::{Json, extract::{State, Path}, http::StatusCode};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{server_repo, slash_command_repo};

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use super::helpers::parse_snowflake;
use super::types::{CreateCommandRequest, SlashCommandResponse, row_to_response};

/// POST /api/v1/servers/{id}/commands — Register a slash command.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn create_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateCommandRequest>,
) -> Result<(StatusCode, Json<SlashCommandResponse>), ApiError> {
    tracing::info!("creating slash command");

    // Parse server ID
    let server_id_sf = parse_snowflake(&server_id)?;

    // Verify user is server owner
    let server = server_repo::get_by_id(&state.db, server_id_sf)
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

    // Validate command name: lowercase letters/dashes/numbers only, 1-32 chars
    let name = req.name.trim().to_lowercase();
    if name.is_empty() || name.len() > 32 {
        return Err(ApiError::BadRequest(
            "command name must be 1-32 characters".to_string(),
        ));
    }
    if !name.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-') {
        return Err(ApiError::BadRequest(
            "command name must contain only lowercase letters, numbers, and dashes".to_string(),
        ));
    }

    // Validate handler_url is http/https
    let handler_url = req.handler_url.trim();
    if !handler_url.starts_with("http://") && !handler_url.starts_with("https://") {
        return Err(ApiError::BadRequest(
            "handler URL must use http:// or https://".to_string(),
        ));
    }

    // Validate description
    let description = req
        .description
        .map(|d| d.trim().to_string())
        .unwrap_or_default();
    if description.len() > 100 {
        return Err(ApiError::BadRequest(
            "description must be 100 characters or less".to_string(),
        ));
    }

    // Generate command ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let command_id = generator.next_id();

    // Create command
    let row = slash_command_repo::create_command(
        &state.db,
        command_id,
        server_id_sf,
        &name,
        &description,
        handler_url,
        auth.user_id,
    )
    .await
    .map_err(|e| {
        // Check for unique constraint violation (command name already exists)
        if e.to_string().contains("duplicate key") {
            tracing::warn!("command name already exists for this server");
            return ApiError::BadRequest(format!("command '{}' already exists", name));
        }
        tracing::error!(error = %e, "failed to create slash command");
        ApiError::Database(e)
    })?;

    tracing::info!(command_id = row.id, "slash command created successfully");

    Ok((StatusCode::CREATED, Json(row_to_response(row))))
}
