//! # Repository: Relationships
//! Friend requests, friendships, and block operations.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading relationships from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RelationshipRow {
    pub id: i64,
    pub from_user: i64,
    pub to_user: i64,
    pub status: String, // "pending", "accepted", "blocked"
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    // Joined fields
    pub other_username: String,
    pub other_avatar_url: Option<String>,
}

/// Send a friend request to a user.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the relationship
/// * `from_user` - Snowflake ID of the user sending the request
/// * `to_user` - Snowflake ID of the recipient user
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate or constraint violation).
#[tracing::instrument(skip(pool))]
pub async fn send_request(
    pool: &PgPool,
    id: Snowflake,
    from_user: Snowflake,
    to_user: Snowflake,
) -> Result<RelationshipRow, sqlx::Error> {
    tracing::info!(
        from_user = %from_user.as_i64(),
        to_user = %to_user.as_i64(),
        "sending friend request"
    );

    let row = sqlx::query_as::<_, RelationshipRow>(
        "WITH inserted AS (
            INSERT INTO relationships (id, from_user, to_user, status)
            VALUES ($1, $2, $3, 'pending')
            RETURNING *
        )
        SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
               u.username as other_username, u.avatar_url as other_avatar_url
        FROM inserted r
        JOIN users u ON r.to_user = u.id",
    )
    .bind(id.as_i64())
    .bind(from_user.as_i64())
    .bind(to_user.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(relationship_id = row.id, "friend request sent");
    Ok(row)
}

/// Get relationship between two users (in either direction).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_a` - First user snowflake
/// * `user_b` - Second user snowflake
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_between(
    pool: &PgPool,
    user_a: Snowflake,
    user_b: Snowflake,
) -> Result<Option<RelationshipRow>, sqlx::Error> {
    let user_a_i64 = user_a.as_i64();
    let user_b_i64 = user_b.as_i64();

    sqlx::query_as::<_, RelationshipRow>(
        "SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
                CASE WHEN r.from_user=$1 THEN tu.username ELSE fu.username END as other_username,
                CASE WHEN r.from_user=$1 THEN tu.avatar_url ELSE fu.avatar_url END as other_avatar_url
        FROM relationships r
        JOIN users fu ON r.from_user = fu.id
        JOIN users tu ON r.to_user = tu.id
        WHERE (r.from_user=$1 AND r.to_user=$2) OR (r.from_user=$2 AND r.to_user=$1)",
    )
    .bind(user_a_i64)
    .bind(user_b_i64)
    .bind(user_a_i64)
    .bind(user_b_i64)
    .fetch_optional(pool)
    .await
}

/// Accept a pending friend request.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Relationship ID to accept
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn accept_request(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(relationship_id = %id.as_i64(), "accepting friend request");

    sqlx::query("UPDATE relationships SET status='accepted', updated_at=NOW() WHERE id=$1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete a relationship (friend or pending request).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Relationship ID to delete
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_relationship(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(relationship_id = %id.as_i64(), "deleting relationship");

    sqlx::query("DELETE FROM relationships WHERE id=$1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Block a user.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the relationship
/// * `from_user` - Snowflake ID of the user doing the blocking
/// * `to_user` - Snowflake ID of the user being blocked
///
/// # Errors
/// Returns sqlx::Error if the operation fails.
#[tracing::instrument(skip(pool))]
pub async fn block_user(
    pool: &PgPool,
    id: Snowflake,
    from_user: Snowflake,
    to_user: Snowflake,
) -> Result<RelationshipRow, sqlx::Error> {
    tracing::info!(
        from_user = %from_user.as_i64(),
        to_user = %to_user.as_i64(),
        "blocking user"
    );

    let row = sqlx::query_as::<_, RelationshipRow>(
        "WITH upserted AS (
            INSERT INTO relationships (id, from_user, to_user, status)
            VALUES ($1, $2, $3, 'blocked')
            ON CONFLICT (from_user, to_user)
            DO UPDATE SET status='blocked', updated_at=NOW()
            RETURNING *
        )
        SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
               u.username as other_username, u.avatar_url as other_avatar_url
        FROM upserted r
        JOIN users u ON r.to_user = u.id",
    )
    .bind(id.as_i64())
    .bind(from_user.as_i64())
    .bind(to_user.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(relationship_id = row.id, "user blocked");
    Ok(row)
}

/// List all friends of a user (accepted relationships).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - User's snowflake ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_friends(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<RelationshipRow>, sqlx::Error> {
    let user_id_i64 = user_id.as_i64();

    sqlx::query_as::<_, RelationshipRow>(
        "SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
                CASE WHEN r.from_user=$1 THEN tu.username ELSE fu.username END as other_username,
                CASE WHEN r.from_user=$1 THEN tu.avatar_url ELSE fu.avatar_url END as other_avatar_url
        FROM relationships r
        JOIN users fu ON r.from_user = fu.id
        JOIN users tu ON r.to_user = tu.id
        WHERE r.status='accepted' AND (r.from_user=$1 OR r.to_user=$1)
        ORDER BY r.updated_at DESC",
    )
    .bind(user_id_i64)
    .fetch_all(pool)
    .await
}

/// List incoming pending friend requests.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - User's snowflake ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_pending_incoming(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<RelationshipRow>, sqlx::Error> {
    sqlx::query_as::<_, RelationshipRow>(
        "SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
                u.username as other_username, u.avatar_url as other_avatar_url
        FROM relationships r
        JOIN users u ON r.from_user = u.id
        WHERE r.to_user=$1 AND r.status='pending'
        ORDER BY r.created_at DESC",
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await
}

/// List outgoing pending friend requests.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - User's snowflake ID
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_pending_outgoing(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<RelationshipRow>, sqlx::Error> {
    sqlx::query_as::<_, RelationshipRow>(
        "SELECT r.id, r.from_user, r.to_user, r.status::text, r.created_at, r.updated_at,
                u.username as other_username, u.avatar_url as other_avatar_url
        FROM relationships r
        JOIN users u ON r.to_user = u.id
        WHERE r.from_user=$1 AND r.status='pending'
        ORDER BY r.created_at DESC",
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await
}
