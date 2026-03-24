//! # Repository: Webhooks
//! CRUD operations for incoming webhooks.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading webhooks from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct WebhookRow {
    pub id: i64,
    pub channel_id: i64,
    pub server_id: i64,
    pub name: String,
    pub token: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Create a new webhook for a channel.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the webhook
/// * `channel_id` - Snowflake ID of the target channel
/// * `server_id` - Snowflake ID of the server
/// * `name` - Webhook display name
/// * `token` - Unique token for webhook execution (32-char hex string)
/// * `created_by` - Snowflake ID of the user creating the webhook
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool, name, token))]
pub async fn create_webhook(
    pool: &PgPool,
    id: Snowflake,
    channel_id: Snowflake,
    server_id: Snowflake,
    name: &str,
    token: &str,
    created_by: Snowflake,
) -> Result<WebhookRow, sqlx::Error> {
    sqlx::query_as::<_, WebhookRow>(
        "INSERT INTO webhooks (id, channel_id, server_id, name, token, created_by, created_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW())
         RETURNING *",
    )
    .bind(id.as_i64())
    .bind(channel_id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(token)
    .bind(created_by.as_i64())
    .fetch_one(pool)
    .await
}

/// Get a webhook by ID.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID of the webhook
///
/// # Errors
/// Returns sqlx::Error on database error.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(
    pool: &PgPool,
    id: Snowflake,
) -> Result<Option<WebhookRow>, sqlx::Error> {
    sqlx::query_as::<_, WebhookRow>("SELECT * FROM webhooks WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// Get a webhook by its token.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `token` - Token string (32-char hex)
///
/// # Errors
/// Returns sqlx::Error on database error.
#[tracing::instrument(skip(pool, token))]
pub async fn get_by_token(
    pool: &PgPool,
    token: &str,
) -> Result<Option<WebhookRow>, sqlx::Error> {
    sqlx::query_as::<_, WebhookRow>("SELECT * FROM webhooks WHERE token = $1")
        .bind(token)
        .fetch_optional(pool)
        .await
}

/// List all webhooks in a channel, ordered by creation time (newest first).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `channel_id` - Snowflake ID of the channel
///
/// # Errors
/// Returns sqlx::Error on database error.
#[tracing::instrument(skip(pool))]
pub async fn list_by_channel(
    pool: &PgPool,
    channel_id: Snowflake,
) -> Result<Vec<WebhookRow>, sqlx::Error> {
    sqlx::query_as::<_, WebhookRow>(
        "SELECT * FROM webhooks WHERE channel_id = $1 ORDER BY created_at DESC",
    )
    .bind(channel_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Delete a webhook by ID.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID of the webhook
///
/// # Errors
/// Returns sqlx::Error on database error.
#[tracing::instrument(skip(pool))]
pub async fn delete_webhook(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM webhooks WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;
    Ok(())
}
