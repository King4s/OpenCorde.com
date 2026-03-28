//! # Route: Moderation - Ban, kick, and timeout management

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::{helpers, permission_check}};
use opencorde_core::permissions::Permissions;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, put},
};
use chrono::Utc;
use opencorde_db::repos::{ban_repo, member_repo};
use serde::{Deserialize, Serialize};
use tracing::instrument;

pub mod audit_mod;
use audit_mod::log_mod_action;

#[derive(Debug, Serialize)]
pub struct BanResponse {
    pub server_id: String,
    pub user_id: String,
    pub reason: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct TimeoutResponse {
    pub server_id: String,
    pub user_id: String,
    pub timeout_until: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct BanRequest {
    pub reason: Option<String>,
    pub delete_messages: bool,
}

#[derive(Debug, Deserialize)]
pub struct TimeoutRequest {
    pub duration_seconds: i64,
    pub reason: Option<String>,
}

const MAX_TIMEOUT_SECONDS: i64 = 2419200; // 28 days

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/bans/{user_id}",
            put(ban_user).delete(unban_user),
        )
        .route("/api/v1/servers/{server_id}/bans", get(list_bans))
        .route(
            "/api/v1/servers/{server_id}/members/{user_id}/timeout",
            put(set_timeout).delete(remove_timeout),
        )
}

fn validate_timeout_duration(duration_seconds: i64) -> Result<(), ApiError> {
    if duration_seconds <= 0 {
        return Err(ApiError::BadRequest(
            "duration must be positive".into(),
        ));
    }
    if duration_seconds > MAX_TIMEOUT_SECONDS {
        return Err(ApiError::BadRequest(
            format!("max timeout duration is {} seconds", MAX_TIMEOUT_SECONDS),
        ));
    }
    Ok(())
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn ban_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
    Json(req): Json<BanRequest>,
) -> Result<(StatusCode, Json<BanResponse>), ApiError> {
    tracing::info!("banning user from server");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;

    // require_server_perm already verifies server exists and user has BAN_MEMBERS
    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::BAN_MEMBERS).await?;

    member_repo::get_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;

    ban_repo::ban_user(
        &state.db,
        server_id.as_i64(),
        target_user_id.as_i64(),
        auth.user_id.as_i64(),
        req.reason.as_deref(),
    )
    .await
    .map_err(ApiError::Database)?;

    member_repo::remove_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?;

    log_mod_action(&state, server_id, auth.user_id, "member.ban", target_user_id.as_i64()).await;

    tracing::info!(target_user_id = target_user_id.as_i64(), "user banned and removed from server");

    Ok((
        StatusCode::CREATED,
        Json(BanResponse {
            server_id: server_id.as_i64().to_string(),
            user_id: target_user_id.as_i64().to_string(),
            reason: req.reason,
        }),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn unban_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("unbanning user from server");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::BAN_MEMBERS).await?;

    let ban_existed = ban_repo::unban_user(&state.db, server_id.as_i64(), target_user_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    if !ban_existed {
        return Err(ApiError::NotFound("ban not found".into()));
    }

    log_mod_action(&state, server_id, auth.user_id, "member.unban", target_user_id.as_i64()).await;

    tracing::info!(target_user_id = target_user_id.as_i64(), "user unbanned");
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_bans(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<BanResponse>>, ApiError> {
    tracing::info!("listing server bans");
    let server_id = helpers::parse_snowflake(&server_id)?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::BAN_MEMBERS).await?;

    let bans = ban_repo::list_bans(&state.db, server_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = bans.len(), "bans fetched");

    Ok(Json(
        bans.into_iter()
            .map(|b| BanResponse {
                server_id: b.server_id.to_string(),
                user_id: b.user_id.to_string(),
                reason: b.reason,
            })
            .collect(),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn set_timeout(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
    Json(req): Json<TimeoutRequest>,
) -> Result<(StatusCode, Json<TimeoutResponse>), ApiError> {
    tracing::info!("setting user timeout");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;

    validate_timeout_duration(req.duration_seconds)?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::MODERATE_MEMBERS).await?;

    member_repo::get_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;

    let timeout_until = Utc::now() + chrono::Duration::seconds(req.duration_seconds);

    ban_repo::set_timeout(
        &state.db,
        server_id.as_i64(),
        target_user_id.as_i64(),
        timeout_until,
        req.reason.as_deref(),
    )
    .await
    .map_err(ApiError::Database)?;

    log_mod_action(&state, server_id, auth.user_id, "member.timeout", target_user_id.as_i64()).await;

    tracing::info!(
        target_user_id = target_user_id.as_i64(),
        timeout_until = ?timeout_until,
        "user timeout set"
    );

    Ok((
        StatusCode::CREATED,
        Json(TimeoutResponse {
            server_id: server_id.as_i64().to_string(),
            user_id: target_user_id.as_i64().to_string(),
            timeout_until,
        }),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn remove_timeout(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("removing user timeout");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;

    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::MODERATE_MEMBERS).await?;

    let timeout = ban_repo::get_timeout(&state.db, server_id.as_i64(), target_user_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    if timeout.is_none() {
        return Err(ApiError::NotFound("timeout not found".into()));
    }

    ban_repo::remove_timeout(&state.db, server_id.as_i64(), target_user_id.as_i64())
        .await
        .map_err(ApiError::Database)?;

    log_mod_action(&state, server_id, auth.user_id, "member.timeout_removed", target_user_id.as_i64()).await;

    tracing::info!(target_user_id = target_user_id.as_i64(), "user timeout removed");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_timeout_zero_duration() {
        assert!(validate_timeout_duration(0).is_err());
    }

    #[test]
    fn test_validate_timeout_negative_duration() {
        assert!(validate_timeout_duration(-100).is_err());
    }

    #[test]
    fn test_validate_timeout_valid() {
        assert!(validate_timeout_duration(3600).is_ok());
        assert!(validate_timeout_duration(MAX_TIMEOUT_SECONDS).is_ok());
    }
}
