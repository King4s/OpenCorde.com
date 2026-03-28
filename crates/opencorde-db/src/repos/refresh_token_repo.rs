//! # Repository: Refresh Tokens
//! JTI-based refresh token storage for rotation and theft detection.
//!
//! ## Design
//! - Each issued refresh token has a unique JTI (UUID v4) stored here
//! - On refresh: look up JTI; if revoked → theft detected → revoke all user tokens
//! - On successful refresh: revoke old JTI, insert new JTI
//! - `cleanup_expired()` should be called periodically to prune old rows
//!
//! ## Depends On
//! - chrono::DateTime<Utc> for timestamp handling
//! - sqlx::PgPool for database queries

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Row type for a stored refresh token JTI.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct RefreshTokenRow {
    pub jti: String,
    pub user_id: i64,
    pub created_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub revoked: bool,
}

/// Store a newly-issued refresh token JTI.
///
/// # Arguments
/// * `pool` — Database connection pool
/// * `jti` — UUID v4 token identifier
/// * `user_id` — Owner's user ID
/// * `expires_at` — When the token expires (mirrors JWT `exp`)
///
/// # Errors
/// Returns `sqlx::Error` on insert failure.
#[tracing::instrument(skip(pool))]
pub async fn insert(
    pool: &PgPool,
    jti: &str,
    user_id: i64,
    expires_at: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO refresh_tokens (jti, user_id, expires_at) VALUES ($1, $2, $3)",
    )
    .bind(jti)
    .bind(user_id)
    .bind(expires_at)
    .execute(pool)
    .await?;
    tracing::debug!(jti = jti, user_id = user_id, "refresh token JTI stored");
    Ok(())
}

/// Fetch a refresh token record by JTI.
///
/// Returns `None` if the JTI has never been issued (or was deleted by cleanup).
///
/// # Errors
/// Returns `sqlx::Error` on query failure.
#[tracing::instrument(skip(pool))]
pub async fn get_by_jti(
    pool: &PgPool,
    jti: &str,
) -> Result<Option<RefreshTokenRow>, sqlx::Error> {
    sqlx::query_as::<_, RefreshTokenRow>(
        "SELECT * FROM refresh_tokens WHERE jti = $1",
    )
    .bind(jti)
    .fetch_optional(pool)
    .await
}

/// Mark a single refresh token as revoked (normal rotation — old token used, new issued).
///
/// # Errors
/// Returns `sqlx::Error` on update failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke(pool: &PgPool, jti: &str) -> Result<(), sqlx::Error> {
    sqlx::query("UPDATE refresh_tokens SET revoked = TRUE WHERE jti = $1")
        .bind(jti)
        .execute(pool)
        .await?;
    tracing::debug!(jti = jti, "refresh token JTI revoked");
    Ok(())
}

/// Revoke ALL refresh tokens for a user (theft response).
///
/// Called when a revoked JTI is presented — someone is replaying a
/// previously-rotated token. Invalidates every session for the user.
///
/// # Errors
/// Returns `sqlx::Error` on update failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke_all_for_user(pool: &PgPool, user_id: i64) -> Result<u64, sqlx::Error> {
    let result = sqlx::query(
        "UPDATE refresh_tokens SET revoked = TRUE WHERE user_id = $1 AND revoked = FALSE",
    )
    .bind(user_id)
    .execute(pool)
    .await?;
    let count = result.rows_affected();
    tracing::warn!(
        user_id = user_id,
        tokens_revoked = count,
        "all refresh tokens revoked for user (theft response)"
    );
    Ok(count)
}

/// Delete all expired refresh token rows.
///
/// Safe to call on a schedule (e.g., hourly). Returns the number of rows deleted.
///
/// # Errors
/// Returns `sqlx::Error` on delete failure.
#[tracing::instrument(skip(pool))]
pub async fn cleanup_expired(pool: &PgPool) -> Result<u64, sqlx::Error> {
    let result =
        sqlx::query("DELETE FROM refresh_tokens WHERE expires_at < NOW()")
            .execute(pool)
            .await?;
    let count = result.rows_affected();
    tracing::info!(deleted = count, "expired refresh tokens cleaned up");
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn test_refresh_token_row_fields() {
        let now = Utc::now();
        let row = RefreshTokenRow {
            jti: "test-jti".to_string(),
            user_id: 123,
            created_at: now,
            expires_at: now + Duration::days(7),
            revoked: false,
        };
        assert_eq!(row.jti, "test-jti");
        assert_eq!(row.user_id, 123);
        assert!(!row.revoked);
        assert!(row.expires_at > row.created_at);
    }

    #[test]
    fn test_revoked_flag() {
        let now = Utc::now();
        let row = RefreshTokenRow {
            jti: "revoked-jti".to_string(),
            user_id: 456,
            created_at: now,
            expires_at: now + Duration::days(7),
            revoked: true,
        };
        assert!(row.revoked);
    }
}
