//! # Repository: Scheduled Messages
//! CRUD operations for scheduled/send-later messages.
//!
//! Messages are created with a future `scheduled_at` time.
//! A background delivery job periodically fetches due messages,
//! verifies permissions, creates the actual message, and marks
//! the scheduled entry as delivered.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value as JsonValue;
use sqlx::PgPool;

/// Row type for reading scheduled messages from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ScheduledMessageRow {
    pub id: i64,
    pub author_id: i64,
    pub channel_id: i64,
    pub content: String,
    pub attachments: JsonValue,
    pub reply_to_id: Option<i64>,
    pub scheduled_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub delivered: bool,
    pub delivered_at: Option<DateTime<Utc>>,
    pub cancelled: bool,
    pub cancelled_at: Option<DateTime<Utc>>,
}

/// Create a new scheduled message.
#[tracing::instrument(skip(pool, content))]
pub async fn create_scheduled(
    pool: &PgPool,
    id: Snowflake,
    author_id: Snowflake,
    channel_id: Snowflake,
    content: &str,
    reply_to_id: Option<Snowflake>,
    attachments: serde_json::Value,
    scheduled_at: DateTime<Utc>,
) -> Result<ScheduledMessageRow, sqlx::Error> {
    tracing::info!(
        scheduled_id = id.as_i64(),
        channel_id = channel_id.as_i64(),
        author_id = author_id.as_i64(),
        scheduled_at = %scheduled_at,
        "creating scheduled message"
    );

    let row = sqlx::query_as::<_, ScheduledMessageRow>(
        "INSERT INTO scheduled_messages (id, author_id, channel_id, content, attachments, reply_to_id, scheduled_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(author_id.as_i64())
    .bind(channel_id.as_i64())
    .bind(content)
    .bind(sqlx::types::Json(attachments))
    .bind(reply_to_id.map(|sf| sf.as_i64()))
    .bind(scheduled_at)
    .fetch_one(pool)
    .await?;

    tracing::info!(scheduled_id = row.id, "scheduled message created");
    Ok(row)
}

/// List pending (undelivered, not cancelled) scheduled messages for a user.
/// Ordered by scheduled_at ascending.
#[tracing::instrument(skip(pool))]
pub async fn list_user_scheduled(
    pool: &PgPool,
    author_id: Snowflake,
    limit: i64,
) -> Result<Vec<ScheduledMessageRow>, sqlx::Error> {
    let limit = std::cmp::min(limit, 100);

    tracing::info!(
        author_id = author_id.as_i64(),
        limit = limit,
        "listing user scheduled messages"
    );

    let rows = sqlx::query_as::<_, ScheduledMessageRow>(
        "SELECT * FROM scheduled_messages \
         WHERE author_id = $1 AND NOT delivered AND NOT cancelled \
         ORDER BY scheduled_at ASC LIMIT $2",
    )
    .bind(author_id.as_i64())
    .bind(limit)
    .fetch_all(pool)
    .await?;

    tracing::info!(count = rows.len(), "user scheduled messages fetched");
    Ok(rows)
}

/// Get a single scheduled message by ID (must belong to the given author).
#[tracing::instrument(skip(pool))]
pub async fn get_user_scheduled(
    pool: &PgPool,
    id: Snowflake,
    author_id: Snowflake,
) -> Result<Option<ScheduledMessageRow>, sqlx::Error> {
    sqlx::query_as::<_, ScheduledMessageRow>(
        "SELECT * FROM scheduled_messages WHERE id = $1 AND author_id = $2",
    )
    .bind(id.as_i64())
    .bind(author_id.as_i64())
    .fetch_optional(pool)
    .await
}

/// Update the content and/or scheduled_at of a pending scheduled message.
/// Only updates non-null fields.
#[tracing::instrument(skip(pool))]
pub async fn update_scheduled(
    pool: &PgPool,
    id: Snowflake,
    author_id: Snowflake,
    content: Option<&str>,
    scheduled_at: Option<DateTime<Utc>>,
) -> Result<Option<ScheduledMessageRow>, sqlx::Error> {
    // Build dynamic query — only update fields that are Some
    // We need to check ownership and return the updated row
    let row = sqlx::query_as::<_, ScheduledMessageRow>(
        "UPDATE scheduled_messages SET \
            content = COALESCE($3, content), \
            scheduled_at = COALESCE($4, scheduled_at) \
         WHERE id = $1 AND author_id = $2 AND NOT delivered AND NOT cancelled \
         RETURNING *",
    )
    .bind(id.as_i64())
    .bind(author_id.as_i64())
    .bind(content)
    .bind(scheduled_at)
    .fetch_optional(pool)
    .await?;

    if let Some(ref r) = row {
        tracing::info!(scheduled_id = r.id, "scheduled message updated");
    }
    Ok(row)
}

/// Cancel a scheduled message (soft-delete — sets cancelled=true).
#[tracing::instrument(skip(pool))]
pub async fn cancel_scheduled(
    pool: &PgPool,
    id: Snowflake,
    author_id: Snowflake,
) -> Result<Option<ScheduledMessageRow>, sqlx::Error> {
    let row = sqlx::query_as::<_, ScheduledMessageRow>(
        "UPDATE scheduled_messages \
         SET cancelled = TRUE, cancelled_at = NOW() \
         WHERE id = $1 AND author_id = $2 AND NOT delivered AND NOT cancelled \
         RETURNING *",
    )
    .bind(id.as_i64())
    .bind(author_id.as_i64())
    .fetch_optional(pool)
    .await?;

    if let Some(ref r) = row {
        tracing::info!(scheduled_id = r.id, "scheduled message cancelled");
    }
    Ok(row)
}

/// Fetch all scheduled messages that are due for delivery (scheduled_at <= now,
/// not delivered, not cancelled). Used by the background delivery job.
#[tracing::instrument(skip(pool))]
pub async fn fetch_due_messages(
    pool: &PgPool,
    limit: i64,
) -> Result<Vec<ScheduledMessageRow>, sqlx::Error> {
    let limit = std::cmp::min(limit, 50);

    let rows = sqlx::query_as::<_, ScheduledMessageRow>(
        "SELECT * FROM scheduled_messages \
         WHERE scheduled_at <= NOW() AND NOT delivered AND NOT cancelled \
         ORDER BY scheduled_at ASC LIMIT $1",
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    tracing::debug!(count = rows.len(), "due scheduled messages fetched");
    Ok(rows)
}

/// Mark a scheduled message as delivered.
#[tracing::instrument(skip(pool))]
pub async fn mark_delivered(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE scheduled_messages SET delivered = TRUE, delivered_at = NOW() WHERE id = $1",
    )
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    tracing::info!(
        scheduled_id = id.as_i64(),
        "scheduled message marked delivered"
    );
    Ok(())
}
