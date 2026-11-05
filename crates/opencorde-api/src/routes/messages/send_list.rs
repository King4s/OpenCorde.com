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
};
use opencorde_db::repos::message_repo;
use tracing::instrument;

use opencorde_core::permissions::Permissions;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check};

use super::types::{MessageQuery, MessageResponse, ReplyContextResponse};
use super::validation::{parse_snowflake_id, validate_limit};

/// Convert MessageRow to MessageResponse.
pub fn message_row_to_response(row: message_repo::MessageRow) -> MessageResponse {
    let reply_to = match (
        row.reply_to_id,
        row.reply_author_username,
        row.reply_content_preview,
    ) {
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
        forwarded_from: None,
    }
}

pub use super::send::send_message;

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

    if crate::routes::e2ee::sync::is_v2(&state.db, channel_id_sf).await? {
        crate::routes::e2ee::sync::require_active(&state.db, auth.user_id, channel_id_sf).await?;
    }

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
    let mut rows = message_repo::list_by_channel(&state.db, channel_id_sf, before, after, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list messages");
            ApiError::Database(e)
        })?;

    if crate::routes::e2ee::sync::is_v2(&state.db, channel_id_sf).await? {
        let visible: Vec<i64> = sqlx::query_scalar(
            "SELECT (e.event->>'message_id')::bigint FROM e2ee_events e
             JOIN e2ee_stream_members m USING(channel_id)
             WHERE e.channel_id=$1 AND m.user_id=$2 AND m.active
             AND e.sequence>m.joined_sequence AND e.event->>'kind'='application'
             AND (e.event->>'message_id')::bigint=ANY($3)",
        ).bind(channel_id_sf.as_i64()).bind(auth.user_id.as_i64())
            .bind(rows.iter().map(|row|row.id).collect::<Vec<_>>())
            .fetch_all(&state.db).await?;
        rows.retain(|row|visible.contains(&row.id));
    }

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
            forwarded_from_id: None,
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
            forwarded_from_id: None,
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
