//! # Route: Reactions - Message emoji reactions
//!
//! ## Endpoints
//! - PUT /api/v1/messages/{message_id}/reactions/{emoji} — Add reaction (204 or 200)
//! - DELETE /api/v1/messages/{message_id}/reactions/{emoji} — Remove reaction (204)
//! - GET /api/v1/messages/{message_id}/reactions — List reactions grouped by emoji
//!
//! ## Depends On
//! - axum (web framework)
//! - opencorde_db::repos::reaction_repo (database operations)
//! - opencorde_db::repos::message_repo (message validation)
//! - crate::middleware::auth::AuthUser (authentication)
//! - crate::AppState (application state)

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::{helpers, permission_check}};
use opencorde_core::{permissions::Permissions, Snowflake};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use opencorde_db::repos::{message_repo, reaction_repo};
use serde::Serialize;
use tracing::instrument;

/// Response type for a single emoji's reaction count.
#[derive(Debug, Serialize)]
pub struct ReactionCount {
    pub emoji: String,
    pub count: i64,
    /// Did the current user react with this emoji?
    pub reacted: bool,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/messages/{message_id}/reactions/{emoji}",
            put(add_reaction).delete(remove_reaction),
        )
        .route(
            "/api/v1/messages/{message_id}/reactions",
            get(list_reactions),
        )
}

/// Validate emoji parameter: must be 1-64 characters (UTF-8 bytes).
fn validate_emoji(emoji: &str) -> Result<(), ApiError> {
    if emoji.is_empty() || emoji.len() > 64 {
        return Err(ApiError::BadRequest(
            "emoji must be 1-64 characters".into(),
        ));
    }
    Ok(())
}

/// PUT /api/v1/messages/{message_id}/reactions/{emoji} — Add a reaction.
///
/// Requires authentication. Idempotent: adding the same reaction twice succeeds
/// but doesn't duplicate the reaction.
///
/// Returns 204 No Content if the reaction already existed,
/// or 200 OK with the updated reaction counts if newly added.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn add_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((message_id, emoji)): Path<(String, String)>,
) -> Result<(StatusCode, Json<Vec<ReactionCount>>), ApiError> {
    tracing::info!("adding reaction to message");

    // Parse and validate message ID
    let message_id_sf = helpers::parse_snowflake(&message_id)?;
    tracing::debug!(message_id = message_id_sf.as_i64(), "parsed message id");

    // Validate emoji
    validate_emoji(&emoji)?;
    tracing::debug!(emoji = &emoji, "validated emoji");

    // Verify message exists (also needed for channel_id in events)
    let msg = message_repo::get_by_id(&state.db, message_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("message not found".into()))?;
    let channel_id_str = msg.channel_id.to_string();
    tracing::debug!("message verified");

    // Require ADD_REACTIONS permission in the message's channel
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        Snowflake::new(msg.channel_id),
        Permissions::VIEW_CHANNEL,
    )
    .await?;
    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        Snowflake::new(msg.channel_id),
        Permissions::ADD_REACTIONS,
    )
    .await?;

    // Add reaction to database
    let is_new = reaction_repo::add_reaction(
        &state.db,
        message_id_sf,
        auth.user_id,
        &emoji,
    )
    .await
    .map_err(ApiError::Database)?;

    // Fetch updated reaction counts
    let reactions = reaction_repo::count_by_emoji(&state.db, message_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    let response: Vec<ReactionCount> = reactions
        .into_iter()
        .map(|(emoji, count, reacted)| ReactionCount {
            emoji,
            count,
            reacted,
        })
        .collect();

    if is_new {
        tracing::info!(message_id = message_id_sf.as_i64(), emoji, "reaction added");

        // Broadcast ReactionAdd event to WebSocket clients (includes channel_id for dispatch filtering)
        let event = serde_json::json!({
            "type": "ReactionAdd",
            "data": {
                "channel_id": channel_id_str,
                "message_id": message_id_sf.to_string(),
                "user_id": auth.user_id.to_string(),
                "emoji": emoji
            }
        });
        if state.event_tx.send(event).is_err() {
            tracing::debug!("no WebSocket subscribers for ReactionAdd event");
        }

        Ok((StatusCode::OK, Json(response)))
    } else {
        tracing::debug!(message_id = message_id_sf.as_i64(), emoji, "reaction already existed");
        Ok((StatusCode::NO_CONTENT, Json(response)))
    }
}

