//! # Repository: Servers
//! CRUD operations for Discord-like servers.
//!
//! Provides functions to manage server entities and retrieve server ownership/membership.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

/// Row type for reading servers from the database.
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct ServerRow {
    pub id: i64,
    pub name: String,
    pub owner_id: i64,
    pub icon_url: Option<String>,
    pub banner_url: Option<String>,
    pub description: Option<String>,
    pub vanity_url: Option<String>,
    pub verification_level: i16,
    pub explicit_content_filter: i16,
    pub default_notifications: i16,
    pub system_channel_id: Option<i64>,
    pub rules_channel_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new server.
///
/// # Arguments
/// * `pool` - Database connection pool
/// * `id` - Snowflake ID for the server
/// * `name` - Server name (max 100 chars)
/// * `owner_id` - Snowflake ID of the server owner
///
/// # Errors
/// Returns sqlx::Error if the insert fails.
#[tracing::instrument(skip(pool))]
pub async fn create_server(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    owner_id: Snowflake,
) -> Result<ServerRow, sqlx::Error> {
    tracing::info!(name = %name, owner_id = owner_id.as_i64(), "creating server");

    let row = sqlx::query_as::<_, ServerRow>(
        "INSERT INTO servers (id, name, owner_id) VALUES ($1, $2, $3) RETURNING *",
    )
    .bind(id.as_i64())
    .bind(name)
    .bind(owner_id.as_i64())
    .fetch_one(pool)
    .await?;

    tracing::info!(server_id = row.id, "server created successfully");
    Ok(row)
}

/// Get a server by its Snowflake ID.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn get_by_id(pool: &PgPool, id: Snowflake) -> Result<Option<ServerRow>, sqlx::Error> {
    sqlx::query_as::<_, ServerRow>("SELECT * FROM servers WHERE id = $1")
        .bind(id.as_i64())
        .fetch_optional(pool)
        .await
}

/// List all servers where the user is a member.
///
/// Joins server_members to find servers for a given user.
///
/// # Errors
/// Returns sqlx::Error if the query fails.
#[tracing::instrument(skip(pool))]
pub async fn list_by_user(
    pool: &PgPool,
    user_id: Snowflake,
) -> Result<Vec<ServerRow>, sqlx::Error> {
    tracing::info!(user_id = user_id.as_i64(), "listing user servers");

    sqlx::query_as::<_, ServerRow>(
        "SELECT s.* FROM servers s \
         INNER JOIN server_members m ON s.id = m.server_id \
         WHERE m.user_id = $1 \
         ORDER BY s.id DESC",
    )
    .bind(user_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update a server's settings.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
#[allow(clippy::too_many_arguments)]
pub async fn update_server(
    pool: &PgPool,
    id: Snowflake,
    name: &str,
    description: Option<&str>,
    verification_level: i16,
    explicit_content_filter: i16,
    default_notifications: i16,
    vanity_url: Option<&str>,
    system_channel_id: Option<i64>,
    rules_channel_id: Option<i64>,
) -> Result<(), sqlx::Error> {
    tracing::info!(server_id = id.as_i64(), name = %name, "updating server");

    sqlx::query(
        "UPDATE servers SET name = $1, description = $2, verification_level = $3, \
         explicit_content_filter = $4, default_notifications = $5, vanity_url = $6, \
         system_channel_id = $7, rules_channel_id = $8, updated_at = NOW() WHERE id = $9",
    )
    .bind(name)
    .bind(description)
    .bind(verification_level)
    .bind(explicit_content_filter)
    .bind(default_notifications)
    .bind(vanity_url)
    .bind(system_channel_id)
    .bind(rules_channel_id)
    .bind(id.as_i64())
    .execute(pool)
    .await?;

    Ok(())
}

/// Update a server's icon URL.
///
/// # Errors
/// Returns sqlx::Error if the update fails.
#[tracing::instrument(skip(pool))]
pub async fn update_icon(pool: &PgPool, id: Snowflake, icon_url: &str) -> Result<(), sqlx::Error> {
    tracing::info!(server_id = id.as_i64(), "updating server icon");

    sqlx::query("UPDATE servers SET icon_url = $1, updated_at = NOW() WHERE id = $2")
        .bind(icon_url)
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

/// Delete a server by its ID.
///
/// Cascades to channels, messages, and other related entities.
///
/// # Errors
/// Returns sqlx::Error if the delete fails.
#[tracing::instrument(skip(pool))]
pub async fn delete_server(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(server_id = id.as_i64(), "deleting server");

    sqlx::query("DELETE FROM servers WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_row_creation() {
        let now = Utc::now();
        let row = ServerRow {
            id: 111222333,
            name: "My Server".to_string(),
            owner_id: 999888777,
            icon_url: None,
            banner_url: None,
            description: Some("A test server".to_string()),
            vanity_url: None,
            verification_level: 0,
            explicit_content_filter: 0,
            default_notifications: 0,
            system_channel_id: None,
            rules_channel_id: None,
            created_at: now,
            updated_at: now,
        };

        assert_eq!(row.id, 111222333);
        assert_eq!(row.name, "My Server");
        assert_eq!(row.owner_id, 999888777);
    }
}
