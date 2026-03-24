//! # Repository: Threads
//! CRUD operations for message threads.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (database operations)
//! - chrono (timestamps)

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading threads from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ThreadRow {
    pub id: i64,
    pub channel_id: i64,
    pub parent_msg_id: Option<i64>,
    pub name: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub last_msg_at: DateTime<Utc>,
    pub msg_count: i32,
}

/// Create a new thread for a message.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the thread
/// * `channel_id` - Snowflake ID of the channel
/// * `parent_msg_id` - Optional Snowflake ID of the parent message
/// * `name` - Thread name/title
/// * `created_by` - Snowflake ID of the user creating the thread
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool, name))]
pub async fn create_thread(
    pool: &PgPool,
    id: Snowflake,
    channel_id: Snowflake,
    parent_msg_id: Option<Snowflake>,
    name: &str,
    created_by: Snowflake,
) -> Result<ThreadRow, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        created_by = created_by.as_i64(),
        parent_msg_id = ?parent_msg_id,
        "creating thread"
    );

    let row = sqlx::query_as::<_, ThreadRow>(
        "INSERT INTO threads (id, channel_id, parent_msg_id, name, created_by, created_at, last_msg_at, msg_count) \
         VALUES ($1, $2, $3, $4, $5, NOW(), NOW(), 0) \
         RETURNING *",
    )
    .bind(id.as_i64())
    .bind(channel_id.as_i64())
    .bind(parent_msg_id.map(|sf| sf.as_i64()))
    .bind(name)
    .bind(created_by.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(thread_id = row.id, "thread created successfully");
    Ok(row)
}

/// Get a thread by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<ThreadRow>, sqlx::Error> {
    sqlx::query_as::<_, ThreadRow>("SELECT * FROM threads WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List threads in a channel ordered by last message time.
///
/// Returns up to 50 most recently active threads.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_channel(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Vec<ThreadRow>, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        "listing channel threads"
    );

    let threads = sqlx::query_as::<_, ThreadRow>(
        "SELECT * FROM threads WHERE channel_id = $1 ORDER BY last_msg_at DESC LIMIT 50",
    )
    .bind(channel_id.as_i64())
    .fetch_all(pool)
    .await?;

    tracing::info!(count = threads.len(), "threads fetched successfully");
    Ok(threads)
}

/// Increment the message count and update last_msg_at for a thread.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn increment_message_count(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(thread_id = id.as_i64(), "incrementing message count");

    sqlx::query(
        "UPDATE threads SET msg_count = msg_count + 1, last_msg_at = NOW() WHERE id = $1",
    )
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_thread_row_creation() {
        let now = Utc::now();
        let row = ThreadRow {
            id: 777888999,
            channel_id: 555666777,
            parent_msg_id: Some(123456789),
            name: "General Discussion".to_string(),
            created_by: 111222333,
            created_at: now,
            last_msg_at: now,
            msg_count: 5,
        };

        assert_eq!(row.id, 777888999);
        assert_eq!(row.name, "General Discussion");
        assert_eq!(row.msg_count, 5);
        assert!(row.parent_msg_id.is_some());
    }
}
