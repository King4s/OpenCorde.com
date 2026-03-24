//! # Repository: Forum Channels
//! Posts and replies for forum-type channels.
//!
//! ## Depends On
//! - sqlx, opencorde_core::Snowflake, chrono

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ForumPostRow {
    pub id: i64,
    pub channel_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub title: String,
    pub content: String,
    pub reply_count: i32,
    pub pinned: bool,
    pub created_at: DateTime<Utc>,
    pub last_reply_at: DateTime<Utc>,
}

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ForumReplyRow {
    pub id: i64,
    pub post_id: i64,
    pub author_id: i64,
    pub author_username: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

/// Create a new forum post. Joins with users table for author_username.
#[tracing::instrument(skip(pool, title, content))]
pub async fn create_post(
    pool: &PgPool,
    id: Snowflake,
    channel_id: Snowflake,
    author_id: Snowflake,
    title: &str,
    content: &str,
) -> Result<ForumPostRow, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        author_id = author_id.as_i64(),
        "creating forum post"
    );

    let row = sqlx::query_as::<_, ForumPostRow>(
        "WITH inserted AS ( \
             INSERT INTO forum_posts (id, channel_id, author_id, title, content) \
             VALUES ($1, $2, $3, $4, $5) RETURNING * \
         ) \
         SELECT i.id, i.channel_id, i.author_id, u.username as author_username, \
                i.title, i.content, i.reply_count, i.pinned, i.created_at, i.last_reply_at \
         FROM inserted i \
         JOIN users u ON i.author_id = u.id",
    )
    .bind(id.as_i64())
    .bind(channel_id.as_i64())
    .bind(author_id.as_i64())
    .bind(title)
    .bind(content)
    .fetch_one(pool)
    .await?;

    tracing::info!(post_id = row.id, "post created successfully");
    Ok(row)
}

/// List posts in a channel, ordered by pinned (DESC), then last_reply_at (DESC).
#[tracing::instrument(skip(pool))]
pub async fn list_posts(
    pool: &PgPool,
    channel_id: Snowflake,
    limit: i64,
) -> Result<Vec<ForumPostRow>, sqlx::Error> {
    tracing::info!(
        channel_id = channel_id.as_i64(),
        limit = limit,
        "listing forum posts"
    );

    sqlx::query_as::<_, ForumPostRow>(
        "SELECT fp.id, fp.channel_id, fp.author_id, u.username as author_username, \
                fp.title, fp.content, fp.reply_count, fp.pinned, fp.created_at, fp.last_reply_at \
         FROM forum_posts fp \
         JOIN users u ON fp.author_id = u.id \
         WHERE fp.channel_id = $1 \
         ORDER BY fp.pinned DESC, fp.last_reply_at DESC \
         LIMIT $2",
    )
    .bind(channel_id.as_i64())
    .bind(limit)
    .fetch_all(pool)
    .await
}

/// Get a single post by ID.
#[tracing::instrument(skip(pool))]
pub async fn get_post(
    pool: &PgPool,
    post_id: Snowflake,
) -> Result<Option<ForumPostRow>, sqlx::Error> {
    tracing::info!(post_id = post_id.as_i64(), "getting forum post");

    sqlx::query_as::<_, ForumPostRow>(
        "SELECT fp.id, fp.channel_id, fp.author_id, u.username as author_username, \
                fp.title, fp.content, fp.reply_count, fp.pinned, fp.created_at, fp.last_reply_at \
         FROM forum_posts fp \
         JOIN users u ON fp.author_id = u.id \
         WHERE fp.id = $1",
    )
    .bind(post_id.as_i64())
    .fetch_optional(pool)
    .await
}

/// Delete a post by ID.
#[tracing::instrument(skip(pool))]
pub async fn delete_post(pool: &PgPool, post_id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(post_id = post_id.as_i64(), "deleting forum post");

    sqlx::query("DELETE FROM forum_posts WHERE id = $1")
        .bind(post_id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Create a reply to a post. Also increments reply_count and updates last_reply_at on the post.
#[tracing::instrument(skip(pool, content))]
pub async fn create_reply(
    pool: &PgPool,
    id: Snowflake,
    post_id: Snowflake,
    author_id: Snowflake,
    content: &str,
) -> Result<ForumReplyRow, sqlx::Error> {
    tracing::info!(
        post_id = post_id.as_i64(),
        author_id = author_id.as_i64(),
        "creating forum reply"
    );

    let row = sqlx::query_as::<_, ForumReplyRow>(
        "WITH inserted AS ( \
             INSERT INTO forum_replies (id, post_id, author_id, content) \
             VALUES ($1, $2, $3, $4) RETURNING * \
         ), updated AS ( \
             UPDATE forum_posts \
             SET reply_count = reply_count + 1, last_reply_at = NOW() \
             WHERE id = $2 \
             RETURNING id \
         ) \
         SELECT i.id, i.post_id, i.author_id, u.username as author_username, \
                i.content, i.created_at \
         FROM inserted i \
         JOIN users u ON i.author_id = u.id",
    )
    .bind(id.as_i64())
    .bind(post_id.as_i64())
    .bind(author_id.as_i64())
    .bind(content)
    .fetch_one(pool)
    .await?;

    tracing::info!(reply_id = row.id, "reply created successfully");
    Ok(row)
}

/// List all replies to a post, ordered by created_at ASC.
#[tracing::instrument(skip(pool))]
pub async fn list_replies(
    pool: &PgPool,
    post_id: Snowflake,
) -> Result<Vec<ForumReplyRow>, sqlx::Error> {
    tracing::info!(post_id = post_id.as_i64(), "listing forum replies");

    sqlx::query_as::<_, ForumReplyRow>(
        "SELECT fr.id, fr.post_id, fr.author_id, u.username as author_username, \
                fr.content, fr.created_at \
         FROM forum_replies fr \
         JOIN users u ON fr.author_id = u.id \
         WHERE fr.post_id = $1 \
         ORDER BY fr.created_at ASC",
    )
    .bind(post_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Delete a reply and decrement the post's reply_count.
#[tracing::instrument(skip(pool))]
pub async fn delete_reply(pool: &PgPool, reply_id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(reply_id = reply_id.as_i64(), "deleting forum reply");

    // Get post_id first so we can update it
    let post_id: (i64,) = sqlx::query_as("SELECT post_id FROM forum_replies WHERE id = $1")
        .bind(reply_id.as_i64())
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| sqlx::Error::RowNotFound)?;

    // Delete reply and decrement count
    sqlx::query(
        "DELETE FROM forum_replies WHERE id = $1; \
         UPDATE forum_posts SET reply_count = GREATEST(0, reply_count - 1) WHERE id = $2",
    )
    .bind(reply_id.as_i64())
    .bind(post_id.0)
    .execute(pool)
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_forum_post_row_has_all_fields() {
        let _ = std::mem::size_of::<ForumPostRow>();
    }

    #[test]
    fn test_forum_reply_row_has_all_fields() {
        let _ = std::mem::size_of::<ForumReplyRow>();
    }
}
