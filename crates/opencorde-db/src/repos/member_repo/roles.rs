//! # Member Roles Operations
//! Assign and remove roles for server members.

use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading member role assignments.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberRoleRow {
    pub user_id: i64,
    pub server_id: i64,
    pub role_id: i64,
}

/// Assign a role to a member in a server.
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate assignment).
#[tracing::instrument(skip(pool))]
pub async fn add_role(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
    role_id: Snowflake,
) -> Result<MemberRoleRow, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        role_id = role_id.as_i64(),
        "assigning role to member"
    );

    let row = sqlx::query_as::<_, MemberRoleRow>(
        "INSERT INTO member_roles (user_id, server_id, role_id) \
         VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(user_id.as_i64())
    .bind(server_id.as_i64())
    .bind(role_id.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(role_id = row.role_id, "role assigned");
    Ok(row)
}

/// Remove a role from a member in a server.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn remove_role(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
    role_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        role_id = role_id.as_i64(),
        "removing role from member"
    );

    sqlx::query("DELETE FROM member_roles WHERE user_id = $1 AND server_id = $2 AND role_id = $3")
        .bind(user_id.as_i64())
        .bind(server_id.as_i64())
        .bind(role_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// List all roles assigned to a member in a server.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_member_roles(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<Vec<MemberRoleRow>, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        "listing member roles"
    );

    sqlx::query_as::<_, MemberRoleRow>(
        "SELECT * FROM member_roles WHERE user_id = $1 AND server_id = $2 ORDER BY role_id ASC",
    )
    .bind(user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}
