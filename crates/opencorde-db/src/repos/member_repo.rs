//! # Repository: Server Members
//! CRUD operations for server membership and member roles.
//!
//! Manages the join relationship between users and servers, plus role assignments.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading server member data.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberRow {
    pub user_id: i64,
    pub server_id: i64,
    pub nickname: Option<String>,
    pub joined_at: DateTime<Utc>,
}

/// Row type for reading member role assignments.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberRoleRow {
    pub user_id: i64,
    pub server_id: i64,
    pub role_id: i64,
}

/// Row type for reading server member data with username joined from users.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MemberWithUsernameRow {
    pub user_id: i64,
    pub server_id: i64,
    pub username: String,
    pub nickname: Option<String>,
    pub joined_at: DateTime<Utc>,
}

/// List all members in a server with their usernames, ordered by join date.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_with_usernames_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<MemberWithUsernameRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing server members with usernames");

    sqlx::query_as::<_, MemberWithUsernameRow>(
        "SELECT m.user_id, m.server_id, u.username, m.nickname, m.joined_at \
         FROM server_members m \
         JOIN users u ON m.user_id = u.id \
         WHERE m.server_id = $1 ORDER BY m.joined_at ASC",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Add a user to a server as a member.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake ID of the user
/// * `server_id` - Snowflake ID of the server
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate membership).
#[tracing::instrument(skip(pool))]
pub async fn add_member(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<MemberRow, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        "adding member to server"
    );

    let row = sqlx::query_as::<_, MemberRow>(
        "INSERT INTO server_members (user_id, server_id) \
         VALUES ($1, $2) RETURNING *",
    )
    .bind(user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = row.user_id,
        server_id = row.server_id,
        "member added"
    );
    Ok(row)
}

/// Remove a user from a server.
///
/// Cascades to remove all role assignments for that member.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn remove_member(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        "removing member from server"
    );

    sqlx::query("DELETE FROM server_members WHERE user_id = $1 AND server_id = $2")
        .bind(user_id.as_i64())
        .bind(server_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Get membership info for a user in a server.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_member(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
) -> Result<Option<MemberRow>, sqlx::Error> {
    sqlx::query_as::<_, MemberRow>(
        "SELECT * FROM server_members WHERE user_id = $1 AND server_id = $2",
    )
    .bind(user_id.as_i64())
    .bind(server_id.as_i64())
    .fetch_optional(pool)
    .await
}

/// List all members in a server, ordered by join date.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<MemberRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing server members");

    sqlx::query_as::<_, MemberRow>(
        "SELECT * FROM server_members WHERE server_id = $1 ORDER BY joined_at ASC",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update a member's nickname in a server.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_nickname(
    pool: &PgPool,
    user_id: Snowflake,
    server_id: Snowflake,
    nickname: Option<&str>,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        server_id = server_id.as_i64(),
        nickname = ?nickname,
        "updating member nickname"
    );

    sqlx::query("UPDATE server_members SET nickname = $1 WHERE user_id = $2 AND server_id = $3")
        .bind(nickname)
        .bind(user_id.as_i64())
        .bind(server_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_member_row_creation() {
        let now = Utc::now();
        let row = MemberRow {
            user_id: 111222333,
            server_id: 555666777,
            nickname: Some("TestNick".to_string()),
            joined_at: now,
        };

        assert_eq!(row.user_id, 111222333);
        assert_eq!(row.server_id, 555666777);
        assert_eq!(row.nickname, Some("TestNick".to_string()));
    }

    #[test]
    fn test_member_role_row_creation() {
        let row = MemberRoleRow {
            user_id: 111222333,
            server_id: 555666777,
            role_id: 999888777,
        };

        assert_eq!(row.user_id, 111222333);
        assert_eq!(row.role_id, 999888777);
    }
}
