//! # Route: Pinned Messages
//! Endpoints for pinning/unpinning messages in channels.
//!
//! ## Endpoints
//! - GET  /api/v1/channels/{channel_id}/pins — List pinned messages
//! - PUT  /api/v1/channels/{channel_id}/pins/{message_id} — Pin a message
//! - DELETE /api/v1/channels/{channel_id}/pins/{message_id} — Unpin a message
//!
//! ## Depends On
//! - opencorde_db::repos::pin_repo

use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use chrono::{DateTime, Utc};
use opencorde_db::repos::pin_repo;
use serde::Serialize;
use tracing::instrument;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};
use crate::routes::helpers;

#[derive(Debug, Serialize, Clone)]
pub struct PinnedMessageResponse {
    pub message_id: String,
    pub channel_id: String,
    pub author_id: String,
    pub author_username: String,
    pub content: String,
    pub attachments: serde_json::Value,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub pinned_by: String,
    pub pinned_at: DateTime<Utc>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/channels/{channel_id}/pins", get(list_pins))
        .route(
            "/api/v1/channels/{channel_id}/pins/{message_id}",
            put(pin_message).delete(unpin_message),
        )
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_pins(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id): Path<String>,
) -> Result<Json<Vec<PinnedMessageResponse>>, ApiError> {
    tracing::info!(channel_id = %channel_id, "listing pinned messages");
    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;
    let rows = pin_repo::list_pinned(&state.db, channel_id_sf)
        .await
        .map_err(ApiError::Database)?;
    let pins = rows
        .into_iter()
        .map(|r| PinnedMessageResponse {
            message_id: r.message_id.to_string(),
            channel_id: r.channel_id.to_string(),
            author_id: r.author_id.to_string(),
            author_username: r.author_username,
            content: r.content,
            attachments: r.attachments,
            edited_at: r.edited_at,
            created_at: r.created_at,
            pinned_by: r.pinned_by.to_string(),
            pinned_at: r.pinned_at,
        })
        .collect();
    Ok(Json(pins))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn pin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, message_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(channel_id = %channel_id, message_id = %message_id, "pinning message");
    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;
    let message_id_sf = helpers::parse_snowflake(&message_id)?;
    pin_repo::pin_message(&state.db, channel_id_sf, message_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn unpin_message(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((channel_id, message_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(channel_id = %channel_id, message_id = %message_id, "unpinning message");
    let channel_id_sf = helpers::parse_snowflake(&channel_id)?;
    let message_id_sf = helpers::parse_snowflake(&message_id)?;
    let existed = pin_repo::unpin_message(&state.db, channel_id_sf, message_id_sf)
        .await
        .map_err(ApiError::Database)?;
    if !existed {
        return Err(ApiError::NotFound("pin not found".into()));
    }
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_message_response_has_all_fields() {
        let _ = std::mem::size_of::<PinnedMessageResponse>();
    }
}
