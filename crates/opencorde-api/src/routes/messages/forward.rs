//! # Message Forward Handler
//! HTTP handler for forwarding messages to another channel.
//!
//! ## Endpoints
//! - POST /api/v1/messages/{id}/forward — Forward a message to another channel
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::message_repo (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::permissions::Permissions;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::message_repo;
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check};

use super::types::{ForwardContextResponse, ForwardMessageRequest, MessageResponse};
use super::validation::parse_snowflake_id;

/// POST /api/v1/messages/{id}/forward — Forward a message to another channel.
///
/// Requires authentication. The user must have VIEW_CHANNEL (or READ_MESSAGE_HISTORY)
/// in the source channel and SEND_MESSAGES in the target channel.
///
/// Creates a new message in the target channel with the same content and attachments,
/// preserving source attribution via forwarded_from_id.
///
/// Returns 201 Created with the new message including forwarded_from context.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn forward_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(source_id): Path<String>,
    Json(req): Json<ForwardMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    tracing::info!("forwarding message");

    // Parse source message ID
    let source_id_sf = parse_snowflake_id(&source_id)?;
    tracing::debug!(
        source_msg_id = source_id_sf.as_i64(),
        "parsed source message id"
    );

    // Parse target channel ID
    let target_channel_sf = parse_snowflake_id(&req.target_channel_id)?;
    tracing::debug!(
        target_channel = target_channel_sf.as_i64(),
        "parsed target channel id"
    );

    // Fetch source message
    let source_msg = message_repo::get_by_id(&state.db, source_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch source message");
            ApiError::Database(e)
        })?
        .ok_or_else(|| {
            tracing::warn!(
                source_msg_id = source_id_sf.as_i64(),
                "source message not found"
            );
            ApiError::NotFound("source message not found".to_string())
        })?;

    let source_channel_sf = Snowflake::new(source_msg.channel_id);

    // Check source channel visibility (user must be able to read the source message)
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        source_channel_sf,
        Permissions::VIEW_CHANNEL | Permissions::READ_MESSAGE_HISTORY,
    )
    .await?;

    // Check target channel SEND_MESSAGES permission
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        target_channel_sf,
        Permissions::SEND_MESSAGES,
    )
    .await?;

    // Generate new message ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let new_msg_id = generator.next_id();

    // Create forwarded message in target channel
    let row = message_repo::create_message(
        &state.db,
        new_msg_id,
        target_channel_sf,
        auth.user_id,
        &source_msg.content,
        None, // no reply_to
        source_msg.attachments.clone(),
        None,               // no thread_id
        Some(source_id_sf), // forwarded_from_id
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create forwarded message");
        ApiError::Database(e)
    })?;

    tracing::info!(
        new_msg_id = row.id,
        source_msg_id = source_id_sf.as_i64(),
        target_channel = target_channel_sf.as_i64(),
        "message forwarded successfully"
    );

    // Build forwarded-from context
    let forwarded_context = ForwardContextResponse {
        id: source_id_sf.to_string(),
        author_username: source_msg.author_username.clone(),
        content: source_msg.content.chars().take(100).collect(),
        channel_id: source_channel_sf.to_string(),
    };

    let response = MessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        attachments: row.attachments,
        edited_at: row.edited_at,
        created_at: row.created_at,
        reply_to_id: None,
        reply_to: None,
        forwarded_from: Some(forwarded_context),
    };

    // Broadcast MessageCreate event to WebSocket clients in the target channel
    let event = serde_json::json!({
        "type": "MessageCreate",
        "data": {
            "message": response,
            "mentions": {
                "users": [],
                "roles": [],
                "everyone": false,
                "here": false,
            }
        }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for forwarded MessageCreate event");
    }

    Ok((StatusCode::CREATED, Json(response)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forward_validation() {
        // Test that parse_snowflake_id works (basic check)
        let result = parse_snowflake_id("123456789");
        assert!(result.is_ok());
    }
}
