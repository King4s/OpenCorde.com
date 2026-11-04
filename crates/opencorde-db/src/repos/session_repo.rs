//! # Repository: User Sessions
//! Session tracking for the Sessions/devices settings page.
//!
//! ## Design
//! - Each login or token refresh creates/updates a session row.
//! - Sessions can be revoked individually or all-at-once (except the current one).
//! - The `current_session_id` is embedded in JWT claims so the API knows
//!   which session is "the current one" when revoking all others.
//!
//! ## Depends On
//! - chrono::DateTime<Utc> for timestamp handling
//! - sqlx::PgPool for database queries

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Row type for a stored user session.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserSessionRow {
    pub id: uuid::Uuid,
    pub user_id: i64,
    pub device_name: Option<String>,
    pub ip_address: Option<sqlx::types::ipnetwork::IpNetwork>,
    pub created_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
    pub revoked: bool,
}

/// Insert or update a session row for a user.
///
/// Uses ON CONFLICT (id) to upsert: on first login, insert; on refresh,
/// update `last_active_at`, `device_name`, and `ip_address`.
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `id` — UUID session identifier
/// * `user_id` — Owner's user ID
/// * `device_name` — Parsed from User-Agent header
/// * `ip_address` — Client IP address (as IpNetwork string)
///
/// # Errors
/// Returns `sqlx::Error` on upsert failure.
#[tracing::instrument(skip(pool))]
pub async fn upsert(
    pool: &PgPool,
    id: uuid::Uuid,
    user_id: i64,
    device_name: Option<&str>,
    ip_address: Option<&str>,
) -> Result<(), sqlx::Error> {
    let ip: Option<sqlx::types::ipnetwork::IpNetwork> = match ip_address {
        Some(addr) => addr.parse().ok(),
        None => None,
    };

    sqlx::query(
        "INSERT INTO user_sessions (id, user_id, device_name, ip_address, last_active_at) \
         VALUES ($1, $2, $3, $4, NOW()) \
         ON CONFLICT (id) DO UPDATE SET \
             device_name = COALESCE(EXCLUDED.device_name, user_sessions.device_name), \
             ip_address = COALESCE(EXCLUDED.ip_address, user_sessions.ip_address), \
             last_active_at = NOW()"
    )
    .bind(id)
    .bind(user_id)
    .bind(device_name)
    .bind(ip)
    .execute(pool)
    .await?;

    tracing::debug!(session_id = %id, user_id = user_id, "session upserted");
    Ok(())
}

/// Update `last_active_at` for a session (token refresh path).
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `id` — UUID session identifier
///
/// # Errors
/// Returns `sqlx::Error` on update failure.
#[tracing::instrument(skip(pool))]
pub async fn touch(pool: &PgPool, id: uuid::Uuid) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE user_sessions SET last_active_at = NOW() WHERE id = $1 AND revoked = FALSE")
        .bind(id)
        .execute(pool)
        .await?;
    tracing::debug!(session_id = %id, "session last_active_at touched");
    Ok(())
}

/// List all non-revoked sessions for a user, newest first.
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `user_id` — The user's ID
///
/// # Errors
/// Returns `sqlx::Error` on query failure.
#[tracing::instrument(skip(pool))]
pub async fn list_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<UserSessionRow>, sqlx::Error> {
    sqlx::query_as::<_, UserSessionRow>(
        "SELECT * FROM user_sessions WHERE user_id = $1 AND revoked = FALSE ORDER BY last_active_at DESC"
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Revoke a single session by ID.
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `session_id` — UUID of the session to revoke
/// * `user_id` — Owner's user ID (ensures user can only revoke their own sessions)
///
/// # Returns
/// `true` if a matching non-revoked session was found and revoked, `false` if none matched.
///
/// # Errors
/// Returns `sqlx::Error` on update failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke_one(
    pool: &PgPool,
    session_id: uuid::Uuid,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE user_sessions SET revoked = TRUE WHERE id = $1 AND user_id = $2 AND revoked = FALSE"
    )
    .bind(session_id)
    .bind(user_id)
    .execute(pool)
    .await?;

    let revoked = result.rows_affected() > 0;
    if revoked {
        tracing::info!(session_id = %session_id, user_id = user_id, "session revoked");
    }
    Ok(revoked)
}

/// Revoke all sessions for a user EXCEPT the current one.
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `user_id` — The user's ID
/// * `current_session_id` — The session to keep active
///
/// # Returns
/// Number of sessions revoked.
///
/// # Errors
/// Returns `sqlx::Error` on update failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke_all_except(
    pool: &PgPool,
    user_id: i64,
    current_session_id: uuid::Uuid,
) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE user_sessions SET revoked = TRUE WHERE user_id = $1 AND id != $2 AND revoked = FALSE"
    )
    .bind(user_id)
    .bind(current_session_id)
    .execute(pool)
    .await?;

    let count = result.rows_affected();
    tracing::info!(user_id = user_id, revoked_count = count, "all sessions except current revoked");
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_session_row_fields() {
        let now = Utc::now();
        let id = uuid::Uuid::new_v4();
        let row = UserSessionRow {
            id,
            user_id: 123,
            device_name: Some("Chrome on Linux".to_string()),
            ip_address: None,
            created_at: now,
            last_active_at: now,
            revoked: false,
        };
        assert_eq!(row.user_id, 123);
        assert_eq!(row.device_name, Some("Chrome on Linux".to_string()));
        assert!(!row.revoked);
    }

    #[test]
    fn test_revoked_flag() {
        let now = Utc::now();
        let row = UserSessionRow {
            id: uuid::Uuid::new_v4(),
            user_id: 456,
            device_name: None,
            ip_address: None,
            created_at: now,
            last_active_at: now,
            revoked: true,
        };
        assert!(row.revoked);
    }
}
