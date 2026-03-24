//! # Query operations for messages
//! Retrieve and list messages with cursor-based pagination.

use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;
use super::crud::MessageRow;

/// Context for a message being replied to (minimal data for display).
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReplyContext {
    pub id: i64,
    pub author_username: String,
    pub content: String,
}

/// Get a message by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<MessageRow>, sqlx::Error> {
    sqlx::query_as::<_, MessageRow>(
        "SELECT m.id, m.channel_id, m.author_id, m.content, m.attachments, m.edited_at, m.created_at, \
                u.username as author_username, m.reply_to_id, \
                ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, m.thread_id \
         FROM messages m \
         JOIN users u ON m.author_id = u.id \
         LEFT JOIN messages rm ON m.reply_to_id = rm.id \
         LEFT JOIN users ru ON rm.author_id = ru.id \
         WHERE m.id = $1",
    )
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
                "SELECT m.id, m.channel_id, m.author_id, m.content, m.attachments, m.edited_at, m.created_at, \
                        u.username as author_username, m.reply_to_id, \
                        ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, m.thread_id \
                 FROM messages m \
                 JOIN users u ON m.author_id = u.id \
                 LEFT JOIN messages rm ON m.reply_to_id = rm.id \
                 LEFT JOIN users ru ON rm.author_id = ru.id \
                 WHERE m.channel_id = $1 AND m.id < $2 AND m.thread_id IS NULL \
                 ORDER BY m.id DESC LIMIT $3",
            )
            .bind(channel_id.as_i64())
            .bind(before_sf.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        (None, Some(after_sf)) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT m.id, m.channel_id, m.author_id, m.content, m.attachments, m.edited_at, m.created_at, \
                        u.username as author_username, m.reply_to_id, \
                        ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, m.thread_id \
                 FROM messages m \
                 JOIN users u ON m.author_id = u.id \
                 LEFT JOIN messages rm ON m.reply_to_id = rm.id \
                 LEFT JOIN users ru ON rm.author_id = ru.id \
                 WHERE m.channel_id = $1 AND m.id > $2 AND m.thread_id IS NULL \
                 ORDER BY m.id ASC LIMIT $3",
            )
            .bind(channel_id.as_i64())
            .bind(after_sf.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        (None, None) => {
            sqlx::query_as::<_, MessageRow>(
                "SELECT m.id, m.channel_id, m.author_id, m.content, m.attachments, m.edited_at, m.created_at, \
                        u.username as author_username, m.reply_to_id, \
                        ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, m.thread_id \
                 FROM messages m \
                 JOIN users u ON m.author_id = u.id \
                 LEFT JOIN messages rm ON m.reply_to_id = rm.id \
                 LEFT JOIN users ru ON rm.author_id = ru.id \
                 WHERE m.channel_id = $1 AND m.thread_id IS NULL \
                 ORDER BY m.id DESC LIMIT $2",
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

/// List messages in a thread (ordered oldest first for thread view).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `thread_id` - Snowflake ID of the thread
/// * `limit` - Maximum messages to return (1-100, capped at 100)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_thread(
    pool: &PgPool,
    thread_id: Snowflake,
    limit: i64,
) -> Result<Vec<MessageRow>, sqlx::Error> {
    let limit = std::cmp::min(limit, 100);

    tracing::info!(
        thread_id = thread_id.as_i64(),
        limit = limit,
        "listing thread messages"
    );

    let messages = sqlx::query_as::<_, MessageRow>(
        "SELECT m.id, m.channel_id, m.author_id, m.content, m.attachments, m.edited_at, m.created_at, \
                u.username as author_username, m.reply_to_id, \
                ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, m.thread_id \
         FROM messages m \
         JOIN users u ON m.author_id = u.id \
         LEFT JOIN messages rm ON m.reply_to_id = rm.id \
         LEFT JOIN users ru ON rm.author_id = ru.id \
         WHERE m.thread_id = $1 \
         ORDER BY m.id ASC LIMIT $2",
    )
    .bind(thread_id.as_i64())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    tracing::info!(count = messages.len(), "thread messages fetched successfully");
    Ok(messages)
}

/// Get reply context (minimal info) for a message being replied to.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_reply_context(
    pool: &PgPool,
    reply_to_id: Snowflake,
) -> Result<Option<ReplyContext>, sqlx::Error> {
    sqlx::query_as::<_, ReplyContext>(
        "SELECT m.id, u.username as author_username, LEFT(m.content, 100) as content \
         FROM messages m \
         JOIN users u ON m.author_id = u.id \
         WHERE m.id = $1",
    )
    .bind(reply_to_id.as_i64())
    .fetch_optional(pool)
    .await
}
