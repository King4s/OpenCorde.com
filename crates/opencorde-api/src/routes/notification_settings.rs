//! # Route: Per-Channel Notification Settings
//! Lets users override notification level per-channel (all / mentions / muted).
//!
//! ## Endpoints
//! - GET  /api/v1/users/@me/notification-settings — bulk-fetch all overrides for the current user
//! - PUT  /api/v1/channels/{id}/notification-settings — set level for one channel
//! - DELETE /api/v1/channels/{id}/notification-settings — reset to server default
//!
//! ## Notification Levels
//! - 0 — ALL_MESSAGES (default)
//! - 1 — ONLY_MENTIONS
//! - 2 — MUTED (no notifications)
//!
//! ## Depends On
//! - opencorde_db (PgPool via AppState)
//! - crate::middleware::auth::AuthUser
//! - crate::error::ApiError

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::{AppState, error::ApiError, middleware::auth::AuthUser};
use crate::routes::helpers::parse_snowflake;

/// Wire type for the setting stored per (user, channel).
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ChannelNotifSetting {
    pub channel_id: i64,
    pub level:      i16,
}

/// Request body for PUT.
#[derive(Debug, Deserialize)]
pub struct SetNotifRequest {
    /// Notification level: 0=all, 1=mentions-only, 2=muted
    pub level: i16,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/users/@me/notification-settings", get(list_settings))
        .route(
            "/api/v1/channels/{id}/notification-settings",
            put(set_setting).delete(reset_setting),
        )
}

/// GET /api/v1/users/@me/notification-settings
///
/// Returns all per-channel overrides for the authenticated user.
#[tracing::instrument(skip(state, auth))]
pub async fn list_settings(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<Vec<ChannelNotifSetting>>, ApiError> {
    let rows = sqlx::query_as::<_, ChannelNotifSetting>(
        "SELECT channel_id, level FROM channel_notification_settings WHERE user_id = $1",
    )
    .bind(auth.user_id.as_i64())
    .fetch_all(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(Json(rows))
}

/// PUT /api/v1/channels/{id}/notification-settings
///
/// Upsert a notification level override for the authenticated user on this channel.
#[tracing::instrument(skip(state, auth))]
pub async fn set_setting(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
    Json(body): Json<SetNotifRequest>,
) -> Result<StatusCode, ApiError> {
    let channel_id = parse_snowflake(&channel_id_str)?;

    if body.level < 0 || body.level > 2 {
        return Err(ApiError::BadRequest("level must be 0, 1, or 2".into()));
    }

    sqlx::query(
        "INSERT INTO channel_notification_settings (user_id, channel_id, level, updated_at) \
         VALUES ($1, $2, $3, NOW()) \
         ON CONFLICT (user_id, channel_id) DO UPDATE SET level = $3, updated_at = NOW()",
    )
    .bind(auth.user_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(body.level)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        channel_id = channel_id.as_i64(),
        level = body.level,
        "notification setting updated"
    );
    Ok(StatusCode::NO_CONTENT)
}

/// DELETE /api/v1/channels/{id}/notification-settings
///
/// Remove the override, reverting the channel to the server-level default.
#[tracing::instrument(skip(state, auth))]
pub async fn reset_setting(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(channel_id_str): Path<String>,
) -> Result<StatusCode, ApiError> {
    let channel_id = parse_snowflake(&channel_id_str)?;

    sqlx::query(
        "DELETE FROM channel_notification_settings WHERE user_id = $1 AND channel_id = $2",
    )
    .bind(auth.user_id.as_i64())
    .bind(channel_id.as_i64())
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_valid_levels() {
        // Level range is 0-2; test boundary values
        assert!((0i16..=2).contains(&0));
        assert!((0i16..=2).contains(&1));
        assert!((0i16..=2).contains(&2));
        assert!(!(0i16..=2).contains(&3));
        assert!(!(0i16..=2).contains(&-1));
    }
}
