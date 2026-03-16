//! # Repository: Server Invites
//! CRUD operations for server invites.
//!
//! Manages invite codes and tracking of usage.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading invite data from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InviteRow {
    pub code: String,
    pub server_id: i64,
    pub creator_id: i64,
    pub uses: i32,
    pub max_uses: Option<i32>,
    pub expires_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create a new invite.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `code` - Invite code (unique)
/// * `server_id` - Snowflake ID of the server
/// * `creator_id` - Snowflake ID of the invite creator
/// * `max_uses` - Maximum uses (None = unlimited)
/// * `expires_at` - Expiration time (None = never expires)
///
/// # Errors
/// Returns sqlx::Error if the insert fails (e.g., duplicate code).
#[tracing::instrument(skip(pool))]
pub async fn create_invite(
    pool: &PgPool,
    code: &str,
    server_id: Snowflake,
    creator_id: Snowflake,
    max_uses: Option<i32>,
    expires_at: Option<DateTime<Utc>>,
) -> Result<InviteRow, sqlx::Error> {
    tracing::info!(
        code = %code,
        server_id = server_id.as_i64(),
        max_uses = ?max_uses,
        "creating invite"
    );

    let row = sqlx::query_as::<_, InviteRow>(
        "INSERT INTO invites (code, server_id, creator_id, max_uses, expires_at) \
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(code)
    .bind(server_id.as_i64())
    .bind(creator_id.as_i64())
    .bind(max_uses)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;

    tracing::info!(code = %code, "invite created");
    Ok(row)
}

/// Get an invite by its code.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_code(pool: &PgPool, code: &str) -> Result<Option<InviteRow>, sqlx::Error> {
    sqlx::query_as::<_, InviteRow>("SELECT * FROM invites WHERE code = $1")
        .bind(code)
        .fetch_optional(pool)
        .await
}

/// Increment the usage count for an invite.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn increment_uses(pool: &PgPool, code: &str) -> Result<(), sqlx::Error> {
    tracing::debug!(code = %code, "incrementing invite uses");

    sqlx::query("UPDATE invites SET uses = uses + 1 WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete an invite by its code.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_invite(pool: &PgPool, code: &str) -> Result<(), sqlx::Error> {
    tracing::info!(code = %code, "deleting invite");

    sqlx::query("DELETE FROM invites WHERE code = $1")
        .bind(code)
        .execute(pool)
        .await?;

    Ok(())
}

/// List all invites for a server.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<InviteRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing server invites");

    sqlx::query_as::<_, InviteRow>(
        "SELECT * FROM invites WHERE server_id = $1 ORDER BY created_at DESC",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invite_row_creation() {
        let now = Utc::now();
        let row = InviteRow {
            code: "abc12345".to_string(),
            server_id: 111222333,
            creator_id: 999888777,
            uses: 0,
            max_uses: Some(10),
            expires_at: Some(now),
            created_at: now,
        };

        assert_eq!(row.code, "abc12345");
        assert_eq!(row.server_id, 111222333);
        assert_eq!(row.uses, 0);
        assert_eq!(row.max_uses, Some(10));
    }

    #[test]
    fn test_invite_row_unlimited() {
        let now = Utc::now();
        let row = InviteRow {
            code: "unlimited".to_string(),
            server_id: 111222333,
            creator_id: 999888777,
            uses: 5,
            max_uses: None,
            expires_at: None,
            created_at: now,
        };

        assert_eq!(row.uses, 5);
        assert_eq!(row.max_uses, None);
        assert_eq!(row.expires_at, None);
    }
}
