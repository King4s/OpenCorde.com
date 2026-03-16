//! # Repository: Messages
//! CRUD operations for channel messages.
//!
//! Supports cursor-based pagination (before/after) for efficient message retrieval.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

/// Row type for reading messages from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct MessageRow {
    pub id: i64,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub attachments: JsonValue,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Create a new message in a channel.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the message
/// * `channel_id` - Snowflake ID of the target channel
/// * `author_id` - Snowflake ID of the message author
/// * `content` - Message text content
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool, content))]
pub async fn create_message(
    pool: &PgPool,
    id: Snowflake,
    channel_id: Snowflake,
    author_id: Snowflake,
    content: &str,
) -> Result<MessageRow, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        author_id = author_id.as_i64(),
        "creating message"
    );

    let row = sqlx::query_as::<_, MessageRow>(
        "INSERT INTO messages (id, channel_id, author_id, content) \
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(channel_id.as_i64())
    .bind(author_id.as_i64())
    .bind(content)
    .fetch_one(pool)
    .await?;

    tracing::info!(message_id = row.id, "message created successfully");
    Ok(row)
}

/// Get a message by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List messages in a channel with cursor-based pagination.
///
/// Returns up to `limit` messages. Use `before` to get messages before a cursor,
/// or `after` to get messages after a cursor. Messages are ordered newest first.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake ID of the channel
/// * `before` - Optional cursor: fetch messages with ID < cursor
/// * `after` - Optional cursor: fetch messages with ID > cursor
/// * `limit` - Maximum messages to return (1-100, capped at 100)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_channel(
    pool: &PgPool,
    channel_id: Snowflake,
    before: Option<Snowflake>,
    after: Option<Snowflake>,
    limit: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let limit = std::cmp::min(limit, 100);

    // Handle conflicting cursors by preferring before
    let (before, after) = if before.is_some() && after.is_some() {
        tracing::warn!("both before and after specified, using before");
        (before, None)
    } else {
        (before, after)
    };

    tracing::info!(
        channel_id = channel_id.as_i64(),
        before = ?before,
        after = ?after,
        limit = limit,
        "listing channel messages"
    );

    let messages = match (before, after) {
        (Some(before_sf), None) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages \
                 WHERE channel_id = $1 AND id < $2 \
                 ORDER BY id DESC LIMIT $3",
            )
            .bind(channel_id.as_i64())
            .bind(before_sf.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        (None, Some(after_sf)) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages \
                 WHERE channel_id = $1 AND id > $2 \
                 ORDER BY id ASC LIMIT $3",
            )
            .bind(channel_id.as_i64())
            .bind(after_sf.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM messages \
                 WHERE channel_id = $1 \
                 ORDER BY id DESC LIMIT $2",
            )
            .bind(channel_id.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        (Some(_), Some(_)) => unreachable!("conflict resolved above"),
    };

    Ok(messages)
}

/// Update a message's content and mark it as edited.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool, new_content))]
pub async fn update_content(
    pool: &PgPool,
    id: Snowflake,
    new_content: &str,
) -> Result<(), sqlx::Error> {
    tracing::info!(message_id = id.as_i64(), "updating message content");

    sqlx::query("UPDATE messages SET content = $1, edited_at = NOW() WHERE id = $2")
        .bind(new_content)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete a message by its ID.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_message(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(message_id = id.as_i64(), "deleting message");

    sqlx::query("DELETE FROM messages WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_row_creation() {
        let now = Utc::now();
        let row = MessageRow {
            id: 777888999,
            channel_id: 555666777,
            author_id: 111222333,
            content: "Hello, world!".to_string(),
            attachments: JsonValue::Array(Vec::new()),
            edited_at: None,
            created_at: now,
        };

        assert_eq!(row.id, 777888999);
        assert_eq!(row.content, "Hello, world!");
        assert!(row.edited_at.is_none());
    }

    #[test]
    fn test_pagination_limit() {
        let limit = 200i64;
        let capped = std::cmp::min(limit, 100);
        assert_eq!(capped, 100);

        let limit = 50i64;
        let capped = std::cmp::min(limit, 100);
        assert_eq!(capped, 50);
    }
}
