//! # Route: Roles - Server role management

use crate::{
    AppState,
    error::ApiError,
    middleware::auth::AuthUser,
    routes::{helpers, moderation::audit_mod::log_mod_action, permission_check},
};
use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, patch, post, put},
};
use chrono::{DateTime, Utc};
use opencorde_core::Snowflake;
use opencorde_core::permissions::Permissions;
use opencorde_core::snowflake::SnowflakeGenerator;
use opencorde_db::repos::{member_repo, role_repo, server_repo};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet};
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

#[derive(Debug, Deserialize)]
pub struct ReorderRoleRequest {
    pub id: String,
    pub position: i32,
}

pub fn router() -> Router<AppState> {
    Router::new()
        .route(
            "/api/v1/servers/{server_id}/roles",
            post(create_role).get(list_roles).patch(reorder_roles),
        )
        .route(
            "/api/v1/servers/{server_id}/roles/{role_id}",
            patch(update_role).delete(delete_role),
        )
        .route(
            "/api/v1/servers/{server_id}/members/{user_id}/roles",
            get(get_member_roles),
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

async fn highest_role_position(
    state: &AppState,
    server_id: Snowflake,
    user_id: Snowflake,
) -> Result<i32, ApiError> {
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if server.owner_id == user_id.as_i64() {
        return Ok(i32::MAX);
    }

    let roles = role_repo::list_by_member(&state.db, user_id, server_id)
        .await
        .map_err(ApiError::Database)?;
    Ok(roles
        .into_iter()
        .map(|role| role.position)
        .max()
        .unwrap_or(0))
}

async fn require_role_below_actor(
    state: &AppState,
    server_id: Snowflake,
    actor_id: Snowflake,
    role: &role_repo::RoleRow,
) -> Result<(), ApiError> {
    let actor_position = highest_role_position(state, server_id, actor_id).await?;
    if role.position >= actor_position {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

async fn require_position_below_actor(
    state: &AppState,
    server_id: Snowflake,
    actor_id: Snowflake,
    position: i32,
) -> Result<(), ApiError> {
    let actor_position = highest_role_position(state, server_id, actor_id).await?;
    if position >= actor_position {
        return Err(ApiError::Forbidden);
    }
    Ok(())
}

async fn require_member_below_actor(
    state: &AppState,
    server_id: Snowflake,
    actor_id: Snowflake,
    target_id: Snowflake,
) -> Result<(), ApiError> {
    let server = server_repo::get_by_id(&state.db, server_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("server not found".into()))?;

    if server.owner_id == actor_id.as_i64() {
        return Ok(());
    }
    if server.owner_id == target_id.as_i64() {
        return Err(ApiError::Forbidden);
    }

    let actor_position = highest_role_position(state, server_id, actor_id).await?;
    let target_position = highest_role_position(state, server_id, target_id).await?;
    if target_position >= actor_position {
        return Err(ApiError::Forbidden);
    }

    Ok(())
}

async fn require_actor_can_set_permissions(
    state: &AppState,
    server_id: Snowflake,
    actor_id: Snowflake,
    permissions: u64,
) -> Result<i64, ApiError> {
    let requested = Permissions::from_bits(permissions)
        .ok_or_else(|| ApiError::BadRequest("invalid permission bits".into()))?;
    let effective =
        permission_check::effective_server_perms(&state.db, actor_id, server_id).await?;

    if !effective.contains(requested) {
        return Err(ApiError::Forbidden);
    }

    i64::try_from(permissions)
        .map_err(|_| ApiError::BadRequest("permission bits exceed storage range".into()))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn list_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<Json<Vec<RoleResponse>>, ApiError> {
    tracing::info!("listing roles for server");
    let server_id_sf = helpers::parse_snowflake(&server_id)?;
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;
    let roles = role_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(roles.into_iter().map(role_row_to_response).collect()))
}

#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn reorder_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
    Json(req): Json<Vec<ReorderRoleRequest>>,
) -> Result<Json<Vec<RoleResponse>>, ApiError> {
    tracing::info!(count = req.len(), "reordering roles for server");
    let server_id_sf = helpers::parse_snowflake(&server_id)?;
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id_sf,
        Permissions::MANAGE_ROLES,
    )
    .await?;

    let roles = role_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?;
    let roles_by_id: HashMap<i64, role_repo::RoleRow> =
        roles.into_iter().map(|role| (role.id, role)).collect();

    let mut seen_roles = HashSet::new();
    let mut seen_positions = HashSet::new();
    let mut parsed = Vec::with_capacity(req.len());
    for item in req {
        let role_id = helpers::parse_snowflake(&item.id)?;
        if !seen_roles.insert(role_id.as_i64()) {
            return Err(ApiError::BadRequest("duplicate role id in reorder".into()));
        }
        if !seen_positions.insert(item.position) {
            return Err(ApiError::BadRequest(
                "duplicate role position in reorder".into(),
            ));
        }
        let role = roles_by_id
            .get(&role_id.as_i64())
            .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
        require_role_below_actor(&state, server_id_sf, auth.user_id, role).await?;
        require_position_below_actor(&state, server_id_sf, auth.user_id, item.position).await?;
        parsed.push((role_id, item.position));
    }

    for (role_id, position) in parsed {
        sqlx::query("UPDATE roles SET position = $1 WHERE id = $2 AND server_id = $3")
            .bind(position)
            .bind(role_id.as_i64())
            .bind(server_id_sf.as_i64())
            .execute(&state.db)
            .await
            .map_err(ApiError::Database)?;
    }

    let updated_roles = role_repo::list_by_server(&state.db, server_id_sf)
        .await
        .map_err(ApiError::Database)?;
    let responses: Vec<RoleResponse> = updated_roles
        .into_iter()
        .map(role_row_to_response)
        .collect();

    for response in &responses {
        let event = json!({"type":"RoleUpdate","data":{"server_id":response.server_id.clone(),"role":response}});
        if state.event_tx.send(event).is_err() {
            tracing::debug!("no WebSocket subscribers for RoleUpdate event");
        }
    }

    log_mod_action(
        &state,
        server_id_sf,
        auth.user_id,
        "role.reorder",
        server_id_sf.as_i64(),
    )
    .await;

    Ok(Json(responses))
}

/// GET /api/v1/servers/{server_id}/members/{user_id}/roles — Get a member's roles.
///
/// Returns all roles assigned to a specific member. Requires authentication.
#[instrument(skip(state, auth), fields(user_id = %auth.user_id))]
async fn get_member_roles(
    State(state): State<AppState>,
    auth: AuthUser,
    Path((server_id, target_user_id)): Path<(String, String)>,
) -> Result<Json<Vec<RoleResponse>>, ApiError> {
    tracing::info!(target_user_id = %target_user_id, "listing member roles");
    let server_id_sf = helpers::parse_snowflake(&server_id)?;
    let target_id = helpers::parse_snowflake(&target_user_id)?;
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id_sf,
        Permissions::VIEW_CHANNEL,
    )
    .await?;
    let roles = role_repo::list_by_member(&state.db, target_id, server_id_sf)
        .await
        .map_err(ApiError::Database)?;
    Ok(Json(roles.into_iter().map(role_row_to_response).collect()))
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
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;
    validate_role_name(&req.name)?;
    let requested_permissions =
        require_actor_can_set_permissions(&state, server_id, auth.user_id, req.permissions).await?;
    let mut role_gen = SnowflakeGenerator::new(2, 0);
    let role_id = role_gen.next_id();
    let role = role_repo::create_role(
        &state.db,
        role_id,
        server_id,
        &req.name,
        requested_permissions,
        req.color,
        req.mentionable.unwrap_or(false),
    )
    .await
    .map_err(ApiError::Database)?;
    tracing::info!(role_id = role.id, "role created");
    log_mod_action(&state, server_id, auth.user_id, "role.create", role.id).await;
    let response = role_row_to_response(role);
    let event = json!({"type":"RoleCreate","data":{"server_id":server_id.as_i64().to_string(),"role":&response}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for RoleCreate event");
    }
    Ok((StatusCode::CREATED, Json(response)))
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
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;
    let role = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
    if role.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("role not found".into()));
    }
    require_role_below_actor(&state, server_id, auth.user_id, &role).await?;
    let new_name = req.name.as_deref().unwrap_or(&role.name);
    if let Some(name) = &req.name {
        validate_role_name(name)?;
    }
    let new_perms = if let Some(permissions) = req.permissions {
        require_actor_can_set_permissions(&state, server_id, auth.user_id, permissions).await?
    } else {
        role.permissions
    };
    let new_color = req.color.or(role.color);
    let new_position = req.position.unwrap_or(role.position);
    require_position_below_actor(&state, server_id, auth.user_id, new_position).await?;
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
    log_mod_action(
        &state,
        server_id,
        auth.user_id,
        "role.update",
        role_id.as_i64(),
    )
    .await;
    let updated = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::Internal(anyhow::anyhow!("role disappeared")))?;
    let response = role_row_to_response(updated);
    let event = json!({"type":"RoleUpdate","data":{"server_id":response.server_id.clone(),"role":&response}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for RoleUpdate event");
    }
    Ok(Json(response))
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
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;
    let role = role_repo::get_by_id(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?
        .ok_or_else(|| ApiError::NotFound("role not found".into()))?;
    if role.server_id != server_id.as_i64() {
        return Err(ApiError::NotFound("role not found".into()));
    }
    require_role_below_actor(&state, server_id, auth.user_id, &role).await?;
    role_repo::delete_role(&state.db, role_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(role_id = role.id, "role deleted");
    log_mod_action(
        &state,
        server_id,
        auth.user_id,
        "role.delete",
        role_id.as_i64(),
    )
    .await;
    let event = json!({"type":"RoleDelete","data":{"server_id":server_id.as_i64().to_string(),"role_id":role_id.as_i64().to_string()}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for RoleDelete event");
    }
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
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;
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
    require_role_below_actor(&state, server_id, auth.user_id, &role).await?;
    require_member_below_actor(&state, server_id, auth.user_id, user_id).await?;
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
    log_mod_action(
        &state,
        server_id,
        auth.user_id,
        "member.role_assign",
        user_id.as_i64(),
    )
    .await;
    let event = json!({"type":"MemberUpdate","data":{"server_id":server_id.as_i64().to_string(),"member":{"user_id":user_id.as_i64().to_string()}}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MemberUpdate event");
    }
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
    permission_check::require_server_perm(
        &state.db,
        auth.user_id,
        server_id,
        Permissions::MANAGE_ROLES,
    )
    .await?;
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
    require_role_below_actor(&state, server_id, auth.user_id, &role).await?;
    require_member_below_actor(&state, server_id, auth.user_id, user_id).await?;
    member_repo::remove_role(&state.db, user_id, server_id, role_id)
        .await
        .map_err(ApiError::Database)?;
    tracing::info!(
        user_id = user_id.as_i64(),
        role_id = role_id.as_i64(),
        "role removed"
    );
    log_mod_action(
        &state,
        server_id,
        auth.user_id,
        "member.role_remove",
        user_id.as_i64(),
    )
    .await;
    let event = json!({"type":"MemberUpdate","data":{"server_id":server_id.as_i64().to_string(),"member":{"user_id":user_id.as_i64().to_string()}}});
    if state.event_tx.send(event).is_err() {
        tracing::debug!("no WebSocket subscribers for MemberUpdate event");
    }
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
