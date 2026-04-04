//! # Route: Invites - Server invite creation and usage

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::{helpers, permission_check}};
use opencorde_core::permissions::Permissions;
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post},
};
use chrono::Utc;
use opencorde_core::Snowflake;
use opencorde_db::repos::{invite_repo, member_repo, server_repo};
use rand::Rng;
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize)]
pub struct InviteResponse {
    pub code: String,
    pub server_id: String,
    pub server_name: String,
    pub creator_id: String,
    pub uses: i32,
    pub max_uses: Option<i32>,
    pub expires_at: Option<chrono::DateTime<Utc>>,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateInviteRequest {
    pub max_uses: Option<i32>,
    pub expires_in: Option<i64>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/servers/{server_id}/invites", post(create_invite).get(list_invites))
        .route("/api/v1/invites/{code}", get(get_invite))
        .route("/api/v1/invites/{code}/join", post(join_invite))
        .route(
            "/api/v1/servers/{server_id}/invites/{code}",
            delete(revoke_invite),
        )
}

fn generate_invite_code() -> String {
    let mut rng = rand::rng();
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    (0..8)
        .map(|_| CHARSET[rng.random_range(0..CHARSET.len())] as char)
        .collect()
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateInviteRequest>,
) -> Result<(StatusCode, Json<InviteResponse>), ApiError> {
    tracing::info!("creating invite");
    let server_id = helpers::parse_snowflake(&server_id)?;
    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::CREATE_INVITE).await?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    let code = generate_invite_code();
    let expires_at = req
        .expires_in
        .map(|secs| Utc::now() + chrono::Duration::seconds(secs));
    let invite = invite_repo::create_invite(
        &state.db,
        &code,
        server_id,
        auth.user_id,
        req.max_uses,
        expires_at,
    )
    .await
    .map_err(ApiError::Database)?;
    tracing::info!(code = %code, "invite created");
    Ok((
        StatusCode::CREATED,
        Json(InviteResponse {
            code: invite.code,
            server_id: server.id.to_string(),
            server_name: server.name,
            creator_id: invite.creator_id.to_string(),
            uses: invite.uses,
            max_uses: invite.max_uses,
            expires_at: invite.expires_at,
            created_at: invite.created_at,
        }),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_invites(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<InviteResponse>>, ApiError> {
    tracing::info!("listing server invites");
    let server_id_sf = helpers::parse_snowflake(&server_id)?;

    // Any member with CREATE_INVITE can list invites
    permission_check::require_server_perm(&state.db, auth.user_id, server_id_sf, Permissions::CREATE_INVITE).await?;

    let server = server_repo::get_by_id(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    let invites = invite_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?;

    tracing::info!(count = invites.len(), "invites listed");

    Ok(Json(invites.into_iter().map(|inv| InviteResponse {
        code: inv.code,
        server_id: server.id.to_string(),
        server_name: server.name.clone(),
        creator_id: inv.creator_id.to_string(),
        uses: inv.uses,
        max_uses: inv.max_uses,
        expires_at: inv.expires_at,
        created_at: inv.created_at,
    }).collect()))
}

#[instrument(skip(state))]
async fn get_invite(
    State(state): State<AppState>,
    Path(code): Path<String>,
) -> Result<Json<InviteResponse>, ApiError> {
    tracing::debug!(code = %code, "fetching invite");
    let invite = invite_repo::get_by_code(&state.db, &code)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("invite not found".into()))?;
    if invite.expires_at.is_some_and(|t| t < Utc::now()) {
        return Err(ApiError::NotFound("invite expired".into()));
    }
    if invite.max_uses.is_some_and(|m| invite.uses >= m) {
        return Err(ApiError::NotFound("invite exhausted".into()));
    }
    let server = server_repo::get_by_id(&state.db, Snowflake::new(invite.server_id))
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("server not found")))?;
    Ok(Json(InviteResponse {
        code: invite.code,
        server_id: server.id.to_string(),
        server_name: server.name,
        creator_id: invite.creator_id.to_string(),
        uses: invite.uses,
        max_uses: invite.max_uses,
        expires_at: invite.expires_at,
        created_at: invite.created_at,
    }))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn join_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(code): Path<String>,
) -> Result<(StatusCode, Json<InviteResponse>), ApiError> {
    tracing::info!(code = %code, "joining server via invite");
    let invite = invite_repo::get_by_code(&state.db, &code)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("invite not found".into()))?;
    if invite.expires_at.is_some_and(|t| t < Utc::now()) {
        return Err(ApiError::Conflict("invite expired".into()));
    }
    if invite.max_uses.is_some_and(|m| invite.uses >= m) {
        return Err(ApiError::Conflict("invite exhausted".into()));
    }
    let server_id = Snowflake::new(invite.server_id);
    if member_repo::get_member(&state.db, auth.user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .is_some()
    {
        return Err(ApiError::Conflict("already a member".into()));
    }

    // Check server verification level
    crate::routes::helpers::check_verification_level(
        &state.db,
        auth.user_id,
        server_id,
        false,  // no member tenure check on join
    )
    .await?;

    member_repo::add_member(&state.db, auth.user_id, server_id)
        .await
        .map_err(ApiError::Database)?;
    invite_repo::increment_uses(&state.db, &code)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(server_id = server_id.as_i64(), "user joined server");
    let updated_invite = invite_repo::get_by_code(&state.db, &code)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("invite disappeared")))?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("server disappeared")))?;
    Ok((
        StatusCode::OK,
        Json(InviteResponse {
            code: updated_invite.code,
            server_id: server.id.to_string(),
            server_name: server.name,
            creator_id: updated_invite.creator_id.to_string(),
            uses: updated_invite.uses,
            max_uses: updated_invite.max_uses,
            expires_at: updated_invite.expires_at,
            created_at: updated_invite.created_at,
        }),
    ))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn revoke_invite(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, code)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!(code = %code, "revoking invite");
    let server_id = helpers::parse_snowflake(&server_id)?;
    // Revoking requires MANAGE_SERVER (moderators can revoke any invite)
    permission_check::require_server_perm(&state.db, auth.user_id, server_id, Permissions::MANAGE_SERVER).await?;
    let invite = invite_repo::get_by_code(&state.db, &code)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("invite not found".into()))?;
    if invite.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("invite not found".into()));
    }
    invite_repo::delete_invite(&state.db, &code)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(code = %code, "invite revoked");
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_invite_code() {
        let code = generate_invite_code();
        assert_eq!(code.len(), 8);
        assert!(code.chars().all(|c| c.is_alphanumeric()));
    }
}
