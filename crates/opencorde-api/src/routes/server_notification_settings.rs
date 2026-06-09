//! # Route: Server Notification Settings
//! Per-server notification defaults, mute, and mention suppression.
//!
//! ## Endpoints
//! - GET  /api/v1/users/@me/servers/{id}/notification-settings
//! - PUT  /api/v1/users/@me/servers/{id}/notification-settings
//! - POST /api/v1/users/@me/servers/{id}/mute
//!
//! ## Notification Levels
//! - 0 — ALL_MESSAGES (default)
//! - 1 — ONLY_MENTIONS
//! - 2 — NOTHING (fully muted)
//!
//! ## Depends On
//! - axum (routing, extractors)
//! - crate::middleware::auth::AuthUser
//! - crate::AppState (database pool)
//! - crate::error::ApiError

use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use tracing::instrument;

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers};

/// Server notification settings as stored in the database.
#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct ServerNotifSettings {
    pub server_id: i64,
    pub level: i16,
    pub mute_until: Option<DateTime<Utc>>,
    pub suppress_everyone: bool,
    pub suppress_here: bool,
    pub suppress_role_mentions: bool,
}

/// Request body for PUT server notification settings.
#[derive(Debug, Deserialize)]
pub struct PutServerNotifRequest {
    /// Notification level: 0=all, 1=mentions, 2=nothing
    pub level: Option<i16>,
    /// Suppress @everyone mentions
    pub suppress_everyone: Option<bool>,
    /// Suppress @here mentions
    pub suppress_here: Option<bool>,
    /// Suppress @role mentions
    pub suppress_role_mentions: Option<bool>,
}

/// Request body for POST mute.
#[derive(Debug, Deserialize)]
pub struct MuteRequest {
    /// When the mute expires (ISO 8601). Pass null to unmute.
    pub until: Option<DateTime<Utc>>,
}

/// Response for mute endpoint.
#[derive(Debug, Serialize)]
pub struct MuteResponse {
    pub mute_until: Option<DateTime<Utc>>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/users/@me/servers/{id}/notification-settings",
            get(get_server_settings).put(put_server_settings),
        )
        .route(
            "/api/v1/users/@me/servers/{id}/mute",
            post(mute_server),
        )
}

/// GET /api/v1/users/@me/servers/{id}/notification-settings
///
/// Returns the server-level notification settings for the authenticated user.
/// If no settings exist, returns default values (level=0, no suppression, no mute).
#[instrument(skip(state, auth))]
async fn get_server_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id_str): Path<String>,
) -> Result<Json<ServerNotifSettings>, ApiError> {
    let server_id = helpers::parse_snowflake(&server_id_str)?;

    let row = sqlx::query_as::<_, ServerNotifSettings>(
        "SELECT server_id, level, mute_until, suppress_everyone, suppress_here, suppress_role_mentions \
         FROM server_notification_settings \
         WHERE user_id = $1 AND server_id = $2",
    )
    .bind(auth.user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    match row {
        Some(settings) => Ok(Json(settings)),
        None => Ok(Json(ServerNotifSettings {
            server_id: server_id.as_i64(),
            level: 0,
            mute_until: None,
            suppress_everyone: false,
            suppress_here: false,
            suppress_role_mentions: false,
        })),
    }
}

/// PUT /api/v1/users/@me/servers/{id}/notification-settings
///
/// Upsert server-level notification settings. Only provided fields are updated;
/// omitted fields keep their current value (or default if no row exists).
#[instrument(skip(state, auth, req))]
async fn put_server_settings(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id_str): Path<String>,
    Json(req): Json<PutServerNotifRequest>,
) -> Result<StatusCode, ApiError> {
    let server_id = helpers::parse_snowflake(&server_id_str)?;

    // Validate level if provided
    if let Some(level) = req.level {
        if !(0..=2).contains(&level) {
            return Err(ApiError::BadRequest(
                "level must be 0, 1, or 2".into(),
            ));
        }
    }

    // Fetch current settings to merge
    let current = sqlx::query_as::<_, ServerNotifSettings>(
        "SELECT server_id, level, mute_until, suppress_everyone, suppress_here, suppress_role_mentions \
         FROM server_notification_settings \
         WHERE user_id = $1 AND server_id = $2",
    )
    .bind(auth.user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_optional(&state.db)
    .await
    .map_err(ApiError::Database)?;

    let level = req.level.unwrap_or(current.as_ref().map(|c| c.level).unwrap_or(0));
    let se = req.suppress_everyone.unwrap_or(current.as_ref().map(|c| c.suppress_everyone).unwrap_or(false));
    let sh = req.suppress_here.unwrap_or(current.as_ref().map(|c| c.suppress_here).unwrap_or(false));
    let sr = req.suppress_role_mentions.unwrap_or(current.as_ref().map(|c| c.suppress_role_mentions).unwrap_or(false));
    let mute_until = current.and_then(|c| c.mute_until);

    sqlx::query(
        "INSERT INTO server_notification_settings \
         (user_id, server_id, level, mute_until, suppress_everyone, suppress_here, suppress_role_mentions, updated_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, NOW()) \
         ON CONFLICT (user_id, server_id) DO UPDATE SET \
         level = $3, mute_until = $4, suppress_everyone = $5, suppress_here = $6, \
         suppress_role_mentions = $7, updated_at = NOW()",
    )
    .bind(auth.user_id.as_i64())
    .bind(server_id.as_i64())
    .bind(level)
    .bind(mute_until)
    .bind(se)
    .bind(sh)
    .bind(sr)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        user_id = auth.user_id.as_i64(),
        server_id = server_id.as_i64(),
        level,
        suppress_everyone = se,
        suppress_here = sh,
        suppress_role_mentions = sr,
        "server notification settings updated"
    );

    Ok(StatusCode::NO_CONTENT)
}