/// DELETE /api/v1/messages/{message_id}/reactions/{emoji} — Remove a reaction.
///
/// Requires authentication. Idempotent: removing a reaction that doesn't exist
/// or wasn't created by the user succeeds silently.
///
/// Returns 204 No Content.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn remove_reaction(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((message_id, emoji)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("removing reaction from message");

    // Parse and validate message ID
    let message_id_sf = helpers::parse_snowflake(&message_id)?;
    tracing::debug!(message_id = message_id_sf.as_i64(), "parsed message id");

    // Validate emoji
    validate_emoji(&emoji)?;
    tracing::debug!(emoji = &emoji, "validated emoji");

    // Verify message exists (also needed for channel_id in events)
    let msg = message_repo::get_by_id(&state.db, message_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("message not found".into()))?;
    let channel_id_str = msg.channel_id.to_string();
    tracing::debug!("message verified");

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        Snowflake::new(msg.channel_id),
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Remove reaction from database
    let was_removed = reaction_repo::remove_reaction(
        &state.db,
        message_id_sf,
        auth.user_id,
        &emoji,
    )
    .await
    .map_err(ApiError::Database)?;

    if was_removed {
        tracing::info!(message_id = message_id_sf.as_i64(), emoji, "reaction removed");

        // Broadcast ReactionRemove event (includes channel_id for dispatch filtering)
        let event = serde_json::json!({
            "type": "ReactionRemove",
            "data": {
                "channel_id": channel_id_str,
                "message_id": message_id_sf.to_string(),
                "user_id": auth.user_id.to_string(),
                "emoji": emoji
            }
        });
        if state.event_tx.send(event).is_err() {
            tracing::debug!("no WebSocket subscribers for ReactionRemove event");
        }
    } else {
        tracing::debug!(message_id = message_id_sf.as_i64(), emoji, "reaction did not exist");
    }

    Ok(StatusCode::NO_CONTENT)
}

/// GET /api/v1/messages/{message_id}/reactions — List reactions grouped by emoji.
///
/// Returns a JSON array of reaction counts, grouped by emoji, ordered by first appearance.
/// Each reaction includes a flag indicating whether the current user has reacted with that emoji.
///
/// Requires authentication.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_reactions(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(message_id): Path<String>,
) -> Result<Json<Vec<ReactionCount>>, ApiError> {
    tracing::info!("listing message reactions");

    // Parse and validate message ID
    let message_id_sf = helpers::parse_snowflake(&message_id)?;
    tracing::debug!(message_id = message_id_sf.as_i64(), "parsed message id");

    // Verify message exists
    let msg = message_repo::get_by_id(&state.db, message_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("message not found".into()))?;
    tracing::debug!("message verified");

    permission_check::require_channel_perm(
        &state.db,
        auth.user_id,
        Snowflake::new(msg.channel_id),
        Permissions::VIEW_CHANNEL,
    )
    .await?;

    // Fetch reaction counts
    let reactions = reaction_repo::count_by_emoji(&state.db, message_id_sf, auth.user_id)
        .await
        .map_err(ApiError::Database)?;

    let response: Vec<ReactionCount> = reactions
        .into_iter()
        .map(|(emoji, count, reacted)| ReactionCount {
            emoji,
            count,
            reacted,
        })
        .collect();

    tracing::info!(count = response.len(), "reactions fetched");
    Ok(Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_emoji_valid() {
        assert!(validate_emoji("👍").is_ok());
        assert!(validate_emoji(":smile:").is_ok());
        assert!(validate_emoji("❤️").is_ok());
    }

    #[test]
    fn test_validate_emoji_empty() {
        assert!(validate_emoji("").is_err());
    }

    #[test]
    fn test_validate_emoji_too_long() {
        let long_emoji = "👍".repeat(65);
        assert!(validate_emoji(&long_emoji).is_err());
    }

    #[test]
    fn test_reaction_count_serialization() {
        let reaction = ReactionCount {
            emoji: "👍".to_string(),
            count: 3,
            reacted: true,
        };
        let json = serde_json::to_string(&reaction).unwrap();
        assert!(json.contains("👍"));
        assert!(json.contains("\"count\":3"));
        assert!(json.contains("\"reacted\":true"));
    }
}
