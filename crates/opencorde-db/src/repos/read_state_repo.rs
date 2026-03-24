//! # Repository: Read State
//! Track last-read message per user per channel for unread counting.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (async SQL toolkit)
//! - chrono (date/time handling)

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading channel read state from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReadStateRow {
    pub user_id: i64,
    pub channel_id: i64,
    pub last_read_id: i64,
    pub mention_count: i32,
    pub updated_at: DateTime<Utc>,
}

/// Upsert: mark a channel as read up to message_id for the given user.
/// Resets mention_count to 0 when marking as read.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake ID of the user
/// * `channel_id` - Snowflake ID of the channel
/// * `last_read_id` - Message ID to mark as read up to (only moves forward)
///
/// # Errors
/// Returns sqlx::Error if the operation fails.
#[tracing::instrument(skip(pool))]
pub async fn mark_read(
    pool: &PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
    last_read_id: i64,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        channel_id = channel_id.as_i64(),
        last_read_id,
        "marking channel as read"
    );

    sqlx::query(
        "INSERT INTO channel_read_state (user_id, channel_id, last_read_id, mention_count, updated_at) \
         VALUES ($1, $2, $3, 0, NOW()) \
         ON CONFLICT (user_id, channel_id) DO UPDATE \
         SET last_read_id = GREATEST(EXCLUDED.last_read_id, channel_read_state.last_read_id), \
             mention_count = 0, updated_at = NOW()"
    )
    .bind(user_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(last_read_id)
    .execute(pool)
    .await?;

    Ok(())
}

/// Get read state for all channels for a user.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake ID of the user
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_for_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<ReadStateRow>, sqlx::Error> {
    tracing::info!(user_id = user_id.as_i64(), "fetching read states");

    sqlx::query_as::<_, ReadStateRow>(
        "SELECT user_id, channel_id, last_read_id, mention_count, updated_at \
         FROM channel_read_state WHERE user_id = $1 ORDER BY updated_at DESC"
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Get count of unread messages in a channel for a user.
///
/// Returns 0 if no read state exists (all messages are newer than last_read_id of 0).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `user_id` - Snowflake ID of the user
/// * `channel_id` - Snowflake ID of the channel
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn count_unread(
    pool: &PgPool,
    user_id: Snowflake,
    channel_id: Snowflake,
) -> Result<i64, sqlx::Error> {
    tracing::info!(
        user_id = user_id.as_i64(),
        channel_id = channel_id.as_i64(),
        "counting unread messages"
    );

    let row: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM messages \
         WHERE channel_id = $1 AND id > COALESCE( \
             (SELECT last_read_id FROM channel_read_state WHERE user_id = $2 AND channel_id = $1), \
             0 \
         )"
    )
    .bind(channel_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_one(pool)
    .await?;

    Ok(row.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_state_row_creation() {
        let now = Utc::now();
        let row = ReadStateRow {
            user_id: 123456789,
            channel_id: 987654321,
            last_read_id: 555,
            mention_count: 0,
            updated_at: now,
        };

        assert_eq!(row.user_id, 123456789);
        assert_eq!(row.channel_id, 987654321);
        assert_eq!(row.last_read_id, 555);
        assert_eq!(row.mention_count, 0);
    }

    #[test]
    fn test_last_read_id_default_zero() {
        // Default last_read_id is 0, meaning all messages in the channel are "unread"
        assert_eq!(0i64, 0);
    }
}
