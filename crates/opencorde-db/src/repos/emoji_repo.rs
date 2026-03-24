//! # Repository: Server Emojis
//! Custom emoji management per server.
//!
//! ## Depends On
//! - sqlx, opencorde_core::Snowflake, chrono

use chrono::{DateTime, Utc};
use sqlx::PgPool;

/// Row type for reading server emoji entries from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerEmojiRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub image_url: String,
    pub uploaded_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Create a server emoji entry.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the emoji
/// * `server_id` - Snowflake ID of the server
/// * `name` - Emoji name (lowercase letters, numbers, underscores)
/// * `image_url` - URL to the emoji image file
/// * `uploaded_by` - Snowflake ID of the user who uploaded the emoji
///
/// # Errors
/// Returns sqlx::Error if the insert fails or unique constraint is violated.
#[tracing::instrument(skip(pool))]
pub async fn create_emoji(
    pool: &PgPool,
    id: i64,
    server_id: i64,
    name: &str,
    image_url: &str,
    uploaded_by: i64,
) -> Result<ServerEmojiRow, sqlx::Error> {
    tracing::info!(
        emoji_id = id,
        server_id = server_id,
        name = %name,
        "creating emoji"
    );

    sqlx::query_as::<_, ServerEmojiRow>(
        r#"
        INSERT INTO server_emojis (id, server_id, name, image_url, uploaded_by)
        VALUES ($1, $2, $3, $4, $5)
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(server_id)
    .bind(name)
    .bind(image_url)
    .bind(uploaded_by)
    .fetch_one(pool)
    .await
}

/// List all emojis for a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_id` - Snowflake ID of the server
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_emojis(
    pool: &PgPool,
    server_id: i64,
) -> Result<Vec<ServerEmojiRow>, sqlx::Error> {
    tracing::debug!(server_id = server_id, "listing server emojis");

    let rows = sqlx::query_as::<_, ServerEmojiRow>(
        "SELECT * FROM server_emojis WHERE server_id = $1 ORDER BY created_at ASC",
    )
    .bind(server_id)
    .fetch_all(pool)
    .await?;

    tracing::debug!(count = rows.len(), server_id = server_id, "emojis fetched");
    Ok(rows)
}

/// Delete a server emoji.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `emoji_id` - Snowflake ID of the emoji to delete
/// * `server_id` - Snowflake ID of the server (for validation)
///
/// # Returns
/// Returns true if the emoji was deleted, false if it didn't exist.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_emoji(
    pool: &PgPool,
    emoji_id: i64,
    server_id: i64,
) -> Result<bool, sqlx::Error> {
    tracing::info!(
        emoji_id = emoji_id,
        server_id = server_id,
        "deleting emoji"
    );

    let result = sqlx::query(
        "DELETE FROM server_emojis WHERE id = $1 AND server_id = $2",
    )
    .bind(emoji_id)
    .bind(server_id)
    .execute(pool)
    .await?;

    let deleted = result.rows_affected() > 0;
    tracing::debug!(deleted = deleted, "emoji deletion processed");
    Ok(deleted)
}

/// Get all emojis for multiple servers (for message rendering).
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `server_ids` - Slice of server IDs to fetch emojis for
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool, server_ids))]
pub async fn get_all_for_message_rendering(
    pool: &PgPool,
    server_ids: &[i64],
) -> Result<Vec<ServerEmojiRow>, sqlx::Error> {
    if server_ids.is_empty() {
        return Ok(Vec::new());
    }

    tracing::debug!(
        count = server_ids.len(),
        "fetching emojis for message rendering"
    );

    let rows = sqlx::query_as::<_, ServerEmojiRow>(
        "SELECT * FROM server_emojis WHERE server_id = ANY($1) ORDER BY server_id, created_at",
    )
    .bind(server_ids)
    .fetch_all(pool)
    .await?;

    tracing::debug!(count = rows.len(), "emojis fetched for rendering");
    Ok(rows)
}
