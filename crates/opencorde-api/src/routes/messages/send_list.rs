//! # Message Send & List Handlers
//! HTTP handlers for sending and listing messages.
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/messages — Send message
//! - GET /api/v1/channels/{channel_id}/messages — List messages (cursor pagination)
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::message_repo (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::message_repo;
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

use super::types::{MessageQuery, MessageResponse, SendMessageRequest};
use super::validation::{parse_snowflake_id, validate_content, validate_limit};

/// Convert MessageRow to MessageResponse.
pub fn message_row_to_response(row: message_repo::MessageRow) -> MessageResponse {
    MessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        attachments: row.attachments,
        edited_at: row.edited_at,
        created_at: row.created_at,
    }
}

/// POST /api/v1/channels/{channel_id}/messages — Send a message to a channel.
///
/// Requires authentication. Generates a new Snowflake ID for the message.
/// Content must be 1-4000 characters.
///
/// Returns 201 Created with the new message.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn send_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<SendMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    tracing::info!("sending message to channel");

    // Parse and validate channel ID
    let channel_id_sf = parse_snowflake_id(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    // Validate content
    validate_content(&req.content)?;
    tracing::debug!(content_len = req.content.len(), "content validated");

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let message_id = generator.next_id();
    tracing::debug!(message_id = message_id.as_i64(), "generated message id");

    // Create message in database
    let row = message_repo::create_message(
        &state.db,
        message_id,
        channel_id_sf,
        auth.user_id,
        &req.content,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create message");
        ApiError::Database(e)
    })?;

    tracing::info!(
        message_id = row.id,
        channel_id = row.channel_id,
        "message created successfully"
    );

    Ok((StatusCode::CREATED, Json(message_row_to_response(row))))
}

/// GET /api/v1/channels/{channel_id}/messages — List messages in a channel.
///
/// Requires authentication. Supports cursor-based pagination with optional
/// `before` and `after` cursors, and adjustable `limit` (1-100, default 50).
///
/// Returns array of messages ordered newest first.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Query(query): Query<MessageQuery>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    tracing::info!("listing channel messages");

    // Parse channel ID
    let channel_id_sf = parse_snowflake_id(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    // Parse cursor IDs if provided
    let before = query
        .before
        .as_deref()
        .map(parse_snowflake_id)
        .transpose()?;
    let after = query.after.as_deref().map(parse_snowflake_id).transpose()?;

    // Validate and set limit
    let limit = validate_limit(query.limit);
    tracing::debug!(before = ?before, after = ?after, limit = limit, "pagination validated");

    // Fetch messages from database
    let rows = message_repo::list_by_channel(&state.db, channel_id_sf, before, after, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list messages");
            ApiError::Database(e)
        })?;

    tracing::info!(count = rows.len(), "messages fetched successfully");

    let messages = rows.into_iter().map(message_row_to_response).collect();
    Ok(Json(messages))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_row_to_response() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Test message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.channel_id, "555666777");
        assert_eq!(response.author_id, "111222333");
        assert_eq!(response.author_username, "testuser");
        assert_eq!(response.content, "Test message");
        assert!(response.edited_at.is_none());
    }
}
