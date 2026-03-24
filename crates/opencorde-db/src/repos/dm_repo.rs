//! # Repository: Direct Messages
//! CRUD operations for direct message channels and messages.
//!
//! Supports get-or-create DM logic with transaction safety,
//! cursor-based pagination for messages, and membership validation.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - sqlx (async database access)
//! - chrono (timestamps)
//! - serde_json (attachments storage)

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

/// Row type for reading DM channels with the other participant's info.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmChannelRow {
    pub id: i64,
    pub other_user_id: i64,
    pub other_username: String,
    pub last_read_id: i64,
}

/// Row type for reading DM messages.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct DmMessageRow {
    pub id: i64,
    pub dm_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub content: String,
    pub attachments: JsonValue,
    pub edited_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

/// Get or create a DM channel between two users.
///
/// Finds an existing DM channel where both users are members,
/// or creates a new one with the provided Snowflake ID and
/// adds both users as members.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `dm_id` - Snowflake ID for the new DM channel (if creating)
/// * `user_a` - First user's Snowflake ID
/// * `user_b` - Second user's Snowflake ID
///
/// # Errors
/// Returns sqlx::Error if the query/transaction fails.
#[tracing::instrument(skip(pool))]
pub async fn get_or_create_dm(
    pool: &PgPool,
    dm_id: Snowflake,
    user_a: Snowflake,
    user_b: Snowflake,
) -> Result<i64, sqlx::Error> {
    let user_a_id = user_a.as_i64();
    let user_b_id = user_b.as_i64();
    let dm_id_val = dm_id.as_i64();

    // Check for existing DM where both users are members
    let existing = sqlx::query_scalar::<_, i64>(
        "SELECT d.id FROM dm_channels d \
         JOIN dm_channel_members m1 ON m1.dm_channel_id = d.id AND m1.user_id = $1 \
         JOIN dm_channel_members m2 ON m2.dm_channel_id = d.id AND m2.user_id = $2 \
         LIMIT 1",
    )
    .bind(user_a_id)
    .bind(user_b_id)
    .fetch_optional(pool)
    .await?;

    if let Some(dm_id) = existing {
        tracing::info!(dm_id = dm_id, "found existing dm channel");
        return Ok(dm_id);
    }

    // Create new DM channel and add both members
    let mut tx = pool.begin().await?;

    sqlx::query("INSERT INTO dm_channels (id) VALUES ($1)")
        .bind(dm_id_val)
        .execute(&mut *tx)
        .await?;

    sqlx::query(
        "INSERT INTO dm_channel_members (dm_channel_id, user_id, last_read_id) VALUES ($1, $2, $3), ($1, $4, $3)",
    )
    .bind(dm_id_val)
    .bind(user_a_id)
    .bind(0i64)
    .bind(user_b_id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;

    tracing::info!(dm_id = dm_id_val, "created new dm channel");
    Ok(dm_id_val)
}

/// List all DM channels for a user.
///
/// Returns DM channels with the other participant's username
/// and that participant's last read message ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_dms_for_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<DmChannelRow>, sqlx::Error> {
    let user_id_val = user_id.as_i64();

    tracing::info!("listing dm channels");

    sqlx::query_as::<_, DmChannelRow>(
        "SELECT d.id, u.id as other_user_id, u.username as other_username, m.last_read_id \
         FROM dm_channels d \
         JOIN dm_channel_members m ON m.dm_channel_id = d.id AND m.user_id = $1 \
         JOIN dm_channel_members m2 ON m2.dm_channel_id = d.id AND m2.user_id != $1 \
         JOIN users u ON u.id = m2.user_id \
         ORDER BY d.id DESC",
    )
    .bind(user_id_val)
    .fetch_all(pool)
    .await
}

