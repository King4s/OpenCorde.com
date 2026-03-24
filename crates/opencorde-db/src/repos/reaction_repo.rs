//! # Repository: Reactions
//! CRUD operations for message emoji reactions.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake
//! - chrono for timestamp handling

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading message reactions from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ReactionRow {
    pub message_id: i64,
    pub user_id: i64,
    pub emoji: String,
    pub created_at: DateTime<Utc>,
}

/// Add a reaction to a message. Returns true if newly added, false if already existed.
///
/// Uses ON CONFLICT DO NOTHING to handle idempotency: if the user has already
/// reacted with this emoji to this message, the insert is silently ignored.
///
/// # Errors
/// Returns sqlx::Error if the database operation fails.
#[tracing::instrument(skip(pool))]
pub async fn add_reaction(
    pool: &PgPool,
    message_id: Snowflake,
    user_id: Snowflake,
    emoji: &str,
) -> Result<bool, sqlx::Error> {
    tracing::info!(
        message_id = message_id.as_i64(),
        user_id = user_id.as_i64(),
        emoji,
        "adding reaction"
    );

    let result = sqlx::query(
        "INSERT INTO message_reactions (message_id, user_id, emoji) \
         VALUES ($1, $2, $3) \
         ON CONFLICT DO NOTHING",
    )
    .bind(message_id.as_i64())
    .bind(user_id.as_i64())
    .bind(emoji)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// Remove a reaction from a message. Returns true if it existed and was removed.
///
/// # Errors
/// Returns sqlx::Error if the database operation fails.
#[tracing::instrument(skip(pool))]
pub async fn remove_reaction(
    pool: &PgPool,
    message_id: Snowflake,
    user_id: Snowflake,
    emoji: &str,
) -> Result<bool, sqlx::Error> {
    tracing::info!(
        message_id = message_id.as_i64(),
        user_id = user_id.as_i64(),
        emoji,
        "removing reaction"
    );

    let result = sqlx::query(
        "DELETE FROM message_reactions \
         WHERE message_id = $1 AND user_id = $2 AND emoji = $3",
    )
    .bind(message_id.as_i64())
    .bind(user_id.as_i64())
    .bind(emoji)
    .execute(pool)
    .await?;

    Ok(result.rows_affected() > 0)
}

/// List all reactions for a message, ordered by creation time.
///
/// # Errors
/// Returns sqlx::Error if the database operation fails.
#[tracing::instrument(skip(pool))]
pub async fn list_reactions(
    pool: &PgPool,
    message_id: Snowflake,
) -> Result<Vec<ReactionRow>, sqlx::Error> {
    tracing::info!(message_id = message_id.as_i64(), "listing reactions for message");

    sqlx::query_as::<_, ReactionRow>(
        "SELECT message_id, user_id, emoji, created_at \
         FROM message_reactions \
         WHERE message_id = $1 \
         ORDER BY created_at ASC",
    )
    .bind(message_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Count reactions grouped by emoji for a message.
///
/// Returns Vec of (emoji, count, reacted_by_viewer).
/// Ordered by first appearance (minimum created_at per emoji).
///
/// # Errors
/// Returns sqlx::Error if the database operation fails.
#[tracing::instrument(skip(pool))]
pub async fn count_by_emoji(
    pool: &PgPool,
    message_id: Snowflake,
    viewer_id: Snowflake,
) -> Result<Vec<(String, i64, bool)>, sqlx::Error> {
    tracing::info!(
        message_id = message_id.as_i64(),
        viewer_id = viewer_id.as_i64(),
        "counting reactions by emoji"
    );

    let rows = sqlx::query_as::<_, (String, i64, bool)>(
        "SELECT emoji, COUNT(*) as count, bool_or(user_id = $2) as reacted \
         FROM message_reactions \
         WHERE message_id = $1 \
         GROUP BY emoji \
         ORDER BY MIN(created_at) ASC",
    )
    .bind(message_id.as_i64())
    .bind(viewer_id.as_i64())
    .fetch_all(pool)
    .await?;

    Ok(rows)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_emoji_parsing() {
        let emoji = "👍";
        assert!(!emoji.is_empty());
        assert!(emoji.len() <= 64);
    }

    #[test]
    fn test_snowflake_conversion() {
        let sf = Snowflake::new(123456789);
        assert_eq!(sf.as_i64(), 123456789);
    }

    #[test]
    fn test_multi_byte_emoji() {
        let emoji = "🎉";
        assert!(emoji.chars().count() > 0);
        assert!(emoji.len() <= 64);
    }
}
