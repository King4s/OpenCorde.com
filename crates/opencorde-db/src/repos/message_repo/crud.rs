//! # CRUD operations for messages
//! Create, update, and delete operations on channel messages.

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
    pub author_username: String,
    pub reply_to_id: Option<i64>,
    /// Replied-to message's author username (from LEFT JOIN)
    pub reply_author_username: Option<String>,
    /// Replied-to message's content preview (from LEFT JOIN, first 100 chars)
    pub reply_content_preview: Option<String>,
    /// Optional thread ID if message is part of a thread
    pub thread_id: Option<i64>,
}

/// Create a new message in a channel.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the message
/// * `channel_id` - Snowflake ID of the target channel
/// * `author_id` - Snowflake ID of the message author
/// * `content` - Message text content
/// * `reply_to_id` - Optional Snowflake ID of the message being replied to
/// * `attachments` - Attachment metadata JSON array
/// * `thread_id` - Optional Snowflake ID of the thread this message belongs to
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[allow(clippy::too_many_arguments)]
#[tracing::instrument(skip(pool, content))]
pub async fn create_message(
    pool: &PgPool,
    id: Snowflake,
    channel_id: Snowflake,
    author_id: Snowflake,
    content: &str,
    reply_to_id: Option<Snowflake>,
    attachments: serde_json::Value,
    thread_id: Option<Snowflake>,
) -> Result<MessageRow, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        author_id = author_id.as_i64(),
        reply_to_id = ?reply_to_id,
        thread_id = ?thread_id,
        "creating message"
    );

    // Use CTE to insert and join with users in one query to get author_username and reply context
    let row = sqlx::query_as::<_, MessageRow>(
        "WITH inserted AS ( \
             INSERT INTO messages (id, channel_id, author_id, content, attachments, reply_to_id, thread_id) \
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING * \
         ) \
         SELECT i.id, i.channel_id, i.author_id, i.content, i.attachments, i.edited_at, i.created_at, \
                u.username as author_username, i.reply_to_id, \
                ru.username as reply_author_username, LEFT(rm.content, 100) as reply_content_preview, i.thread_id \
         FROM inserted i \
         JOIN users u ON i.author_id = u.id \
         LEFT JOIN messages rm ON i.reply_to_id = rm.id \
         LEFT JOIN users ru ON rm.author_id = ru.id",
    )
    .bind(id.as_i64())
    .bind(channel_id.as_i64())
    .bind(author_id.as_i64())
    .bind(content)
    .bind(sqlx::types::Json(attachments))
    .bind(reply_to_id.map(|sf| sf.as_i64()))
    .bind(thread_id.map(|sf| sf.as_i64()))
    .fetch_one(pool)
    .await?;

    tracing::info!(message_id = row.id, "message created successfully");
    Ok(row)
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