/// Send a DM message to a channel.
///
/// Creates a new message with the provided Snowflake ID and
/// returns the full message with author username populated.
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool, content))]
pub async fn send_dm_message(
    pool: &PgPool,
    id: Snowflake,
    dm_id: Snowflake,
    author_id: Snowflake,
    content: &str,
) -> Result<DmMessageRow, sqlx::Error> {
    tracing::info!(dm_id = dm_id.as_i64(), author_id = author_id.as_i64(), "sending dm message");

    let row = sqlx::query_as::<_, DmMessageRow>(
        "WITH inserted AS ( \
             INSERT INTO dm_messages (id, dm_id, author_id, content) \
             VALUES ($1, $2, $3, $4) RETURNING * \
         ) \
         SELECT i.id, i.dm_id, i.author_id, u.username as author_username, i.content, i.attachments, i.edited_at, i.created_at \
         FROM inserted i \
         JOIN users u ON i.author_id = u.id",
    )
    .bind(id.as_i64())
    .bind(dm_id.as_i64())
    .bind(author_id.as_i64())
    .bind(content)
    .fetch_one(pool)
    .await?;

    tracing::info!(message_id = row.id, "dm message sent successfully");
    Ok(row)
}

/// List DM messages in a channel with cursor-based pagination.
///
/// Returns up to `limit` messages ordered newest first.
/// Use `before` to get messages before a cursor ID.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `dm_id` - Snowflake ID of the DM channel
/// * `before` - Optional cursor: fetch messages with ID < cursor
/// * `limit` - Maximum messages to return (capped at 100)
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_dm_messages(
    pool: &PgPool,
    dm_id: Snowflake,
    before: Option<Snowflake>,
    limit: i64,
) -> Result<Vec<DmMessageRow>, sqlx::Error> {
    let limit = std::cmp::min(limit, 100);
    let dm_id_val = dm_id.as_i64();

    tracing::info!(dm_id = dm_id_val, before = ?before, limit = limit, "listing dm messages");

    let messages = match before {
        Some(before_sf) => {
            sqlx::query_as::<_, DmMessageRow>(
                "SELECT m.id, m.dm_id, m.author_id, u.username as author_username, m.content, m.attachments, m.edited_at, m.created_at \
                 FROM dm_messages m \
                 JOIN users u ON m.author_id = u.id \
                 WHERE m.dm_id = $1 AND m.id < $2 \
                 ORDER BY m.id DESC LIMIT $3",
            )
            .bind(dm_id_val)
            .bind(before_sf.as_i64())
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
        None => {
            sqlx::query_as::<_, DmMessageRow>(
                "SELECT m.id, m.dm_id, m.author_id, u.username as author_username, m.content, m.attachments, m.edited_at, m.created_at \
                 FROM dm_messages m \
                 JOIN users u ON m.author_id = u.id \
                 WHERE m.dm_id = $1 \
                 ORDER BY m.id DESC LIMIT $2",
            )
            .bind(dm_id_val)
            .bind(limit)
            .fetch_all(pool)
            .await?
        }
    };

    Ok(messages)
}

/// Check if a user is a member of a DM channel.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn is_dm_member(
    pool: &PgPool,
    dm_id: Snowflake,
    user_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM dm_channel_members WHERE dm_channel_id = $1 AND user_id = $2)",
    )
    .bind(dm_id.as_i64())
    .bind(user_id.as_i64())
    .fetch_one(pool)
    .await?;

    Ok(exists)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dm_channel_row_creation() {
        let row = DmChannelRow {
            id: 111222333,
            other_user_id: 444555666,
            other_username: "alice".to_string(),
            last_read_id: 999888777,
        };

        assert_eq!(row.id, 111222333);
        assert_eq!(row.other_username, "alice");
    }

    #[test]
    fn test_dm_message_row_creation() {
        let now = Utc::now();
        let row = DmMessageRow {
            id: 777888999,
            dm_id: 111222333,
            author_id: 444555666,
            author_username: "bob".to_string(),
            content: "Hello!".to_string(),
            attachments: JsonValue::Array(Vec::new()),
            edited_at: None,
            created_at: now,
        };

        assert_eq!(row.dm_id, 111222333);
        assert_eq!(row.content, "Hello!");
        assert_eq!(row.author_username, "bob");
    }

    #[test]
    fn test_message_limit_capping() {
        let limit = 200i64;
        let capped = std::cmp::min(limit, 100);
        assert_eq!(capped, 100);
    }
}
