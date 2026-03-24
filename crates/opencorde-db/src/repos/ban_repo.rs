//! # Repository: Server Moderation
//! CRUD operations for server bans and member timeouts.
//!
//! Manages ban lists and temporary timeout restrictions for server members.
//!
//! ## Depends On
//! - chrono::DateTime<Utc> for timestamp handling
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Row type for reading server ban data.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BanRow {
    pub server_id: i64,
    pub user_id: i64,
    pub banned_by: i64,
    pub reason: Option<String>,
    pub banned_at: DateTime<Utc>,
}

/// Row type for reading server timeout data.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct TimeoutRow {
    pub server_id: i64,
    pub user_id: i64,
    pub timeout_until: DateTime<Utc>,
    pub reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Ban a user from a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64) to ban
/// * `banned_by` - User ID (i64) of the moderator
/// * `reason` - Optional ban reason
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn ban_user(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
    banned_by: i64,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        server_id = server_id,
        user_id = user_id,
        banned_by = banned_by,
        reason = ?reason,
        "banning user from server"
    );

    sqlx::query(
        "INSERT INTO server_bans (server_id, user_id, banned_by, reason) \
         VALUES ($1, $2, $3, $4) ON CONFLICT DO NOTHING",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(banned_by)
    .bind(reason)
    .execute(pool)
    .await?;

    tracing::info!(user_id = user_id, "user banned");
    Ok(())
}

/// Unban a user from a server.
///
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64) to unban
///
/// # Returns
/// Returns true if a ban existed and was removed, false if no ban existed.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn unban_user(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    tracing::info!(
        server_id = server_id,
        user_id = user_id,
        "unbanning user from server"
    );

    let result = sqlx::query("DELETE FROM server_bans WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    let exists = result.rows_affected() > 0;
    if exists {
        tracing::info!(user_id = user_id, "user unbanned");
    }
    Ok(exists)
}

/// Check if a user is banned from a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn is_banned(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let result: Option<(i64,)> = sqlx::query_as(
        "SELECT 1 FROM server_bans WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await?;

    Ok(result.is_some())
}

/// List all bans for a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_bans(pool: &PgPool, server_id: i64) -> Result<Vec<BanRow>, sqlx::Error> {
    tracing::info!(server_id = server_id, "listing server bans");

    sqlx::query_as::<_, BanRow>(
        "SELECT * FROM server_bans WHERE server_id = $1 ORDER BY banned_at DESC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await
}

/// Set a timeout for a user in a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64) to timeout
/// * `timeout_until` - DateTime when the timeout expires
/// * `reason` - Optional timeout reason
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn set_timeout(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
    timeout_until: DateTime<Utc>,
    reason: Option<&str>,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        server_id = server_id,
        user_id = user_id,
        timeout_until = ?timeout_until,
        reason = ?reason,
        "setting user timeout"
    );

    sqlx::query(
        "INSERT INTO member_timeouts (server_id, user_id, timeout_until, reason) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (server_id, user_id) \
         DO UPDATE SET timeout_until = EXCLUDED.timeout_until, reason = EXCLUDED.reason",
    )
    .bind(server_id)
    .bind(user_id)
    .bind(timeout_until)
    .bind(reason)
    .execute(pool)
    .await?;

    tracing::info!(user_id = user_id, "user timeout set");
    Ok(())
}

/// Remove a timeout for a user in a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64)
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn remove_timeout(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        server_id = server_id,
        user_id = user_id,
        "removing user timeout"
    );

    sqlx::query("DELETE FROM member_timeouts WHERE server_id = $1 AND user_id = $2")
        .bind(server_id)
        .bind(user_id)
        .execute(pool)
        .await?;

    tracing::info!(user_id = user_id, "user timeout removed");
    Ok(())
}

/// Get timeout information for a user in a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Server ID (i64)
/// * `user_id` - User ID (i64)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_timeout(
    pool: &PgPool,
    server_id: i64,
    user_id: i64,
) -> Result<Option<TimeoutRow>, sqlx::Error> {
    sqlx::query_as::<_, TimeoutRow>(
        "SELECT * FROM member_timeouts WHERE server_id = $1 AND user_id = $2",
    )
    .bind(server_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ban_row_creation() {
        let now = Utc::now();
        let row = BanRow {
            server_id: 111222333,
            user_id: 444555666,
            banned_by: 777888999,
            reason: Some("spam".to_string()),
            banned_at: now,
        };

        assert_eq!(row.server_id, 111222333);
        assert_eq!(row.user_id, 444555666);
        assert_eq!(row.banned_by, 777888999);
    }

    #[test]
    fn test_timeout_row_creation() {
        let now = Utc::now();
        let later = now + chrono::Duration::hours(1);
        let row = TimeoutRow {
            server_id: 111222333,
            user_id: 444555666,
            timeout_until: later,
            reason: Some("spam".to_string()),
            created_at: now,
        };

        assert_eq!(row.server_id, 111222333);
        assert_eq!(row.user_id, 444555666);
        assert!(row.timeout_until > row.created_at);
    }
}
