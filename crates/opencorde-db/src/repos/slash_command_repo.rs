//! # Repository: Slash Commands
//! CRUD for slash command registrations.
//!
//! ## Depends On
//! - sqlx, opencorde_core::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SlashCommandRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub description: String,
    pub handler_url: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
}

/// Create a slash command for a server.
#[tracing::instrument(skip(pool))]
pub async fn create_command(
    pool: &PgPool,
    id: Snowflake,
    server_id: Snowflake,
    name: &str,
    description: &str,
    handler_url: &str,
    created_by: Snowflake,
) -> Result<SlashCommandRow, sqlx::Error> {
    tracing::info!(
        command_id = id.as_i64(),
        server_id = server_id.as_i64(),
        name = %name,
        "creating slash command"
    );

    sqlx::query_as::<_, SlashCommandRow>(
        r#"
        INSERT INTO slash_commands (id, server_id, name, description, handler_url, created_by, created_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW())
        RETURNING *
        "#,
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(description)
    .bind(handler_url)
    .bind(created_by.as_i64())
    .fetch_one(pool)
    .await
}

/// List all commands for a server.
#[tracing::instrument(skip(pool))]
pub async fn list_commands(
    pool: &PgPool,
    server_id: Snowflake,
) -> Result<Vec<SlashCommandRow>, sqlx::Error> {
    tracing::debug!(server_id = server_id.as_i64(), "listing slash commands");

    sqlx::query_as::<_, SlashCommandRow>(
        "SELECT * FROM slash_commands WHERE server_id = $1 ORDER BY created_at",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Get command by server_id + name.
#[tracing::instrument(skip(pool))]
pub async fn get_by_name(
    pool: &PgPool,
    server_id: Snowflake,
    name: &str,
) -> Result<Option<SlashCommandRow>, sqlx::Error> {
    tracing::debug!(
        server_id = server_id.as_i64(),
        name = %name,
        "fetching slash command by name"
    );

    sqlx::query_as::<_, SlashCommandRow>(
        "SELECT * FROM slash_commands WHERE server_id = $1 AND name = $2",
    )
    .bind(server_id.as_i64())
    .bind(name)
    .fetch_optional(pool)
    .await
}

/// Delete a slash command.
#[tracing::instrument(skip(pool))]
pub async fn delete_command(pool: &PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(command_id = id.as_i64(), "deleting slash command");

    sqlx::query("DELETE FROM slash_commands WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}
