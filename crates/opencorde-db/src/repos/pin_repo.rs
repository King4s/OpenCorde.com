//! # Repository: Pinned Messages
//! CRUD operations for channel pinned messages.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct PinnedMessageRow {
    pub message_id: i64,
    pub channel_id: i64,
    pub author_id: i64,
    pub content: String,
    pub attachments: serde_json::Value,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub author_username: String,
    pub pinned_by: i64,
    pub pinned_at: DateTime<Utc>,
}

pub async fn pin_message(
    pool: &PgPool,
    channel_id: Snowflake,
    message_id: Snowflake,
    pinned_by: Snowflake,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO pinned_messages (channel_id, message_id, pinned_by) VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(channel_id.as_i64())
    .bind(message_id.as_i64())
    .bind(pinned_by.as_i64())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn unpin_message(
    pool: &PgPool,
    channel_id: Snowflake,
    message_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let result = sqlx::query(
        "DELETE FROM pinned_messages WHERE channel_id = $1 AND message_id = $2",
    )
    .bind(channel_id.as_i64())
    .bind(message_id.as_i64())
    .execute(pool)
    .await?;
    Ok(result.rows_affected() > 0)
}

pub async fn list_pinned(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Vec<PinnedMessageRow>, sqlx::Error> {
    sqlx::query_as::<_, PinnedMessageRow>(
        "SELECT m.id as message_id, m.channel_id, m.author_id, m.content, m.attachments, \
         m.edited_at, m.created_at, u.username as author_username, pm.pinned_by, pm.pinned_at \
         FROM pinned_messages pm \
         JOIN messages m ON pm.message_id = m.id \
         JOIN users u ON m.author_id = u.id \
         WHERE pm.channel_id = $1 \
         ORDER BY pm.pinned_at DESC",
    )
    .bind(channel_id.as_i64())
    .fetch_all(pool)
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pinned_message_row_fields() {
        let _ = std::mem::size_of::<PinnedMessageRow>();
    }
}
