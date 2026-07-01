//! # Repository: User OAuth Authorizations
//! Persistence layer for user-to-application OAuth authorization grants.
//!
//! ## Operations
//! - `upsert` — record or update an authorization grant (idempotent on user+app)
//! - `list_for_user` — list all authorizations for a user with application name
//! - `revoke` — delete a specific authorization
//!
//! ## Depends On
//! - sqlx::PgPool
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row returned by list_for_user — authorization joined with application name.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AuthorizedAppRow {
    pub id: i64,
    pub user_id: i64,
    pub application_id: i64,
    pub scope: String,
    pub authorized_at: DateTime<Utc>,
    pub app_name: String,
    pub app_description: Option<String>,
    pub app_icon_url: Option<String>,
}

/// Insert or update an authorization grant for a user+application pair.
///
/// Idempotent — if the user already authorized this app, the scope and
/// authorized_at are updated. Returns the row ID.
///
/// # Errors
/// Returns `sqlx::Error` on insert/update failure.
#[tracing::instrument(skip(pool))]
pub async fn upsert(
    pool: &PgPool,
    id: Snowflake,
    user_id: Snowflake,
    application_id: Snowflake,
    scope: &str,
) -> Result<i64, sqlx::Error> {
    let row = sqlx::query_as::<_, (i64,)>(
        "INSERT INTO user_oauth_authorizations (id, user_id, application_id, scope)
         VALUES ($1, $2, $3, $4)
         ON CONFLICT (user_id, application_id)
         DO UPDATE SET scope = EXCLUDED.scope, authorized_at = NOW()
         RETURNING id",
    )
    .bind(id.as_i64())
    .bind(user_id.as_i64())
    .bind(application_id.as_i64())
    .bind(scope)
    .fetch_one(pool)
    .await?;

    tracing::info!(
        user_id = %user_id,
        application_id = %application_id,
        scope = scope,
        "OAuth authorization recorded"
    );
    Ok(row.0)
}

/// List all authorizations for a user, joined with application details.
///
/// Returns rows ordered by most recent authorization first.
///
/// # Errors
/// Returns `sqlx::Error` on query failure.
#[tracing::instrument(skip(pool))]
pub async fn list_for_user(
    pool: &PgPool,
    user_id: i64,
) -> Result<Vec<AuthorizedAppRow>, sqlx::Error> {
    sqlx::query_as::<_, AuthorizedAppRow>(
        "SELECT ua.id, ua.user_id, ua.application_id, ua.scope, ua.authorized_at,
                a.name AS app_name, a.description AS app_description, a.icon_url AS app_icon_url
         FROM user_oauth_authorizations ua
         JOIN applications a ON a.id = ua.application_id
         WHERE ua.user_id = $1
         ORDER BY ua.authorized_at DESC",
    )
    .bind(user_id)
    .fetch_all(pool)
    .await
}

/// Revoke an authorization by its ID, scoped to a specific user.
///
/// Returns `true` if a row was deleted, `false` if no matching authorization existed.
///
/// # Errors
/// Returns `sqlx::Error` on delete failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke(
    pool: &PgPool,
    id: i64,
    user_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM user_oauth_authorizations WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(user_id)
    .execute(pool)
    .await?;

    let deleted = result.rows_affected() > 0;
    if deleted {
        tracing::info!(auth_id = id, user_id = user_id, "OAuth authorization revoked");
    }
    Ok(deleted)
}

/// Revoke an authorization by application ID for a user.
///
/// Returns `true` if a row was deleted, `false` if none existed.
///
/// # Errors
/// Returns `sqlx::Error` on delete failure.
#[tracing::instrument(skip(pool))]
pub async fn revoke_by_app(
    pool: &PgPool,
    user_id: i64,
    application_id: i64,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM user_oauth_authorizations WHERE user_id = $1 AND application_id = $2",
    )
    .bind(user_id)
    .bind(application_id)
    .execute(pool)
    .await?;

    let deleted = result.rows_affected() > 0;
    if deleted {
        tracing::info!(
            user_id = user_id,
            application_id = application_id,
            "OAuth authorization revoked by app"
        );
    }
    Ok(deleted)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_authorized_app_row_fields() {
        let now = Utc::now();
        let row = AuthorizedAppRow {
            id: 1000,
            user_id: 200,
            application_id: 300,
            scope: "identify".to_string(),
            authorized_at: now,
            app_name: "TestApp".to_string(),
            app_description: Some("A test app".to_string()),
            app_icon_url: None,
        };

        assert_eq!(row.id, 1000);
        assert_eq!(row.app_name, "TestApp");
        assert_eq!(row.scope, "identify");
    }
}
