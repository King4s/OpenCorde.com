//! # Repository: Channels
//! CRUD operations for server channels (text and voice).
//!
//! Supports channel types: 0=Text, 1=Voice, 2=Category.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading channels from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub channel_type: i16,
    pub topic: Option<String>,
    pub position: i32,
    pub parent_id: Option<i64>,
    pub nsfw: bool,
    /// Slowmode cooldown in seconds (0 = disabled).
    pub slowmode_delay: i32,
    /// Whether this channel uses E2EE (OpenMLS).
    pub e2ee_enabled: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new channel in a server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the channel
/// * `server_id` - Snowflake ID of the parent server
/// * `name` - Channel name (max 100 chars)
/// * `channel_type` - 0=Text, 1=Voice, 2=Category
/// * `nsfw` - Whether channel is marked NSFW
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn create_channel(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    name: &str,
    channel_type: i16,
    nsfw: bool,
) -> Result<ChannelRow, sqlx::Error> {
    tracing::info!(
        name = %name,
        server_id = server_id.as_i64(),
        channel_type = channel_type,
        nsfw = nsfw,
        "creating channel"
    );

    let row = sqlx::query_as::<_, ChannelRow>(
        "INSERT INTO channels (id, server_id, name, channel_type, nsfw) \
         VALUES ($1, $2, $3, $4, $5) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(channel_type)
    .bind(nsfw)
    .fetch_one(pool)
    .await?;

    tracing::info!(channel_id = row.id, "channel created successfully");
    Ok(row)
}

/// Get a channel by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<ChannelRow>, sqlx::Error> {
    sqlx::query_as::<_, ChannelRow>("SELECT * FROM channels WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all channels in a server, ordered by position.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<ChannelRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing server channels");

    sqlx::query_as::<_, ChannelRow>(
        "SELECT * FROM channels WHERE server_id = $1 ORDER BY position ASC, id ASC",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update a channel's name, topic, parent category, nsfw flag, slowmode delay, and e2ee flag.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_channel(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    topic: Option<&str>,
    parent_id: Option<Snowflake>,
    nsfw: Option<bool>,
    slowmode_delay: Option<i32>,
    e2ee_enabled: Option<bool>,
) -> Result<(), sqlx::Error> {
    tracing::info!(channel_id = id.as_i64(), name = %name, "updating channel");

    let parent_id_i64 = parent_id.map(|sf| sf.as_i64());
    let nsfw_val = nsfw.unwrap_or(false);
    let delay = slowmode_delay.unwrap_or(0);

    sqlx::query(
        "UPDATE channels \
         SET name = $1, topic = $2, parent_id = $3, nsfw = $4, slowmode_delay = $5, e2ee_enabled = $6, updated_at = NOW() \
         WHERE id = $7",
    )
    .bind(name)
    .bind(topic)
    .bind(parent_id_i64)
    .bind(nsfw_val)
    .bind(delay)
    .bind(e2ee_enabled.unwrap_or(false))
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a channel's position in the channel list.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_position(
    pool: &PgPool,
    id: Snowflake,
    position: i32,
) -> Result<(), sqlx::Error> {
    tracing::info!(
        channel_id = id.as_i64(),
        position = position,
        "updating channel position"
    );

    sqlx::query("UPDATE channels SET position = $1, updated_at = NOW() WHERE id = $2")
        .bind(position)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// List all channel IDs accessible to a user (via server membership).
///
/// Returns all channel IDs for every server the user belongs to.
/// Used by the WebSocket gateway to filter events to only channels the user can see.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_ids_by_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<i64>, sqlx::Error> {
    sqlx::query_scalar::<_, i64>(
        "SELECT c.id FROM channels c \
         INNER JOIN server_members m ON c.server_id = m.server_id \
         WHERE m.user_id = $1",
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Delete a channel by its ID.
///
/// Cascades to messages within the channel.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_channel(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(channel_id = id.as_i64(), "deleting channel");

    sqlx::query("DELETE FROM channels WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_channel_row_creation() {
        let now = Utc::now();
        let row = ChannelRow {
            id: 555666777,
            server_id: 111222333,
            name: "general".to_string(),
            channel_type: 0,
            topic: Some("Main discussion".to_string()),
            position: 0,
            parent_id: None,
            nsfw: false,
            slowmode_delay: 0,
            e2ee_enabled: false,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(row.id, 555666777);
        assert_eq!(row.name, "general");
        assert_eq!(row.channel_type, 0);
        assert!(!row.nsfw);
    }
}
