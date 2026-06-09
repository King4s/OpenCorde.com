//! # Repository: DM Channel Unreads
//! Per-user unread message tracking for DM channels.
//!
//! When a message is sent in a DM channel, the recipient's unread count
//! is incremented (or created if missing). When the user marks the DM
//! as read, the row is deleted.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (async database access)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for DM channel unread entries.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannelUnreadRow {
    pub id: i64,
    pub user_id: i64,
    pub dm_channel_id: i64,
    pub unread_count: i32,
    pub last_message_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Increment the unread count for a user in a DM channel.
///
/// Creates the row if it doesn't exist, otherwise increments `unread_count`
/// and updates `last_message_id` and `updated_at`.
#[tracing::instrument(skip(pool))]
pub async fn increment_unread(
    pool: &PgPool,
    user_id: Snowflake,
    dm_channel_id: Snowflake,
    last_message_id: Snowflake,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO dm_channel_unreads (user_id, dm_channel_id, unread_count, last_message_id) \
         VALUES ($1, $2, 1, $3) \
         ON CONFLICT (user_id, dm_channel_id) \
         DO UPDATE SET \
             unread_count = dm_channel_unreads.unread_count + 1, \
             last_message_id = EXCLUDED.last_message_id, \
             updated_at = NOW()",
    )
    .bind(user_id.as_i64())
    .bind(dm_channel_id.as_i64())
    .bind(last_message_id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Clear all unreads for a user in a DM channel.
///
/// Deletes the unread row for the given user and DM channel.
#[tracing::instrument(skip(pool))]
pub async fn clear_unread(
    pool: &PgPool,
    user_id: Snowflake,
    dm_channel_id: Snowflake,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM dm_channel_unreads WHERE user_id = $1 AND dm_channel_id = $2")
        .bind(user_id.as_i64())
        .bind(dm_channel_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Get unread counts for all DM channels of a user.
///
/// Returns a vector of `(dm_channel_id, unread_count)` tuples.
#[tracing::instrument(skip(pool))]
pub async fn get_unread_counts_for_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<(i64, i32)>, sqlx::Error> {
    let rows: Vec<(i64, i32)> = sqlx::query_as(
        "SELECT dm_channel_id, unread_count FROM dm_channel_unreads WHERE user_id = $1",
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dm_unread_row_creation() {
        let now = Utc::now();
        let row = DmChannelUnreadRow {
            id: 1,
            user_id: 100,
            dm_channel_id: 200,
            unread_count: 3,
            last_message_id: Some(300),
            created_at: now,
            updated_at: now,
        };

        assert_eq!(row.unread_count, 3);
        assert_eq!(row.last_message_id, Some(300));
    }
}
