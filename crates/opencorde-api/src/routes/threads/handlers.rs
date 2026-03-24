//! # Thread Route Handlers
//! HTTP handlers for thread endpoints.
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{message_repo, thread_repo};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::helpers::parse_snowflake;
use crate::routes::messages::{message_row_to_response, MessageResponse};
use super::types::{CreateThreadRequest, SendThreadMessageRequest, ThreadResponse};

/// Convert ThreadRow to ThreadResponse.
fn thread_row_to_response(row: thread_repo::ThreadRow) -> ThreadResponse {
    ThreadResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        parent_msg_id: row.parent_msg_id.map(|id| id.to_string()),
        name: row.name,
        created_by: row.created_by.to_string(),
        created_at: row.created_at,
        last_msg_at: row.last_msg_at,
        msg_count: row.msg_count,
    }
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new()
        .route(
            "/api/v1/channels/{channel_id}/messages/{message_id}/thread",
            axum::routing::post(create_thread),
        )
        .route(
            "/api/v1/channels/{channel_id}/threads",
            axum::routing::get(list_threads),
        )
        .route(
            "/api/v1/threads/{thread_id}",
            axum::routing::get(get_thread),
        )
        .route(
            "/api/v1/threads/{thread_id}/messages",
            axum::routing::get(list_thread_messages),
        )
        .route(
            "/api/v1/threads/{thread_id}/messages",
            axum::routing::post(send_thread_message),
        )
}

/// POST /channels/{channel_id}/messages/{message_id}/thread — Create thread from message.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn create_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, message_id)): Path<(String, String)>,
    Json(req): Json<CreateThreadRequest>,
) -> Result<(StatusCode, Json<ThreadResponse>), ApiError> {
    tracing::info!("creating thread from message");

    let channel_id_sf = parse_snowflake(&channel_id)?;
    let message_id_sf = parse_snowflake(&message_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), message_id = message_id_sf.as_i64(), "parsed IDs");

    let name = req.name.unwrap_or_else(|| "Thread".to_string());

    let mut generator = SnowflakeGenerator::new(1, 1);
    let thread_id = generator.next_id();
    tracing::debug!(thread_id = thread_id.as_i64(), "generated thread id");

    let row = thread_repo::create_thread(
        &state.db,
        thread_id,
        channel_id_sf,
        Some(message_id_sf),
        &name,
        auth.user_id,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create thread");
        ApiError::Database(e)
    })?;

    tracing::info!(thread_id = row.id, "thread created successfully");

    let response = thread_row_to_response(row);
    Ok((StatusCode::CREATED, Json(response)))
}

/// GET /channels/{channel_id}/threads — List threads in a channel.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_threads(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<ThreadResponse>>, ApiError> {
    tracing::info!("listing channel threads");

    let channel_id_sf = parse_snowflake(&channel_id)?;
    tracing::debug!(channel_id = channel_id_sf.as_i64(), "parsed channel id");

    let rows = thread_repo::list_by_channel(&state.db, channel_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list threads");
            ApiError::Database(e)
        })?;

    tracing::info!(count = rows.len(), "threads fetched successfully");

    let threads = rows.into_iter().map(thread_row_to_response).collect();
    Ok(Json(threads))
}

/// GET /threads/{thread_id} — Get a single thread.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_thread(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(thread_id): Path<String>,
) -> Result<Json<ThreadResponse>, ApiError> {
    tracing::info!("getting thread");

    let thread_id_sf = parse_snowflake(&thread_id)?;
    tracing::debug!(thread_id = thread_id_sf.as_i64(), "parsed thread id");

    let row = thread_repo::get_by_id(&state.db, thread_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get thread");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("Thread not found".to_string()))?;

    tracing::info!(thread_id = row.id, "thread fetched successfully");

    Ok(Json(thread_row_to_response(row)))
}

/// GET /threads/{thread_id}/messages — List messages in a thread.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_thread_messages(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(thread_id): Path<String>,
) -> Result<Json<Vec<MessageResponse>>, ApiError> {
    tracing::info!("listing thread messages");

    let thread_id_sf = parse_snowflake(&thread_id)?;
    tracing::debug!(thread_id = thread_id_sf.as_i64(), "parsed thread id");

    let rows = message_repo::list_by_thread(&state.db, thread_id_sf, 50)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to list thread messages");
            ApiError::Database(e)
        })?;

    tracing::info!(count = rows.len(), "thread messages fetched successfully");

    let messages = rows.into_iter().map(message_row_to_response).collect();
    Ok(Json(messages))
}

/// POST /threads/{thread_id}/messages — Send a message in a thread.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
async fn send_thread_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(thread_id): Path<String>,
    Json(req): Json<SendThreadMessageRequest>,
) -> Result<(StatusCode, Json<MessageResponse>), ApiError> {
    tracing::info!("sending message to thread");

    let thread_id_sf = parse_snowflake(&thread_id)?;
    tracing::debug!(thread_id = thread_id_sf.as_i64(), "parsed thread id");

    // Verify thread exists and get channel_id
    let thread_row = thread_repo::get_by_id(&state.db, thread_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to get thread");
            ApiError::Database(e)
        })?
        .ok_or(ApiError::NotFound("Thread not found".to_string()))?;

    let channel_id_sf = opencorde_core::snowflake::Snowflake::new(thread_row.channel_id);

    // Generate message ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let message_id = generator.next_id();
    tracing::debug!(message_id = message_id.as_i64(), "generated message id");

    // Create message in thread
    let row = message_repo::create_message(
        &state.db,
        message_id,
        channel_id_sf,
        auth.user_id,
        &req.content,
        None,
        serde_json::json!([]),
        Some(thread_id_sf),
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create message");
        ApiError::Database(e)
    })?;

    // Increment thread message count
    thread_repo::increment_message_count(&state.db, thread_id_sf)
        .await
        .map_err(|e| {
            tracing::error!(error = %e, "failed to increment message count");
            ApiError::Database(e)
        })?;

    tracing::info!(
        message_id = row.id,
        thread_id = thread_id_sf.as_i64(),
        "message created in thread successfully"
    );

    let response = message_row_to_response(row);

    // Broadcast MessageCreate event
    let event = serde_json::json!({
        "type": "MessageCreate",
        "data": { "message": response }
    });
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MessageCreate event");
    }

    Ok((StatusCode::CREATED, Json(response)))
}
