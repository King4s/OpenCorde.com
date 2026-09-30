//! # Repository: Channel Templates
//! CRUD operations for reusable channel configuration templates.
//!
//! Templates store channel settings and permission overrides that can be
//! reused when creating new channels in a server.
//!
//! ## Depends On
//! - sqlx::PgPool
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

/// A single permission override entry stored as JSONB.
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct OverrideEntry {
    pub target_type: String,
    pub target_id: i64,
    pub allow_bits: i64,
    pub deny_bits: i64,
}

/// Row type for reading channel templates from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ChannelTemplateRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub channel_type: i16,
    pub topic: Option<String>,
    pub nsfw: bool,
    pub slowmode_delay: i32,
    pub e2ee_enabled: bool,
    pub synced_with_category: bool,
    pub permission_overrides: serde_json::Value,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new channel template.
#[tracing::instrument(skip(pool))]
pub async fn create(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    name: &str,
    channel_type: i16,
    topic: Option<&str>,
    nsfw: bool,
    slowmode_delay: i32,
    e2ee_enabled: bool,
    synced_with_category: bool,
    permission_overrides: serde_json::Value,
    created_by: Snowflake,
) -> Result<ChannelTemplateRow, sqlx::Error> {
    tracing::info!(name = %name, server_id = server_id.as_i64(), "creating channel template");

    let row = sqlx::query_as::<_, ChannelTemplateRow>(
        "INSERT INTO channel_templates \
         (id, server_id, name, channel_type, topic, nsfw, slowmode_delay, e2ee_enabled, synced_with_category, permission_overrides, created_by) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(channel_type)
    .bind(topic)
    .bind(nsfw)
    .bind(slowmode_delay)
    .bind(e2ee_enabled)
    .bind(synced_with_category)
    .bind(permission_overrides)
    .bind(created_by.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(template_id = row.id, "channel template created");
    Ok(row)
}

/// List all channel templates for a server.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<ChannelTemplateRow>, sqlx::Error> {
    tracing::info!(server_id = server_id.as_i64(), "listing channel templates");

    sqlx::query_as::<_, ChannelTemplateRow>(
        "SELECT * FROM channel_templates WHERE server_id = $1 ORDER BY name ASC",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Get a channel template by ID.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(
    pool: &PgPool,
    id: Snowflake,
) -> Result<Option<ChannelTemplateRow>, sqlx::Error> {
    sqlx::query_as::<_, ChannelTemplateRow>("SELECT * FROM channel_templates WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// Delete a channel template.
#[tracing::instrument(skip(pool))]
pub async fn delete(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(template_id = id.as_i64(), "deleting channel template");

    sqlx::query("DELETE FROM channel_templates WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_override_entry_serde() {
        let entry = OverrideEntry {
            target_type: "role".to_string(),
            target_id: 100,
            allow_bits: 0x10,
            deny_bits: 0x04,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("role"));
    }
}
