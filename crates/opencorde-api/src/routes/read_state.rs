//! # Route: Read State - Unread tracking and mark-as-read
//!
//! ## Endpoints
//! - POST /api/v1/channels/{channel_id}/ack — Mark channel as read
//! - GET /api/v1/users/@me/read-states — Get all read states for authenticated user
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::read_state_repo (database operations)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use opencorde_core::{Snowflake, permissions::Permissions};
use opencorde_db::repos::read_state_repo;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};

use super::helpers;
use super::permission_check;

/// Request body for marking a channel as read.
#[derive(Debug, Deserialize)]
pub struct AckRequest {
    /// ID of the last message read (as string to preserve Snowflake format).
    pub message_id: String,
}

/// Response for a single channel's read state.
#[derive(Debug, Serialize)]
pub struct ReadStateResponse {
    pub channel_id: String,
    pub last_read_id: String,
    pub mention_count: i32,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/channels/{channel_id}/ack", post(mark_channel_read))
        .route("/api/v1/users/@me/read-states", get(get_user_read_states))
}

/// POST /api/v1/channels/{channel_id}/ack — Mark a channel as read.
///
/// Accepts the message ID of the last message the user has read.
/// Resets mention_count to 0. Also broadcasts ChannelAck event to other sessions.
///
/// Returns 204 No Content on success.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn mark_channel_read(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<AckRequest>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(channel_id = %channel_id, "marking channel as read");

    // Parse channel and message IDs
    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;
    let message_id = req
        .message_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid message_id format".into()))?;

    tracing::debug!(
        channel_id = channel_id_sf.as_i64(),
        message_id,
        "parsed ids"
    );

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Mark as read in database
    read_state_repo::mark_read(&state.db, auth.user_id, channel_id_sf, message_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        channel_id = channel_id_sf.as_i64(),
        message_id,
        "channel marked as read"
    );

    // Broadcast ChannelAck event to other WebSocket sessions of the same user
    let event = serde_json::json!({
        "type": "ChannelAck",
        "data": {
            "user_id": auth.user_id.to_string(),
            "channel_id": channel_id_sf.to_string(),
            "last_read_id": message_id.to_string(),
        }
    });

    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for ChannelAck event");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/users/@me/read-states — Get all read states for the authenticated user.
///
/// Returns an array of read state entries for all channels the user has interacted with.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_user_read_states(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ReadStateResponse>>, ApiError> {
    tracing::info!("fetching read states for user");

    let rows = read_state_repo::get_for_user(&state.db, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = rows.len(), "read states fetched");

    let mut responses = Vec::with_capacity(rows.len());
    for row in rows {
        match permission_check::require_channel_perm(
            &state.db,
            auth.user_id,
            Snowflake::new(row.channel_id),
            Permissions::VIEW_CHANNEL,
        )
        .await
        {
            Ok(()) => responses.push(ReadStateResponse {
                channel_id: row.channel_id.to_string(),
                last_read_id: row.last_read_id.to_string(),
                mention_count: row.mention_count,
            }),
            Err(ApiError::Forbidden | ApiError::NotFound(_)) => {
                tracing::debug!(channel_id = row.channel_id, "filtered read state");
            }
            Err(err) => return Err(err),
        }
    }

    Ok(Json(responses))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ack_request_deserialization() {
        let json = r#"{"message_id":"123456789"}"#;
        let req: AckRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.message_id, "123456789");
    }

    #[test]
    fn test_read_state_response_serialization() {
        let response = ReadStateResponse {
            channel_id: "999".to_string(),
            last_read_id: "777".to_string(),
            mention_count: 2,
        };
        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("\"channel_id\":\"999\""));
        assert!(json.contains("\"mention_count\":2"));
    }
}
