//! # Member CRUD Operations
//! Add, remove, and list server members.

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
