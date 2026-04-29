//! # Message Edit, Delete & Typing Handlers
//! HTTP handlers for editing, deleting messages and typing indicator.
//!
//! ## Endpoints
//! - PATCH /api/v1/messages/{id} — Edit message (author only)
//! - DELETE /api/v1/messages/{id} — Delete message (author only)
//! - POST /api/v1/channels/{channel_id}/typing — Typing indicator
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::message_repo (database operations)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::message_repo;
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check};

use super::send_list::message_row_to_response;
use super::types::{EditMessageRequest, MessageResponse};
use super::validation::{parse_snowflake_id, validate_content};

/// PATCH /api/v1/messages/{id} — Edit a message.
///
/// Requires authentication. Only the message author can edit their own messages.
/// Updates content and sets edited_at timestamp.
///
/// Returns 200 OK with the updated message.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn edit_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<EditMessageRequest>,
) -> Result<Json<MessageResponse>, ApiError> {
    tracing::info!("editing message");

    // Parse message ID
    let message_id = parse_snowflake_id(&id)?;
    tracing::debug!(message_id = message_id.as_i64(), "parsed message id");

    // Validate new content
    validate_content(&req.content)?;
    tracing::debug!(content_len = req.content.len(), "new content validated");

    // Fetch message to check ownership
    let message = message_repo::get_by_id(&state.db, message_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch message");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(message_id = message_id.as_i64(), "message not found");
            ApiError::NotFound("message not found".to_string())
        })?;

    // Check ownership
    if message.author_id != auth.user_id.as_i64() {
        tracing::warn!(
            message_author = message.author_id,
            user_id = auth.user_id.as_i64(),
            "user is not message author"
        );
        return Err(ApiError::Forbidden);
    }

    // Update message content
    message_repo::update_content(&state.db, message_id, &req.content)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to update message");
            ApiError::Database(e)
        })?;

    tracing::info!(
        message_id = message_id.as_i64(),
        "message updated successfully"
    );

    // Fetch updated message
    let updated = message_repo::get_by_id(&state.db, message_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch updated message");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::error!(
                message_id = message_id.as_i64(),
                "updated message disappeared"
            );
            ApiError::Internal(anyhow::anyhow!("message vanished after update"))
        })?;

    let response = message_row_to_response(updated);

    // Broadcast MessageUpdate to WebSocket clients
    let event = serde_json::json!({
        "type": "MessageUpdate",
        "data": { "message": response }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MessageUpdate event");
    }

    Ok(Json(response))
}

/// DELETE /api/v1/messages/{id} — Delete a message.
///
/// Requires authentication. Authors can delete their own messages; moderators
/// with MANAGE_MESSAGES can delete messages from other users.
///
/// Returns 204 No Content on success.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn delete_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("deleting message");

    // Parse message ID
    let message_id = parse_snowflake_id(&id)?;
    tracing::debug!(message_id = message_id.as_i64(), "parsed message id");

    // Fetch message to check ownership
    let message = message_repo::get_by_id(&state.db, message_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch message");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(message_id = message_id.as_i64(), "message not found");
            ApiError::NotFound("message not found".to_string())
        })?;

    // Check ownership or moderation permission.
    if message.author_id != auth.user_id.as_i64() {
        permission_check::require_channel_perm(
            &state.db,
            auth.user_id,
            opencorde_core::Snowflake::new(message.channel_id),
            Permissions::VIEW_CHANNEL | Permissions::MANAGE_MESSAGES,
        )
        .await?;
    }

    // Delete message
    message_repo::delete_message(&state.db, message_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to delete message");
            ApiError::Database(e)
        })?;

    tracing::info!(
        message_id = message_id.as_i64(),
        "message deleted successfully"
    );

    // Broadcast MessageDelete to WebSocket clients
    let event = serde_json::json!({
        "type": "MessageDelete",
        "data": {
            "channel_id": message.channel_id.to_string(),
            "message_id": message_id.to_string()
        }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MessageDelete event");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/channels/{channel_id}/typing — Send typing indicator.
///
/// Requires authentication. Broadcasts TypingStart event via WebSocket to
/// all clients watching this channel.
///
/// Returns 204 No Content.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn typing_indicator(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(channel_id = %channel_id, "typing indicator received");

    // Validate channel ID format
    let channel_id_sf = parse_snowflake_id(&channel_id)?;

    tracing::debug!(
        user_id = auth.user_id.as_i64(),
        channel_id = channel_id_sf.as_i64(),
        "broadcasting TypingStart event"
    );

    // Broadcast TypingStart event to WebSocket clients in this channel
    let event = serde_json::json!({
        "type": "TypingStart",
        "data": {
            "channel_id": channel_id_sf.to_string(),
            "user_id": auth.user_id.to_string()
        }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for TypingStart event");
    }

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_typing_indicator_validation() {
        // Test that parse_snowflake_id is working (basic check)
        let result = parse_snowflake_id("123456789");
        assert!(result.is_ok());
    }
}
