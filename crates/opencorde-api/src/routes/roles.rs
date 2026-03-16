//! # Route: Roles - Server role management

use crate::{AppState, error::ApiError, middleware::auth::AuthUser, routes::helpers};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{patch, post, put},
};
use chrono::{DateTime, Utc};
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{member_repo, role_repo, server_repo};
use serde::{Deserialize, Serialize};
use tracing::instrument;

#[derive(Debug, Serialize, Clone)]
pub struct RoleResponse {
    pub id: String,
    pub server_id: String,
    pub name: String,
    pub permissions: u64,
    pub color: Option<i32>,
    pub position: i32,
    pub mentionable: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub name: String,
    pub permissions: u64,
    pub color: Option<i32>,
    pub mentionable: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: Option<String>,
    pub permissions: Option<u64>,
    pub color: Option<i32>,
    pub position: Option<i32>,
    pub mentionable: Option<bool>,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/api/v1/servers/{server_id}/roles", post(create_role))
        .route(
            "/api/v1/servers/{server_id}/roles/{role_id}",
            patch(update_role).delete(delete_role),
        )
        .route(
            "/api/v1/servers/{server_id}/members/{user_id}/roles/{role_id}",
            put(assign_role).delete(unassign_role),
        )
}

fn validate_role_name(name: &str) -> Result<(), ApiError> {
    if name.is_empty() || name.len() > 100 {
        return Err(ApiError::BadRequest(
            "role name must be 1-100 characters".into(),
        ));
    }
    Ok(())
}

fn role_row_to_response(row: role_repo::RoleRow) -> RoleResponse {
    RoleResponse {
        id: row.id.to_string(),
        server_id: row.server_id.to_string(),
        name: row.name,
        permissions: row.permissions as u64,
        color: row.color,
        position: row.position,
        mentionable: row.mentionable,
        created_at: row.created_at,
    }
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn create_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<CreateRoleRequest>,
) -> Result<(StatusCode, Json<RoleResponse>), ApiError> {
    tracing::info!(name = %req.name, "creating role");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    helpers::check_server_owner(auth.user_id, server.owner_id)?;
    validate_role_name(&req.name)?;
    let mut role_gen = SnowflakeGenerator::new(2, 0);
    let role_id = role_gen.next_id();
    let role = role_repo::create_role(
        &state.db,
        role_id,
        server_id,
        &req.name,
        req.permissions as i64,
        req.color,
        req.mentionable.unwrap_or(false),
    )
    .await
    .map_err(ApiError::Database)?;
    tracing::info!(role_id = role.id, "role created");
    Ok((StatusCode::CREATED, Json(role_row_to_response(role))))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn update_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, role_id)): Path<(String, String)>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<RoleResponse>, ApiError> {
    tracing::debug!("updating role");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let role_id = helpers::parse_snowflake(&role_id)?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    helpers::check_server_owner(auth.user_id, server.owner_id)?;
    let role = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
    if role.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("role not found".into()));
    }
    let new_name = req.name.as_deref().unwrap_or(&role.name);
    if let Some(name) = &req.name {
        validate_role_name(name)?;
    }
    let new_perms = req
        .permissions
        .map(|p| p as i64)
        .unwrap_or(role.permissions);
    let new_color = req.color.or(role.color);
    let new_position = req.position.unwrap_or(role.position);
    let new_mentionable = req.mentionable.unwrap_or(role.mentionable);
    role_repo::update_role(
        &state.db,
        role_id,
        new_name,
        new_perms,
        new_color,
        new_position,
        new_mentionable,
    )
    .await
    .map_err(ApiError::Database)?;
    tracing::info!(role_id = role.id, name = %new_name, "role updated");
    let updated = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("role disappeared")))?;
    Ok(Json(role_row_to_response(updated)))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn delete_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, role_id)): Path<(String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!("deleting role");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let role_id = helpers::parse_snowflake(&role_id)?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    helpers::check_server_owner(auth.user_id, server.owner_id)?;
    let role = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
    if role.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("role not found".into()));
    }
    role_repo::delete_role(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(role_id = role.id, "role deleted");
    Ok(StatusCode::NO_CONTENT)
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn assign_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id, role_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::info!("assigning role to member");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let user_id = helpers::parse_snowflake(&user_id)?;
    let role_id = helpers::parse_snowflake(&role_id)?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    helpers::check_server_owner(auth.user_id, server.owner_id)?;
    member_repo::get_member(&state.db, user_id, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("member not found".into()))?;
    let role = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
    if role.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("role not found".into()));
    }
    member_repo::add_role(&state.db, user_id, server_id, role_id)
        .await
        .map_err(|e| {
            if e.to_string().contains("duplicate") {
                ApiError::Conflict("member already has role".into())
            } else {
                ApiError::Database(e)
            }
        })?;
    tracing::info!(
        user_id = user_id.as_i64(),
        role_id = role.id,
        "role assigned"
    );
    Ok(StatusCode::OK)
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn unassign_role(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, user_id, role_id)): Path<(String, String, String)>,
) -> Result<StatusCode, ApiError> {
    tracing::debug!("removing role from member");
    let server_id = helpers::parse_snowflake(&server_id)?;
    let user_id = helpers::parse_snowflake(&user_id)?;
    let role_id = helpers::parse_snowflake(&role_id)?;
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;
    helpers::check_server_owner(auth.user_id, server.owner_id)?;
    member_repo::remove_role(&state.db, user_id, server_id, role_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(
        user_id = user_id.as_i64(),
        role_id = role_id.as_i64(),
        "role removed"
    );
    Ok(StatusCode::NO_CONTENT)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_role_name_valid() {
        assert!(validate_role_name("Moderator").is_ok());
    }

    #[test]
    fn test_validate_role_name_empty() {
        assert!(validate_role_name("").is_err());
    }
}
