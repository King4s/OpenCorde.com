//! # Repository: User Activities
//! CRUD operations for user activity records.
//!
//! Manages rich activity presence — playing, listening, streaming, custom —
//! with details, state, time tracking, and optional asset URL.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading user activities from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct UserActivityRow {
    pub id: i64,
    pub user_id: i64,
    pub activity_type: String,
    pub name: String,
    pub details: Option<String>,
    pub state: Option<String>,
    pub started_at: DateTime<Utc>,
    pub ends_at: Option<DateTime<Utc>>,
    pub asset_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

/// Create a new activity record for a user.
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn create_activity(
    pool: &PgPool,
    id: Snowflake,
    user_id: Snowflake,
    activity_type: &str,
    name: &str,
    details: Option<&str>,
    state: Option<&str>,
    ends_at: Option<DateTime<Utc>>,
    asset_url: Option<&str>,
) -> Result<UserActivityRow, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        activity_type = %activity_type,
        name = %name,
        "creating user activity"
    );

    let row = sqlx::query_as::<_, UserActivityRow>(
        "INSERT INTO user_activities (id, user_id, activity_type, name, details, state, started_at, ends_at, asset_url) \
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7, $8) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(user_id.as_i64())
    .bind(activity_type)
    .bind(name)
    .bind(details)
    .bind(state)
    .bind(ends_at)
    .bind(asset_url)
    .fetch_one(pool)
    .await?;

    tracing::info!(activity_id = row.id, "activity created successfully");
    Ok(row)
}

/// Get all activities for a user (current + recent), newest first.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_user(
    pool: &PgPool,
    user_id: Snowflake,
    limit: i64,
) -> Result<Vec<UserActivityRow>, sqlx::Error> {
    sqlx::query_as::<_, UserActivityRow>(
        "SELECT * FROM user_activities \
         WHERE user_id = $1 \
         ORDER BY started_at DESC \
         LIMIT $2",
    )
    .bind(user_id.as_i64())
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get the current active activity for a user (the most recent one that hasn't ended).
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_current_activity(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Option<UserActivityRow>, sqlx::Error> {
    sqlx::query_as::<_, UserActivityRow>(
        "SELECT * FROM user_activities \
         WHERE user_id = $1 \
           AND (ends_at IS NULL OR ends_at > NOW()) \
         ORDER BY started_at DESC \
         LIMIT 1",
    )
    .bind(user_id.as_i64())
    .fetch_optional(pool)
    .await
}

/// Delete a specific activity by ID (must belong to the user).
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_activity(
    pool: &PgPool,
    id: Snowflake,
    user_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    tracing::info!(
        activity_id = id.as_i64(),
        user_id = user_id.as_i64(),
        "deleting user activity"
    );

    let result = sqlx::query(
        "DELETE FROM user_activities WHERE id = $1 AND user_id = $2",
    )
    .bind(id.as_i64())
    .bind(user_id.as_i64())
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Clear all activities for a user.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn clear_all_activities(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<u64, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        "clearing all user activities"
    );

    let result = sqlx::query("DELETE FROM user_activities WHERE user_id = $1")
        .bind(user_id.as_i64())
        .execute(pool)
        .await?;

    Ok(result.rows_affected())
}

/// Update an activity's state (e.g., "In Menu" → "Playing").
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_activity_state(
    pool: &PgPool,
    id: Snowflake,
    user_id: Snowflake,
    state: &str,
) -> Result<Option<UserActivityRow>, sqlx::Error> {
    tracing::info!(
        activity_id = id.as_i64(),
        user_id = user_id.as_i64(),
        state = %state,
        "updating activity state"
    );

    sqlx::query_as::<_, UserActivityRow>(
        "UPDATE user_activities SET state = $1 \
         WHERE id = $2 AND user_id = $3 \
         RETURNING *",
    )
    .bind(state)
    .bind(id.as_i64())
    .bind(user_id.as_i64())
    .fetch_optional(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_activity_row_creation() {
        let now = Utc::now();
        let row = UserActivityRow {
            id: 1001,
            user_id: 42,
            activity_type: "playing".to_string(),
            name: "Rust".to_string(),
            details: Some("Competitive - 3v3".to_string()),
            state: Some("In Menu".to_string()),
            started_at: now,
            ends_at: None,
            asset_url: Some("https://example.com/rust.png".to_string()),
            created_at: now,
        };

        assert_eq!(row.id, 1001);
        assert_eq!(row.user_id, 42);
        assert_eq!(row.activity_type, "playing");
        assert_eq!(row.name, "Rust");
        assert_eq!(row.details.as_deref(), Some("Competitive - 3v3"));
        assert_eq!(row.state.as_deref(), Some("In Menu"));
        assert_eq!(row.asset_url.as_deref(), Some("https://example.com/rust.png"));
    }

    #[test]
    fn test_minimal_activity_row() {
        let now = Utc::now();
        let row = UserActivityRow {
            id: 2002,
            user_id: 99,
            activity_type: "custom".to_string(),
            name: "Coding".to_string(),
            details: None,
            state: None,
            started_at: now,
            ends_at: None,
            asset_url: None,
            created_at: now,
        };

        assert_eq!(row.activity_type, "custom");
        assert!(row.details.is_none());
        assert!(row.state.is_none());
    }
}
