//! # Route: Members - Server membership management

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::{helpers, moderation::audit_mod::log_mod_action, permission_check}};
use opencorde_core::permissions::Permissions;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get},
};
use chrono::{DateTime, Utc};
use opencorde_db::repos::{member_repo, server_repo, user_repo};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize, Clone)]
pub struct MemberResponse {
    pub user_id: String,
    pub server_id: String,
    pub username: String,
    pub nickname: Option<String>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemberRequest {
    pub nickname: Option<String>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/servers/{server_id}/members", get(list_members))
        .route(
            "/api/v1/servers/{server_id}/members/{user_id}",
            delete(remove_member).patch(update_member),
        )
}

fn validate_nickname(nickname: Option<&str>) -> Result<(), ApiError> {
    if let Some(nick) = nickname
        && (nick.is_empty() || nick.len() > 32)
    {
        return Err(ApiError::BadRequest(
            "nickname must be 1-32 characters".into(),
        ));
    }
    Ok(())
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_members(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<MemberResponse>>, ApiError> {
    tracing::info!("listing server members");
    let server_id = helpers::parse_snowflake(&server_id)?;
    server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    let members = member_repo::list_with_usernames_by_server(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(count = members.len(), "members fetched");
    Ok(Json(
        members
            .into_iter()
            .map(|m| MemberResponse {
                user_id: m.user_id.to_string(),
                server_id: m.server_id.to_string(),
                username: m.username,
                nickname: m.nickname,
                joined_at: m.joined_at,
            })
            .collect(),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn remove_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("removing member");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;
    // Kicking another user requires KICK_MEMBERS; leaving (self-remove) is always allowed
    if target_user_id != auth.user_id {
        permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::KICK_MEMBERS).await?;
    }
    member_repo::get_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    member_repo::remove_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(target_user_id = target_user_id.as_i64(), "member removed");
    // Audit: log kick (not self-leave)
    if target_user_id != auth.user_id {
        log_mod_action(&state, server_id, auth.user_id, "member.kick", target_user_id.as_i64()).await;
    }
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_member(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id)): Path<(String, String)>,
    Json(req): Json<UpdateMemberRequest>,
) -> Result<Json<MemberResponse>, ApiError> {
    tracing::debug!("updating member");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let target_user_id = helpers::parse_snowflake(&user_id)?;
    if target_user_id != auth.user_id {
        return Err(ApiError::Forbidden);
    }
    validate_nickname(req.nickname.as_deref())?;
    server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    let member = member_repo::get_member(&state.db, target_user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    let user = user_repo::get_by_id(&state.db, target_user_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("user not found".into()))?;
    member_repo::update_nickname(
        &state.db,
        target_user_id,
        server_id,
        req.nickname.as_deref(),
    )
    .await
    .map_err(ApiError::Database)?;
    tracing::info!(user_id = target_user_id.as_i64(), nickname = ?req.nickname, "member updated");
    Ok(Json(MemberResponse {
        user_id: member.user_id.to_string(),
        server_id: member.server_id.to_string(),
        username: user.username,
        nickname: req.nickname,
        joined_at: member.joined_at,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_nickname_valid() {
        assert!(validate_nickname(Some("ValidNick")).is_ok());
        assert!(validate_nickname(None).is_ok());
    }

    #[test]
    fn test_validate_nickname_empty() {
        assert!(validate_nickname(Some("")).is_err());
    }

    #[test]
    fn test_validate_nickname_too_long() {
        assert!(validate_nickname(Some(&"x".repeat(33))).is_err());
    }
}
