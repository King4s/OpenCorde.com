//! # Route: Scheduled Messages
//! Schedule messages for future delivery.
//!
//! ## Endpoints
//! - GET /api/v1/users/@me/scheduled-messages — List my pending scheduled messages
//! - POST /api/v1/channels/{channel_id}/scheduled-messages — Schedule a message
//! - PATCH /api/v1/scheduled-messages/{id} — Edit a scheduled message
//! - DELETE /api/v1/scheduled-messages/{id} — Cancel a scheduled message
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::scheduled_message_repo (database operations)
//! - opencorde_core::Snowflake (ID generation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::Utc;
use opencorde_core::snowflake::SnowflakeGenerator;
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::permission_check};
use opencorde_core::permissions::Permissions;
use opencorde_db::repos::scheduled_message_repo::{self, ScheduledMessageRow};

use crate::routes::messages::validation::{parse_snowflake_id, validate_content};

use axum::{
    Router,
    routing::{get, patch, post},
};

/// Build the scheduled messages router.
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/users/@me/scheduled-messages", get(list_scheduled))
        .route(
            "/api/v1/channels/{channel_id}/scheduled-messages",
            post(schedule_message),
        )
        .route(
            "/api/v1/scheduled-messages/{id}",
            patch(edit_scheduled).delete(cancel_scheduled),
        )
}

/// Request body for scheduling a message.
#[derive(Debug, Deserialize)]
pub struct ScheduleMessageRequest {
    /// Message content (1-4000 characters)
    pub content: String,
    /// ISO 8601 timestamp for when the message should be delivered
    pub scheduled_at: String,
    /// Optional Snowflake ID of the message being replied to
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reply_to_id: Option<String>,
    /// Optional array of attachment objects
    #[serde(default)]
    pub attachments: Option<serde_json::Value>,
}

/// Request body for editing a scheduled message.
#[derive(Debug, Deserialize)]
pub struct EditScheduledRequest {
    /// Updated message content (1-4000 characters, optional)
    pub content: Option<String>,
    /// Updated ISO 8601 delivery timestamp (optional)
    pub scheduled_at: Option<String>,
}

/// Response body for a scheduled message.
#[derive(Debug, Serialize)]
pub struct ScheduledMessageResponse {
    pub id: String,
    pub channel_id: String,
    pub content: String,
    pub attachments: serde_json::Value,
    pub reply_to_id: Option<String>,
    pub scheduled_at: String,
    pub created_at: String,
    pub delivered: bool,
}

fn row_to_response(row: ScheduledMessageRow) -> ScheduledMessageResponse {
    ScheduledMessageResponse {
        id: row.id.to_string(),
        channel_id: row.channel_id.to_string(),
        content: row.content,
        attachments: row.attachments,
        reply_to_id: row.reply_to_id.map(|id| id.to_string()),
        scheduled_at: row.scheduled_at.to_rfc3339(),
        created_at: row.created_at.to_rfc3339(),
        delivered: row.delivered,
    }
}

/// Parse an ISO 8601 string into a DateTime<Utc> that must be in the future.
fn parse_future_time(s: &str) -> Result<chrono::DateTime<Utc>, ApiError> {
    let dt = chrono::DateTime::parse_from_rfc3339(s)
        .map_err(|_| ApiError::BadRequest("invalid scheduled_at format (use ISO 8601)".into()))?;
    let utc = dt.with_timezone(&Utc);
    if utc <= Utc::now() {
        return Err(ApiError::BadRequest(
            "scheduled_at must be in the future".into(),
        ));
    }
    // Cap at 30 days in the future
    let max_future = Utc::now() + chrono::Duration::days(30);
    if utc > max_future {
        return Err(ApiError::BadRequest(
            "scheduled_at must be within 30 days".into(),
        ));
    }
    Ok(utc)
}

