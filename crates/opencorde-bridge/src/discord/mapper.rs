//! # Channel Mapping & Message Forwarding
//! DB queries for channel mappings and inserting/forwarding bridged messages.
//!
//! ## Responsibilities
//! - Load active channel mappings from bridge_channel_mappings
//! - Insert Discord messages into OpenCorde messages table
//! - Poll new OpenCorde messages and return them for forwarding to Discord
//! - Update cursor positions after processing
//!
//! ## Depends On
//! - sqlx (database)
//! - opencorde_core::SnowflakeGenerator (message ID generation)

use opencorde_core::SnowflakeGenerator;
use sqlx::{PgPool, Row};
use std::sync::Mutex;

static MSG_GENERATOR: Mutex<Option<SnowflakeGenerator>> = Mutex::new(None);

fn next_msg_id() -> i64 {
    let mut guard = MSG_GENERATOR.lock().expect("snowflake generator poisoned");
    let sg = guard.get_or_insert_with(|| SnowflakeGenerator::new(2, 0));
    sg.next_id().as_i64()
}

/// Active channel mapping row.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Fields read via bridge runtime at query time
pub struct ChannelMapping {
    pub id: i64,
    pub discord_channel_id: i64,
    pub discord_webhook_id: Option<i64>,
    pub discord_webhook_token: Option<String>,
    pub opencorde_channel_id: i64,
    pub last_discord_msg_id: i64,
    pub last_opencorde_msg_id: i64,
}

/// A pending OpenCorde message to be forwarded to Discord.
#[derive(Debug)]
pub struct PendingMessage {
    pub opencorde_msg_id: i64,
    pub author_username: String,
    pub author_avatar_url: Option<String>,
    pub content: String,
}

/// Load all enabled channel mappings.
pub async fn load_active_mappings(db: &PgPool) -> anyhow::Result<Vec<ChannelMapping>> {
    let rows = sqlx::query(
        "SELECT id, discord_channel_id, discord_webhook_id, discord_webhook_token,
                opencorde_channel_id, last_discord_msg_id, last_opencorde_msg_id
         FROM bridge_channel_mappings
         WHERE enabled = TRUE",
    )
    .fetch_all(db)
    .await?;

    let mappings = rows
        .iter()
        .map(|r| ChannelMapping {
            id: r.get("id"),
            discord_channel_id: r.get("discord_channel_id"),
            discord_webhook_id: r.get("discord_webhook_id"),
            discord_webhook_token: r.get("discord_webhook_token"),
            opencorde_channel_id: r.get("opencorde_channel_id"),
            last_discord_msg_id: r.get("last_discord_msg_id"),
            last_opencorde_msg_id: r.get("last_opencorde_msg_id"),
        })
        .collect();

    Ok(mappings)
}

/// Look up the mapping for an incoming Discord channel ID.
pub async fn get_by_discord_channel(
    db: &PgPool,
    discord_channel_id: u64,
) -> anyhow::Result<Option<ChannelMapping>> {
    let row = sqlx::query(
        "SELECT id, discord_channel_id, discord_webhook_id, discord_webhook_token,
                opencorde_channel_id, last_discord_msg_id, last_opencorde_msg_id
         FROM bridge_channel_mappings
         WHERE discord_channel_id = $1 AND enabled = TRUE",
    )
    .bind(discord_channel_id as i64)
    .fetch_optional(db)
    .await?;

    Ok(row.map(|r| ChannelMapping {
        id: r.get("id"),
        discord_channel_id: r.get("discord_channel_id"),
        discord_webhook_id: r.get("discord_webhook_id"),
        discord_webhook_token: r.get("discord_webhook_token"),
        opencorde_channel_id: r.get("opencorde_channel_id"),
        last_discord_msg_id: r.get("last_discord_msg_id"),
        last_opencorde_msg_id: r.get("last_opencorde_msg_id"),
    }))
}

/// Insert a bridged Discord message into the OpenCorde messages table.
pub async fn insert_discord_message(
    db: &PgPool,
    opencorde_channel_id: i64,
    author_id: i64,
    content: &str,
) -> anyhow::Result<i64> {
    let msg_id = next_msg_id();

    sqlx::query(
        "INSERT INTO messages (id, channel_id, author_id, content)
         VALUES ($1, $2, $3, $4)",
    )
    .bind(msg_id)
    .bind(opencorde_channel_id)
    .bind(author_id)
    .bind(content)
    .execute(db)
    .await?;

    Ok(msg_id)
}

/// Advance the Discord cursor after processing a message.
pub async fn update_discord_cursor(
    db: &PgPool,
    mapping_id: i64,
    discord_msg_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE bridge_channel_mappings
         SET last_discord_msg_id = GREATEST(last_discord_msg_id, $1)
         WHERE id = $2",
    )
    .bind(discord_msg_id)
    .bind(mapping_id)
    .execute(db)
    .await?;
    Ok(())
}

/// Fetch OpenCorde messages newer than the cursor that are NOT from ghost users.
pub async fn pending_opencorde_messages(
    db: &PgPool,
    mapping: &ChannelMapping,
) -> anyhow::Result<Vec<PendingMessage>> {
    let rows = sqlx::query(
        "SELECT m.id, m.content, u.username, u.avatar_url
         FROM messages m
         JOIN users u ON u.id = m.author_id
         WHERE m.channel_id = $1
           AND m.id > $2
           AND m.author_id NOT IN (
               SELECT opencorde_user_id FROM bridge_ghost_users
           )
         ORDER BY m.id ASC
         LIMIT 20",
    )
    .bind(mapping.opencorde_channel_id)
    .bind(mapping.last_opencorde_msg_id)
    .fetch_all(db)
    .await?;

    Ok(rows
        .iter()
        .map(|r| PendingMessage {
            opencorde_msg_id: r.get("id"),
            author_username: r.get("username"),
            author_avatar_url: r.get("avatar_url"),
            content: r.get("content"),
        })
        .collect())
}

/// Advance the OpenCorde cursor after forwarding a message to Discord.
pub async fn update_opencorde_cursor(
    db: &PgPool,
    mapping_id: i64,
    opencorde_msg_id: i64,
) -> anyhow::Result<()> {
    sqlx::query(
        "UPDATE bridge_channel_mappings
         SET last_opencorde_msg_id = GREATEST(last_opencorde_msg_id, $1)
         WHERE id = $2",
    )
    .bind(opencorde_msg_id)
    .bind(mapping_id)
    .execute(db)
    .await?;
    Ok(())
}
