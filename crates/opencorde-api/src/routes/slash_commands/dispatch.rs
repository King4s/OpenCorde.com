//! POST /api/v1/channels/{channel_id}/interact handler.

use axum::{Json, extract::{State, Path}};
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::{channel_repo, message_repo, slash_command_repo};
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, routes::permission_check, AppState};
use super::helpers::parse_snowflake;
use super::types::{InteractRequest, CommandHandlerPayload, CommandHandlerResponse};

/// POST /api/v1/channels/{channel_id}/interact — Dispatch slash command.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn dispatch_command(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<InteractRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    tracing::info!("dispatching slash command");

    // Parse channel ID
    let channel_id_sf = parse_snowflake(&channel_id)?;

    // Fetch channel to get server_id
    let channel = channel_repo::get_by_id(&state.db, channel_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch channel");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("channel not found".to_string()))?;

    let server_id_sf = Snowflake::new(channel.server_id);

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::USE_APPLICATION_COMMANDS,
    )
    .await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::SEND_MESSAGES,
    )
    .await?;

    // Parse command name (strip leading /)
    let command_name = req.command.trim_start_matches('/').to_lowercase();

    // Look up command by name in server
    let command = slash_command_repo::get_by_name(&state.db, server_id_sf, &command_name)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch slash command");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(command = %command_name, "command not found");
            ApiError::NotFound(format!("command not found: /{}", command_name))
        })?;

    // Prepare handler payload
    let args = req.args.unwrap_or_default();
    let payload = CommandHandlerPayload {
        command: command_name.clone(),
        args,
        user_id: auth.user_id.to_string(),
        username: auth.username.clone(),
        channel_id: channel_id_sf.to_string(),
        server_id: server_id_sf.to_string(),
    };

    // POST to handler_url
    let client = reqwest::Client::new();
    let response = client
        .post(&command.handler_url)
        .json(&payload)
        .send()
        .await
        .map_err(|e| {
            tracing::error!(
                error = %e,
                handler_url = %command.handler_url,
                "failed to call command handler"
            );
            ApiError::ServiceUnavailable("command handler unavailable".to_string())
        })?;

    // Check status code
    if !response.status().is_success() {
        tracing::error!(
            status = response.status().as_u16(),
            "command handler returned error status"
        );
        return Err(ApiError::ServiceUnavailable(
            "command handler unavailable".to_string(),
        ));
    }

    // Parse response
    let handler_response: CommandHandlerResponse = response.json().await.map_err(|e| {
        tracing::error!(error = %e, "failed to parse command handler response");
        ApiError::BadRequest("invalid command handler response".to_string())
    })?;

    // Create message in channel from invoking user
    let mut msg_generator = SnowflakeGenerator::new(3, 0);
    let msg_id = msg_generator.next_id();

    let message = message_repo::create_message(
        &state.db,
        msg_id,
        channel_id_sf,
        auth.user_id,
        &handler_response.content,
        None,
        serde_json::json!([]),
        None,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create message from command response");
        ApiError::Database(e)
    })?;

    tracing::info!(
        command = %command_name,
        message_id = message.id,
        "command executed successfully"
    );

    Ok(Json(serde_json::json!({
        "message": {
            "id": message.id.to_string(),
            "content": message.content,
            "author_id": message.author_id.to_string(),
            "created_at": message.created_at,
        }
    })))
}
