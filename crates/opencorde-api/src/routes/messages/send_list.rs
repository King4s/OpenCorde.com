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
use chrono::Utc;
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use opencorde_db::repos::message_repo;
use tracing::instrument;

use opencorde_core::permissions::Permissions;

use crate::{AppState, automod, error::ApiError, middleware::auth::AuthUser, routes::permission_check};

use super::types::{MessageQuery, MessageResponse, ReplyContextResponse, SendMessageRequest};
use super::validation::{extract_mention_ids, parse_snowflake_id, validate_content, validate_limit};

/// Convert MessageRow to MessageResponse.
pub fn message_row_to_response(row: message_repo::MessageRow) -> MessageResponse {
    let reply_to = match (row.reply_to_id, row.reply_author_username, row.reply_content_preview) {
        (Some(id), Some(author), Some(content)) => Some(ReplyContextResponse {
            id: id.to_string(),
            author_username: author,
            content,
        }),
        _ => None,
    };
    MessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        author_id: row.author_id.to_string(),
        author_username: row.author_username,
        content: row.content,
        attachments: row.attachments,
        edited_at: row.edited_at,
        created_at: row.created_at,
        reply_to_id: row.reply_to_id.map(|id| id.to_string()),
        reply_to,
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

    // Verify the user can send messages in this channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::SEND_MESSAGES,
    )
    .await?;

    // Validate content
    validate_content(&req.content)?;
    tracing::debug!(content_len = req.content.len(), "content validated");

    // Parse reply_to_id if provided
    let reply_to_id = req
        .reply_to_id
        .as_deref()
        .map(parse_snowflake_id)
        .transpose()?;
    tracing::debug!(reply_to_id = ?reply_to_id, "reply_to_id parsed");

    // Fetch server_id and slowmode_delay from channel
    let channel_info: (i64, i64, i32) = sqlx::query_as(
        "SELECT id, server_id, slowmode_delay FROM channels WHERE id = $1",
    )
    .bind(channel_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or(ApiError::NotFound("channel not found".to_string()))?;

    let server_id = Snowflake::new(channel_info.1);
    let slowmode_delay = channel_info.2;

    // Enforce slowmode: check when this user last sent a message in this channel
    if slowmode_delay > 0 {
        let last_sent: Option<chrono::DateTime<Utc>> = sqlx::query_scalar(
            "SELECT created_at FROM messages \
             WHERE channel_id = $1 AND author_id = $2 \
             ORDER BY created_at DESC LIMIT 1",
        )
        .bind(channel_id_sf.as_i64())
        .bind(auth.user_id.as_i64())
        .fetch_optional(&state.db)
        .await
        .map_err(ApiError::Database)?;

        if let Some(last) = last_sent {
            let elapsed = Utc::now()
                .signed_duration_since(last)
                .num_seconds();
            let remaining = slowmode_delay as i64 - elapsed;
            if remaining > 0 {
                tracing::info!(
                    user_id = auth.user_id.as_i64(),
                    channel_id = channel_id_sf.as_i64(),
                    retry_after = remaining,
                    "slowmode: message rejected"
                );
                return Err(ApiError::RateLimited {
                    retry_after: remaining as u64,
                });
            }
        }
    }

    // Check AutoMod rules
    match automod::check_message(&state.db, server_id, &req.content).await {
        automod::AutomodResult::Block { rule_name, .. } => {
            tracing::info!(rule = %rule_name, "message blocked by automod");
            return Err(ApiError::BadRequest(
                format!("message blocked by automod rule: {}", rule_name),
            ));
        }
        automod::AutomodResult::Allow => {}
    }

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let message_id = generator.next_id();
    tracing::debug!(message_id = message_id.as_i64(), "generated message id");

    // Create message in database
    let attachments = req.attachments.unwrap_or_else(|| serde_json::json!([]));
    let row = message_repo::create_message(
        &state.db,
        message_id,
        channel_id_sf,
        auth.user_id,
        &req.content,
        reply_to_id,
        attachments,
        None,
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

    let response = message_row_to_response(row);

    // Broadcast MessageCreate event to all connected WebSocket clients.
    // Clients filter on their accessible channel IDs; errors here are non-fatal
    // (no subscribers = SendError, lagged subscriber = RecvError on their side).
    let event = serde_json::json!({
        "type": "MessageCreate",
        "data": { "message": response }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MessageCreate event");
    }

    // Fire push notifications for any users @mentioned in this message.
    // Mentions use the format <@USER_ID> (Snowflake ID). We parse them here
    // and dispatch non-blocking so the HTTP response is not delayed.
    let mention_ids = extract_mention_ids(&req.content);
    if !mention_ids.is_empty() {
        let db = state.db.clone();
        let config = state.config.clone();
        let sender_username = response.author_username.clone();
        let content_preview: String = req.content.chars().take(80).collect();
        tokio::spawn(async move {
            for uid in mention_ids {
                crate::push_sender::send_push(
                    &db,
                    &config,
                    uid,
                    &format!("Mention from {}", sender_username),
                    &content_preview,
                )
                .await;
            }
        });
    }

    // Index message in full-text search (non-blocking, non-fatal)
    if let Some(ref engine) = state.search {
        let engine = engine.clone();
        let msg_id = response.id.parse::<u64>().unwrap_or(0);
        let ch_id  = response.channel_id.parse::<u64>().unwrap_or(0);
        let srv_id = server_id.as_i64() as u64;
        let auth_id = auth.user_id.as_i64() as u64;
        let text   = req.content.clone();
        let ts     = chrono::Utc::now().timestamp() as u64;
        tokio::spawn(async move {
            if let Ok(mut indexer) = engine.make_indexer(50_000_000) {
                let _ = indexer.index_message(msg_id, ch_id, srv_id, auth_id, &text, ts)
                    .and_then(|_| indexer.commit());
            }
        });
    }

    Ok((StatusCode::CREATED, Json(response)))
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

    // Verify the user can view this channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

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
            reply_to_id: None,
            reply_author_username: None,
            reply_content_preview: None,
            thread_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.channel_id, "555666777");
        assert_eq!(response.author_id, "111222333");
        assert_eq!(response.author_username, "testuser");
        assert_eq!(response.content, "Test message");
        assert!(response.edited_at.is_none());
        assert!(response.reply_to_id.is_none());
        assert!(response.reply_to.is_none());
    }

    #[test]
    fn test_message_row_to_response_with_reply() {
        use chrono::Utc;
        use serde_json::json;

        let now = Utc::now();
        let row = message_repo::MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Reply message".to_string(),
            attachments: json!([]),
            edited_at: None,
            created_at: now,
            author_username: "testuser".to_string(),
            reply_to_id: Some(123456),
            reply_author_username: Some("originaluser".to_string()),
            reply_content_preview: Some("Original content".to_string()),
            thread_id: None,
        };

        let response = message_row_to_response(row);
        assert_eq!(response.id, "777888999");
        assert_eq!(response.reply_to_id, Some("123456".to_string()));
        assert!(response.reply_to.is_some());
        let ctx = response.reply_to.unwrap();
        assert_eq!(ctx.author_username, "originaluser");
        assert_eq!(ctx.content, "Original content");
    }
}
