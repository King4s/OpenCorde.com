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
use opencorde_core::snowflake::{Snowflake, SnowflakeGenerator};
use reqwest::Client;
use serde_json::json;
use std::time::Duration;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use opencorde_db::repos::{dm_federated_repo, dm_repo, user_repo};

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
/// For same-server DMs: `{ "recipient_id": "123456789" }`
/// For cross-server DMs: `{ "recipient_address": "alice@chat.example.com" }`
///
/// Cross-server: verifies the remote user exists via their server's federation
/// endpoint, then creates a local federated DM channel.
///
/// Requires authentication.
#[tracing::instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn open_dm(
    State(state): State<AppState>,
    auth: AuthUser,
    Json(req): Json<super::types::OpenDmRequest>,
) -> Result<(StatusCode, Json<DmChannelResponse>), ApiError> {
    let mut generator = SnowflakeGenerator::new(1, 1);
    let dm_id = generator.next_id();

    // Cross-server DM: recipient_address = "username@hostname"
    if let Some(ref address) = req.recipient_address {
        return open_federated_dm(&state, auth.user_id, address, dm_id).await;
    }

    // Local DM: recipient_id = snowflake string
    let raw_id = req.recipient_id
        .ok_or_else(|| ApiError::BadRequest("recipient_id or recipient_address required".into()))?;

    tracing::info!(recipient_id = %raw_id, "opening local dm");

    let recipient_id = raw_id
        .parse::<i64>()
        .map_err(|_| ApiError::BadRequest("invalid recipient_id format".into()))
        .map(Snowflake::new)?;

    let dm_id_result = dm_repo::get_or_create_dm(&state.db, dm_id, auth.user_id, recipient_id)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get or create dm");
            ApiError::InternalServerError("failed to get or create dm".into())
        })?;

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

    Ok((StatusCode::CREATED, Json(DmChannelResponse {
        id: channel.id.to_string(),
        other_user_id: channel.other_user_id.to_string(),
        other_username: channel.other_username,
        last_read_id: channel.last_read_id.to_string(),
    })))
}

/// Open a federated DM channel with a user on another server.
///
/// 1. Parses "username@hostname" from the address
/// 2. Verifies the remote user exists via GET /api/v1/federation/users/{username}
/// 3. Creates a local dm_channel with remote_peer_address set
async fn open_federated_dm(
    state: &AppState,
    local_user: Snowflake,
    address: &str,
    dm_id: opencorde_core::snowflake::Snowflake,
) -> Result<(StatusCode, Json<DmChannelResponse>), ApiError> {
    tracing::info!(recipient_address = %address, "opening federated dm");

    let parts: Vec<&str> = address.splitn(2, '@').collect();
    if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
        return Err(ApiError::BadRequest(
            "recipient_address must be in format username@hostname".into(),
        ));
    }
    let (remote_username, remote_server) = (parts[0], parts[1]);

    // Prevent opening a DM with yourself across servers (edge case)
    if remote_server == state.config.mesh_hostname {
        // Route to local DM instead
        let local_recipient = user_repo::get_by_username(&state.db, remote_username)
            .await
            .map_err(ApiError::Database)?
            .ok_or_else(|| ApiError::NotFound(format!("user '{}' not found", remote_username)))?;
        let recipient_sf = Snowflake::new(local_recipient.id);
        let dm_id_result = dm_repo::get_or_create_dm(&state.db, dm_id, local_user, recipient_sf)
            .await
            .map_err(ApiError::Database)?;
        let channels = dm_repo::list_dms_for_user(&state.db, local_user)
            .await
            .map_err(ApiError::Database)?;
        let ch = channels.into_iter().find(|c| c.id == dm_id_result)
            .ok_or_else(|| ApiError::NotFound("dm channel not found".into()))?;
        return Ok((StatusCode::CREATED, Json(DmChannelResponse {
            id: ch.id.to_string(),
            other_user_id: ch.other_user_id.to_string(),
            other_username: ch.other_username,
            last_read_id: ch.last_read_id.to_string(),
        })));
    }

    // Verify remote user exists
    let lookup_url = format!(
        "https://{}/api/v1/federation/users/{}",
        remote_server, remote_username
    );
    let client = Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .map_err(|e| ApiError::InternalServerError(format!("http client: {}", e)))?;

    let resp = client.get(&lookup_url).send().await.map_err(|e| {
        tracing::warn!(error = %e, url = %lookup_url, "failed to reach remote server for user lookup");
        ApiError::BadRequest(format!("could not reach {}: {}", remote_server, e))
    })?;

    if !resp.status().is_success() {
        return Err(ApiError::NotFound(format!(
            "user '{}' not found on {}",
            remote_username, remote_server
        )));
    }

    // Create federated DM channel
    let dm_id_result = dm_federated_repo::get_or_create_federated_dm(
        &state.db,
        dm_id,
        local_user,
        address,
        remote_server,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create federated dm channel");
        ApiError::InternalServerError("failed to create dm channel".into())
    })?;

    tracing::info!(dm_id = dm_id_result, recipient_address = %address, "federated dm channel opened");

    Ok((StatusCode::CREATED, Json(DmChannelResponse {
        id: dm_id_result.to_string(),
        other_user_id: "0".to_string(),
        other_username: address.to_string(),
        last_read_id: "0".to_string(),
    })))
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
        .map(Snowflake::new)?;

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

    // Send message locally
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
        author_username: message.author_username.clone(),
        content: message.content.clone(),
        attachments: message.attachments.clone(),
        edited_at: message.edited_at,
        created_at: message.created_at,
    };

    // Broadcast DmMessageCreate event to local WebSocket subscribers
    let ws_event = json!({
        "type": "DmMessageCreate",
        "data": { "message": &response }
    });
    if state.event_tx.send(ws_event).is_err() {
        tracing::debug!("no WebSocket subscribers for DmMessageCreate event");
    }

    // Forward to remote server if this is a federated DM channel
    if let Ok(Some(remote_server)) = dm_federated_repo::get_remote_server(&state.db, dm_id_sf).await {
        let remote_peer = dm_federated_repo::get_remote_peer_address(&state.db, dm_id_sf)
            .await
            .unwrap_or(None)
            .unwrap_or_default();

        let parts: Vec<&str> = remote_peer.splitn(2, '@').collect();
        let recipient_username = parts.first().copied().unwrap_or(&remote_peer).to_string();

        let me = user_repo::get_by_id(&state.db, auth.user_id)
            .await
            .ok()
            .flatten();
        let sender_username = me.map(|u| u.username).unwrap_or_default();
        let sender_address = format!("{}@{}", sender_username, state.config.mesh_hostname);

        let payload = serde_json::json!({
            "recipient_username": recipient_username,
            "sender_address": sender_address,
            "content": message.content,
            "message_id": response.id,
        });

        crate::federation::forward_event_to(
            &state.db,
            &state.identity,
            &state.config.mesh_hostname,
            &remote_server,
            "FederatedDMCreate",
            payload,
        )
        .await;
    }

    Ok((StatusCode::CREATED, Json(response)))
}
