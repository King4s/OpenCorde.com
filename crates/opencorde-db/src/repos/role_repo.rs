//! # Repository: Server Roles
//! CRUD operations for server roles and their permissions.
//!
//! Manages role creation, deletion, and updates.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading role data from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RoleRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub permissions: i64,
    pub color: Option<i32>,
    pub position: i32,
    pub mentionable: bool,
    pub created_at: DateTime<Utc>,
}

/// Create a new role for a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the role
/// * `server_id` - Snowflake ID of the server
/// * `name` - Role name
/// * `permissions` - 64-bit permission bitfield
/// * `color` - Optional color (RGB integer)
/// * `mentionable` - Whether role is mentionable
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn create_role(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    name: &str,
    permissions: i64,
    color: Option<i32>,
    mentionable: bool,
) -> Result<RoleRow, sqlx::Error> {
    tracing::info!(
        role_id = id.as_i64(),
        server_id = server_id.as_i64(),
        name = %name,
        "creating role"
    );

    let row = sqlx::query_as::<_, RoleRow>(
        "INSERT INTO roles (id, server_id, name, permissions, color, mentionable) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(permissions)
    .bind(color)
    .bind(mentionable)
    .fetch_one(pool)
    .await?;

    tracing::info!(role_id = row.id, "role created");
    Ok(row)
}

/// Get a role by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<RoleRow>, sqlx::Error> {
    sqlx::query_as::<_, RoleRow>("SELECT * FROM roles WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all roles in a server, ordered by position.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<RoleRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing server roles");

    sqlx::query_as::<_, RoleRow>("SELECT * FROM roles WHERE server_id = $1 ORDER BY position ASC")
        .bind(server_id.as_i64())
        .fetch_all(pool)
        .await
}

/// Update a role's properties.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID of the role
/// * `name` - New role name
/// * `permissions` - New permission bitfield
/// * `color` - New color (optional)
/// * `position` - New position
/// * `mentionable` - New mentionable flag
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_role(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    permissions: i64,
    color: Option<i32>,
    position: i32,
    mentionable: bool,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        role_id = id.as_i64(),
        name = %name,
        position = position,
        "updating role"
    );

    sqlx::query(
        "UPDATE roles SET name = $1, permissions = $2, color = $3, position = $4, mentionable = $5 \
         WHERE id = $6",
    )
    .bind(name)
    .bind(permissions)
    .bind(color)
    .bind(position)
    .bind(mentionable)
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete a role by its ID.
///
/// Cascades to remove all member role assignments.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_role(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(role_id = id.as_i64(), "deleting role");

    sqlx::query("DELETE FROM roles WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// List all roles assigned to a specific member in a server.
///
/// Returns full role rows joined through member_roles, ordered by position.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_member(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<Vec<RoleRow>, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        "listing member roles"
    );

    sqlx::query_as::<_, RoleRow>(
        "SELECT r.* FROM roles r \
         INNER JOIN member_roles mr ON mr.role_id = r.id \
         WHERE mr.user_id = $1 AND mr.server_id = $2 \
         ORDER BY r.position ASC",
    )
    .bind(user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_role_row_creation() {
        let now = Utc::now();
        let row = RoleRow {
            id: 111222333,
            server_id: 555666777,
            name: "Moderator".to_string(),
            permissions: 0b1111,
            color: Some(0xFF0000),
            position: 1,
            mentionable: true,
            created_at: now,
        };

        assert_eq!(row.id, 111222333);
        assert_eq!(row.name, "Moderator");
        assert_eq!(row.permissions, 0b1111);
        assert_eq!(row.color, Some(0xFF0000));
        assert!(row.mentionable);
    }

    #[test]
    fn test_role_row_no_color() {
        let now = Utc::now();
        let row = RoleRow {
            id: 999888777,
            server_id: 555666777,
            name: "Member".to_string(),
            permissions: 0b10,
            color: None,
            position: 0,
            mentionable: false,
            created_at: now,
        };

        assert_eq!(row.color, None);
        assert!(!row.mentionable);
    }
}