/// POST /api/v1/users/@me/servers/{id}/mute
///
/// Mute a server until a given timestamp, or unmute by passing `null` for `until`.
#[instrument(skip(state, auth, req))]
async fn mute_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id_str): Path<String>,
    Json(req): Json<MuteRequest>,
) -> Result<Json<MuteResponse>, ApiError> {
    let server_id = helpers::parse_snowflake(&server_id_str)?;

    // If mute_until is set and in the past, treat as unmute
    let effective_until = req.until.filter(|t| *t > Utc::now());

    sqlx::query(
        "INSERT INTO server_notification_settings \
         (user_id, server_id, level, mute_until, updated_at) \
         VALUES ($1, $2, 0, $3, NOW()) \
         ON CONFLICT (user_id, server_id) DO UPDATE SET \
         mute_until = $3, updated_at = NOW()",
    )
    .bind(auth.user_id.as_i64())
    .bind(server_id.as_i64())
    .bind(effective_until)
    .execute(&state.db)
    .await
    .map_err(ApiError::Database)?;

    tracing::info!(
        user_id = auth.user_id.as_i64(),
        server_id = server_id.as_i64(),
        mute_until = ?effective_until,
        "server mute updated"
    );

    Ok(Json(MuteResponse {
        mute_until: effective_until,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_levels() {
        assert!((0i16..=2).contains(&0));
        assert!((0i16..=2).contains(&1));
        assert!((0i16..=2).contains(&2));
        assert!(!(0i16..=2).contains(&3));
        assert!(!(0i16..=2).contains(&-1));
    }

    #[test]
    fn test_put_request_deserialization() {
        let json = r#"{"level": 1, "suppress_everyone": true}"#;
        let req: PutServerNotifRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.level, Some(1));
        assert_eq!(req.suppress_everyone, Some(true));
        assert_eq!(req.suppress_here, None);
        assert_eq!(req.suppress_role_mentions, None);
    }

    #[test]
    fn test_mute_request_deserialization() {
        let json = r#"{"until": "2026-12-31T23:59:59Z"}"#;
        let req: MuteRequest = serde_json::from_str(json).unwrap();
        assert!(req.until.is_some());
    }

    #[test]
    fn test_mute_request_unmute() {
        let json = r#"{"until": null}"#;
        let req: MuteRequest = serde_json::from_str(json).unwrap();
        assert!(req.until.is_none());
    }
}
