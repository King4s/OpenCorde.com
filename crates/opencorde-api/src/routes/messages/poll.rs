//! # Poll Vote & Close Handlers
//! HTTP handlers for voting on polls and closing polls.
//!
//! ## Endpoints
//! - POST /api/v1/messages/{id}/vote — Vote on a poll message
//! - POST /api/v1/messages/{id}/poll/close — Close a poll (author only)
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_core::Snowflake (ID parsing)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use opencorde_core::permissions::Permissions;
use tracing::instrument;

use crate::{
    AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check,
};

use super::types::VoteRequest;
use super::validation::parse_snowflake_id;

/// POST /api/v1/messages/{id}/vote — Vote on a poll message.
///
/// Requires authentication. The message must have a poll attached,
/// and the poll must not be closed or expired.
/// If allow_multiselect is false, existing votes for other answers
/// are removed. Duplicate votes for the same answer are rejected.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn vote_on_poll(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<String>,
    Json(req): Json<VoteRequest>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    tracing::info!("voting on poll");

    let message_id_sf = parse_snowflake_id(&message_id)?;
    let user_id_i64 = auth.user_id.as_i64();

    // Fetch the message and its poll
    let row: (Option<serde_json::Value>, i64) = sqlx::query_as(
        "SELECT poll, channel_id FROM messages WHERE id = $1",
    )
    .bind(message_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or(ApiError::NotFound("message not found".to_string()))?;

    let poll_json = row.0.ok_or(ApiError::BadRequest(
        "message does not contain a poll".to_string(),
    ))?;

    let channel_id_sf = opencorde_core::snowflake::Snowflake::new(row.1);

    // Verify the user can view this channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Parse poll data
    let mut poll: serde_json::Value = poll_json;

    // Check if poll is closed
    if poll.get("closed").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(ApiError::BadRequest("poll is closed".to_string()));
    }

    // Check if poll is expired
    if let Some(expires_at) = poll.get("expires_at").and_then(|v| v.as_str()) {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expires_at) {
            if chrono::Utc::now() > expiry {
                return Err(ApiError::BadRequest("poll has expired".to_string()));
            }
        }
    }

    let allow_multiselect = poll
        .get("allow_multiselect")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let answers = poll
        .get_mut("answers")
        .and_then(|a| a.as_array_mut())
        .ok_or(ApiError::BadRequest("poll has no answers".to_string()))?;

    let answer_id = req.answer_id;

    // Find the target answer
    let target_answer = answers
        .iter_mut()
        .find(|a| a.get("id").and_then(|v| v.as_i64()) == Some(answer_id as i64))
        .ok_or(ApiError::BadRequest(format!(
            "answer {} not found in poll",
            answer_id
        )))?;

    let votes = target_answer
        .get_mut("votes")
        .and_then(|v| v.as_array_mut())
        .ok_or(ApiError::InternalServerError("poll answer missing votes array".to_string()))?;

    // Check if user already voted for this answer
    let already_voted = votes.iter().any(|v| v.as_i64() == Some(user_id_i64));
    if already_voted {
        return Err(ApiError::BadRequest(
            "already voted for this answer".to_string(),
        ));
    }

    // If not multiselect, remove user's vote from all other answers
    if !allow_multiselect {
        for answer in answers.iter_mut() {
            if let Some(other_votes) = answer.get_mut("votes").and_then(|v| v.as_array_mut()) {
                other_votes.retain(|v| v.as_i64() != Some(user_id_i64));
            }
        }
    }

    // Add user's vote
    votes.push(serde_json::Value::Number(user_id_i64.into()));

    // Update the poll in the database
    sqlx::query("UPDATE messages SET poll = $1 WHERE id = $2")
        .bind(sqlx::types::Json(&poll))
        .bind(message_id_sf.as_i64())
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        message_id = message_id_sf.as_i64(),
        user_id = user_id_i64,
        answer_id = answer_id,
        "vote recorded"
    );

    Ok((StatusCode::OK, Json(poll)))
}

/// POST /api/v1/messages/{id}/poll/close — Close a poll.
///
/// Requires authentication. Only the message author can close the poll.
/// Closing prevents further votes.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn close_poll(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> Result<(StatusCode, Json<serde_json::Value>), ApiError> {
    tracing::info!("closing poll");

    let message_id_sf = parse_snowflake_id(&message_id)?;

    // Fetch the message to check ownership
    let row: (i64, Option<serde_json::Value>) = sqlx::query_as(
        "SELECT author_id, poll FROM messages WHERE id = $1",
    )
    .bind(message_id_sf.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?
    .ok_or(ApiError::NotFound("message not found".to_string()))?;

    // Only the author can close the poll
    if row.0 != auth.user_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    let mut poll = row.1.ok_or(ApiError::BadRequest(
        "message does not contain a poll".to_string(),
    ))?;

    if poll.get("closed").and_then(|v| v.as_bool()).unwrap_or(false) {
        return Err(ApiError::BadRequest("poll is already closed".to_string()));
    }

    poll["closed"] = serde_json::Value::Bool(true);

    // Update the poll in the database
    sqlx::query("UPDATE messages SET poll = $1 WHERE id = $2")
        .bind(sqlx::types::Json(&poll))
        .bind(message_id_sf.as_i64())
        .execute(&state.db)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(
        message_id = message_id_sf.as_i64(),
        "poll closed"
    );

    Ok((StatusCode::OK, Json(poll)))
}
