//! # Admin Handlers
//! Request handlers for admin endpoints.

use axum::{
    extract::{Path, Query, State},
    Json,
};
use sqlx::Row;

use crate::{error::ApiError, middleware::auth::AuthUser, AppState};

use super::types::{AdminServerRow, AdminUserRow, InstanceStats, PaginationQuery};

/// Check if user is admin.
fn is_admin(auth: &AuthUser, state: &AppState) -> bool {
    state
        .config
        .admin_user_ids
        .contains(&auth.user_id.as_i64().to_string())
}

/// GET /api/v1/admin/stats — Get instance statistics.
///
/// Requires admin role. Returns counts of all major resources.
#[tracing::instrument(skip(state, auth))]
pub async fn get_stats(
    State(state): State<AppState>,
    auth: AuthUser,
) -> Result<Json<InstanceStats>, ApiError> {
    tracing::info!(user_id = %auth.user_id, "admin: fetching instance stats");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted stats access");
        return Err(ApiError::Forbidden);
    }

    // Count total users
    let users_row = sqlx::query("SELECT COUNT(*) as count FROM users")
        .fetch_one(&state.db)
        .await?;
    let total_users: i64 = users_row.get("count");

    // Count total servers
    let servers_row = sqlx::query("SELECT COUNT(*) as count FROM servers")
        .fetch_one(&state.db)
        .await?;
    let total_servers: i64 = servers_row.get("count");

    // Count total messages
    let messages_row = sqlx::query("SELECT COUNT(*) as count FROM messages")
        .fetch_one(&state.db)
        .await?;
    let total_messages: i64 = messages_row.get("count");

    // Count total channels
    let channels_row = sqlx::query("SELECT COUNT(*) as count FROM channels")
        .fetch_one(&state.db)
        .await?;
    let total_channels: i64 = channels_row.get("count");

    // Count active voice sessions
    let voice_row = sqlx::query("SELECT COUNT(*) as count FROM voice_states")
        .fetch_one(&state.db)
        .await?;
    let active_voice_sessions: i64 = voice_row.get("count");

    let stats = InstanceStats {
        total_users,
        total_servers,
        total_messages,
        total_channels,
        active_voice_sessions,
    };

    tracing::info!(
        total_users,
        total_servers,
        total_messages,
        total_channels,
        active_voice_sessions,
        "admin: instance stats retrieved"
    );

    Ok(Json(stats))
}

/// GET /api/v1/admin/users — List all users (paginated).
///
/// Requires admin role. Supports limit and offset query parameters.
#[tracing::instrument(skip(state, auth))]
pub async fn list_users(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<AdminUserRow>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, limit = pagination.limit, offset = pagination.offset, "admin: listing users");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted users list");
        return Err(ApiError::Forbidden);
    }

    // Fetch users with pagination
    let rows = sqlx::query(
        "SELECT id, username, email, created_at FROM users ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(pagination.limit)
    .bind(pagination.offset)
    .fetch_all(&state.db)
    .await?;

    let users: Vec<AdminUserRow> = rows
        .iter()
        .map(|row| {
            let id: i64 = row.get("id");
            AdminUserRow {
                id: id.to_string(),
                username: row.get("username"),
                email: row.get("email"),
                created_at: row.get("created_at"),
            }
        })
        .collect();

    tracing::info!(user_count = users.len(), "admin: users listed");

    Ok(Json(users))
}

/// DELETE /api/v1/admin/users/{user_id} — Delete a user account.
///
/// Requires admin role. Permanently deletes the user and related data.
#[tracing::instrument(skip(state, auth))]
pub async fn delete_user(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(user_id): Path<String>,
) -> Result<(), ApiError> {
    tracing::info!(admin_id = %auth.user_id, target_user_id = %user_id, "admin: deleting user");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted user deletion");
        return Err(ApiError::Forbidden);
    }

    // Delete the user (cascading deletes should handle related records)
    let uid: i64 = user_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid user_id".into()))?;
    let result = sqlx::query("DELETE FROM users WHERE id = $1")
        .bind(uid)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        tracing::warn!(user_id = %user_id, "admin: attempted deletion of non-existent user");
        return Err(ApiError::NotFound("user not found".into()));
    }

    tracing::info!(user_id = %user_id, "admin: user deleted");

    Ok(())
}

/// GET /api/v1/admin/servers — List all servers (paginated).
///
/// Requires admin role. Ordered by member count (descending).
#[tracing::instrument(skip(state, auth))]
pub async fn list_servers(
    State(state): State<AppState>,
    auth: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> Result<Json<Vec<AdminServerRow>>, ApiError> {
    tracing::info!(user_id = %auth.user_id, limit = pagination.limit, offset = pagination.offset, "admin: listing servers");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted servers list");
        return Err(ApiError::Forbidden);
    }

    // Fetch servers with pagination, ordered by member count
    let rows = sqlx::query(
        "SELECT id, name, owner_id, member_count, created_at FROM servers ORDER BY member_count DESC LIMIT $1 OFFSET $2"
    )
    .bind(pagination.limit)
    .bind(pagination.offset)
    .fetch_all(&state.db)
    .await?;

    let servers: Vec<AdminServerRow> = rows
        .iter()
        .map(|row| {
            let id: i64 = row.get("id");
            let owner_id: i64 = row.get("owner_id");
            AdminServerRow {
                id: id.to_string(),
                name: row.get("name"),
                owner_id,
                member_count: row.get("member_count"),
                created_at: row.get("created_at"),
            }
        })
        .collect();

    tracing::info!(server_count = servers.len(), "admin: servers listed");

    Ok(Json(servers))
}

/// DELETE /api/v1/admin/servers/{server_id} — Delete a server.
///
/// Requires admin role. Permanently deletes the server and all related data.
#[tracing::instrument(skip(state, auth))]
pub async fn delete_server(
    State(state): State<AppState>,
    auth: AuthUser,
    Path(server_id): Path<String>,
) -> Result<(), ApiError> {
    tracing::info!(admin_id = %auth.user_id, target_server_id = %server_id, "admin: deleting server");

    if !is_admin(&auth, &state) {
        tracing::warn!(user_id = %auth.user_id, "admin: non-admin attempted server deletion");
        return Err(ApiError::Forbidden);
    }

    // Delete the server (cascading deletes should handle related records)
    let sid: i64 = server_id
        .parse()
        .map_err(|_| ApiError::BadRequest("invalid server_id".into()))?;
    let result = sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(sid)
        .execute(&state.db)
        .await?;

    if result.rows_affected() == 0 {
        tracing::warn!(server_id = %server_id, "admin: attempted deletion of non-existent server");
        return Err(ApiError::NotFound("server not found".into()));
    }

    tracing::info!(server_id = %server_id, "admin: server deleted");

    Ok(())
}