/// GET /api/v1/users/@me/scheduled-messages — List current user's pending scheduled messages.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn list_scheduled(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ScheduledMessageResponse>>, ApiError> {
    tracing::info!("listing user scheduled messages");

    let rows = scheduled_message_repo::list_user_scheduled(&state.db, auth.user_id, 100)
        .await
        .map_err(ApiError::Database)?;

    let responses: Vec<_> = rows.into_iter().map(row_to_response).collect();
    Ok(Json(responses))
}

/// POST /api/v1/channels/{channel_id}/scheduled-messages — Schedule a message for future delivery.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn schedule_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
    Json(req): Json<ScheduleMessageRequest>,
) -> Result<(StatusCode, Json<ScheduledMessageResponse>), ApiError> {
    tracing::info!("scheduling message for channel");

    // Parse and validate channel ID
    let channel_id_sf = parse_snowflake_id(&channel_id)?;

    // Verify the user can send messages in this channel NOW
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        channel_id_sf,
        Permissions::SEND_MESSAGES,
    )
    .await?;

    // Validate content
    validate_content(&req.content)?;

    // Parse reply_to_id if provided
    let reply_to_id = req
        .reply_to_id
        .as_deref()
        .map(parse_snowflake_id)
        .transpose()?;

    // Parse and validate scheduled_at
    let scheduled_at = parse_future_time(&req.scheduled_at)?;

    // Generate ID
    let mut generator = SnowflakeGenerator::new(3, 0);
    let scheduled_id = generator.next_id();

    // Create in database
    let attachments = req.attachments.unwrap_or_else(|| serde_json::json!([]));
    let row = scheduled_message_repo::create_scheduled(
        &state.db,
        scheduled_id,
        auth.user_id,
        channel_id_sf,
        &req.content,
        reply_to_id,
        attachments,
        scheduled_at,
    )
    .await
    .map_err(|e| {
        tracing::error!(error = %e, "failed to create scheduled message");
        ApiError::Database(e)
    })?;

    tracing::info!(
        scheduled_id = row.id,
        channel_id = row.channel_id,
        scheduled_at = %row.scheduled_at,
        "scheduled message created"
    );

    let response = row_to_response(row);
    Ok((StatusCode::CREATED, Json(response)))
}

/// PATCH /api/v1/scheduled-messages/{id} — Edit a pending scheduled message.
#[instrument(skip(state, auth, req), fields(user_id = %auth.user_id))]
pub async fn edit_scheduled(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
    Json(req): Json<EditScheduledRequest>,
) -> Result<Json<ScheduledMessageResponse>, ApiError> {
    tracing::info!("editing scheduled message");

    let id_sf = parse_snowflake_id(&id)?;

    // Validate content if provided
    if let Some(ref content) = req.content {
        validate_content(content)?;
    }

    // Parse scheduled_at if provided
    let scheduled_at = req
        .scheduled_at
        .as_deref()
        .map(parse_future_time)
        .transpose()?;

    let row = scheduled_message_repo::update_scheduled(
        &state.db,
        id_sf,
        auth.user_id,
        req.content.as_deref(),
        scheduled_at,
    )
    .await
    .map_err(ApiError::Database)?
    .ok_or_else(|| ApiError::NotFound("scheduled message not found".into()))?;

    Ok(Json(row_to_response(row)))
}

/// DELETE /api/v1/scheduled-messages/{id} — Cancel a pending scheduled message.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
pub async fn cancel_scheduled(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(id): Path<String>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("cancelling scheduled message");

    let id_sf = parse_snowflake_id(&id)?;

    let _row = scheduled_message_repo::cancel_scheduled(&state.db, id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("scheduled message not found".into()))?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_future_time_valid() {
        let future = (Utc::now() + chrono::Duration::hours(1)).to_rfc3339();
        let result = parse_future_time(&future);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_future_time_past() {
        let past = (Utc::now() - chrono::Duration::hours(1)).to_rfc3339();
        let result = parse_future_time(&past);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_future_time_far_future() {
        let far = (Utc::now() + chrono::Duration::days(60)).to_rfc3339();
        let result = parse_future_time(&far);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_future_time_invalid() {
        let result = parse_future_time("not-a-date");
        assert!(result.is_err());
    }
}
