//! # DM Route Handlers
//! HTTP request handlers for direct message endpoints.
//!
//! ## Handlers
//! - `list_dms` — GET /api/v1/users/@me/channels
//! - `open_dm` — POST /api/v1/users/@me/channels
//! - `list_dm_messages` — GET /api/v1/channels/@dms/{dm_id}/messages
//! - `send_dm_message` — POST /api/v1/channels/@dms/{dm_id}/messages
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::dm_repo (database operations)
//! - opencorde_core::Snowflake (ID handling)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)
//! - crate::error::ApiError (error handling)

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use opencorde_core::snowflake::SnowflakeGenerator;
use serde_json::json;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use opencorde_db::repos::dm_repo;

use super::types::{DmChannelResponse, DmMessageResponse, MessageListQuery, SendDmRequest};

/// GET /api/v1/users/@me/channels — List DM channels for current user.
///
/// Returns all DM channels the user is a member of, with the other
/// participant's info and last read message ID.
///
/// Requires authentication.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_dms(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<DmChannelResponse>>, ApiError> {
    tracing::info!("listing user dm channels");

    let channels = dm_repo::list_dms_for_user(&state.db, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list dms");
            ApiError::InternalServerError("failed to list dms".into())
        })?;

    let responses = channels
        .into_iter()
        .map(|ch| DmChannelResponse {
            id: ch.id.to_string(),
            other_user_id: ch.other_user_id.to_string(),
            other_username: ch.other_username,
            last_read_id: ch.last_read_id.to_string(),
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/v1/users/@me/channels — Open a DM with a user.
///
/// Gets or creates a DM channel between the current user and the recipient.
/// Returns the DM channel (existing or newly created).
///
/// Requires authentication.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn open_dm(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<super::types::OpenDmRequest>,
) -> Result<(StatusCode, Json<DmChannelResponse>), ApiError> {
    tracing::info!(recipient_id = %req.recipient_id, "opening dm");

    // Parse recipient ID
    let recipient_id = req
        .recipient_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid recipient_id format".into()))
        .map(opencorde_core::snowflake::Snowflake::new)?;

    // Generate a new DM channel ID (in case we need to create one)
    let mut generator = SnowflakeGenerator::new(1, 1);
    let dm_id = generator.next_id();

    // Get or create DM and return
    let dm_id_result = dm_repo::get_or_create_dm(&state.db, dm_id, auth.user_id, recipient_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get or create dm");
            ApiError::InternalServerError("failed to get or create dm".into())
        })?;

    // Fetch the DM channel info (with the other user's details)
    let channels = dm_repo::list_dms_for_user(&state.db, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to fetch dm channel");
            ApiError::InternalServerError("failed to fetch dm channel".into())
        })?;

    let channel = channels
        .into_iter()
        .find(|ch| ch.id == dm_id_result)
        .ok_or_else(|| {
            tracing::warn!(dm_id = dm_id_result, "dm channel not found after creation");
            ApiError::NotFound("dm channel not found".into())
        })?;

    let response = DmChannelResponse {
        id: channel.id.to_string(),
        other_user_id: channel.other_user_id.to_string(),
        other_username: channel.other_username,
        last_read_id: channel.last_read_id.to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /api/v1/channels/@dms/{dm_id}/messages — List DM messages.
///
/// Returns messages from a DM channel with cursor-based pagination.
/// Default limit is 50, maximum is 100.
/// Use `before` query param to fetch messages before a cursor ID.
///
/// Requires authentication and DM channel membership.
#[tracing::instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_dm_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(dm_id): Path<String>,
    Query(params): Query<MessageListQuery>,
) -> Result<Json<Vec<DmMessageResponse>>, ApiError> {
    tracing::info!(dm_id = %dm_id, "listing dm messages");

    // Parse DM ID
    let dm_id_sf = dm_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid dm_id format".into()))
        .map(opencorde_core::snowflake::Snowflake::new)?;

    // Check membership
    let is_member = dm_repo::is_dm_member(&state.db, dm_id_sf, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to check dm membership");
            ApiError::InternalServerError("failed to check membership".into())
        })?;

    if !is_member {
        tracing::warn!(dm_id = dm_id_sf.as_i64(), "user not a member of dm");
        return Err(ApiError::Forbidden);
    }

    // Parse optional before cursor
    let before = params
        .before
        .map(|b| {
            b.parse::<i64>()
                .map_err(|_| ApiError::BadRequest("invalid before format".into()))
                .map(opencorde_core::snowflake::Snowflake::new)
        })
        .transpose()?;

    let limit = params.limit.unwrap_or(50);

    let messages = dm_repo::list_dm_messages(&state.db, dm_id_sf, before, limit)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list dm messages");
            ApiError::InternalServerError("failed to list messages".into())
        })?;

    let responses = messages
        .into_iter()
        .map(|msg| DmMessageResponse {
            id: msg.id.to_string(),
            dm_id: msg.dm_id.to_string(),
            author_id: msg.author_id.to_string(),
            author_username: msg.author_username,
            content: msg.content,
            attachments: msg.attachments,
            edited_at: msg.edited_at,
            created_at: msg.created_at,
        })
        .collect();

    Ok(Json(responses))
}

/// POST /api/v1/channels/@dms/{dm_id}/messages — Send a DM message.
///
/// Creates a new message in a DM channel.
/// Broadcasts a DmMessageCreate event to both participants via WebSocket.
///
/// Requires authentication and DM channel membership.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn send_dm_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(dm_id): Path<String>,
    Json(req): Json<SendDmRequest>,
) -> Result<(StatusCode, Json<DmMessageResponse>), ApiError> {
    tracing::info!(dm_id = %dm_id, "sending dm message");

    // Parse DM ID
    let dm_id_sf = dm_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid dm_id format".into()))
        .map(opencorde_core::snowflake::Snowflake::new)?;

    // Check membership
    let is_member = dm_repo::is_dm_member(&state.db, dm_id_sf, auth.user_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to check dm membership");
            ApiError::InternalServerError("failed to check membership".into())
        })?;

    if !is_member {
        tracing::warn!(dm_id = dm_id_sf.as_i64(), "user not a member of dm");
        return Err(ApiError::Forbidden);
    }

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(1, 1);
    let message_id = generator.next_id();

    // Send message
    let message = dm_repo::send_dm_message(&state.db, message_id, dm_id_sf, auth.user_id, &req.content)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to send dm message");
            ApiError::InternalServerError("failed to send message".into())
        })?;

    let response = DmMessageResponse {
        id: message.id.to_string(),
        dm_id: message.dm_id.to_string(),
        author_id: message.author_id.to_string(),
        author_username: message.author_username,
        content: message.content,
        attachments: message.attachments,
        edited_at: message.edited_at,
        created_at: message.created_at,
    };

    // Broadcast DmMessageCreate event to WebSocket subscribers
    let event = json!({
        "type": "DmMessageCreate",
        "data": { "message": &response }
    });

    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for DmMessageCreate event");
    }

    Ok((StatusCode::CREATED, Json(response)))
}
