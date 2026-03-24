//! # Repository: AutoMod Rules
//! Keyword filter rules for automatic message moderation.
//!
//! ## Depends On
//! - opencorde_core::snowflake::Snowflake

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AutomodRuleRow {
    pub id: i64,
    pub server_id: i64,
    pub name: String,
    pub keywords: String,
    pub enabled: bool,
    pub action: String,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Create a new AutoMod rule.
#[tracing::instrument(skip(pool))]
pub async fn create_rule(
    pool: &sqlx::PgPool,
    id: Snowflake,
    server_id: Snowflake,
    name: &str,
    keywords: &str,
    action: &str,
    created_by: Snowflake,
) -> Result<AutomodRuleRow, sqlx::Error> {
    tracing::info!(
        rule_id = id.as_i64(),
        server_id = server_id.as_i64(),
        name = %name,
        "creating automod rule"
    );

    sqlx::query_as::<_, AutomodRuleRow>(
        r#"
        INSERT INTO automod_rules (id, server_id, name, keywords, action, created_by, created_at, updated_at)
        VALUES ($1, $2, $3, $4, $5, $6, NOW(), NOW())
        RETURNING *
        "#,
    )
    .bind(id.as_i64())
    .bind(server_id.as_i64())
    .bind(name)
    .bind(keywords)
    .bind(action)
    .bind(created_by.as_i64())
    .fetch_one(pool)
    .await
}

/// List all AutoMod rules for a server.
#[tracing::instrument(skip(pool))]
pub async fn list_by_server(
    pool: &sqlx::PgPool,
    server_id: Snowflake,
) -> Result<Vec<AutomodRuleRow>, sqlx::Error> {
    tracing::debug!(server_id = server_id.as_i64(), "listing automod rules");

    sqlx::query_as::<_, AutomodRuleRow>(
        "SELECT * FROM automod_rules WHERE server_id = $1 ORDER BY created_at",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// List enabled AutoMod rules for a server.
#[tracing::instrument(skip(pool))]
pub async fn list_enabled_by_server(
    pool: &sqlx::PgPool,
    server_id: Snowflake,
) -> Result<Vec<AutomodRuleRow>, sqlx::Error> {
    tracing::debug!(server_id = server_id.as_i64(), "listing enabled automod rules");

    sqlx::query_as::<_, AutomodRuleRow>(
        "SELECT * FROM automod_rules WHERE server_id = $1 AND enabled = TRUE ORDER BY created_at",
    )
    .bind(server_id.as_i64())
    .fetch_all(pool)
    .await
}

/// Update an AutoMod rule.
#[tracing::instrument(skip(pool))]
pub async fn update_rule(
    pool: &sqlx::PgPool,
    id: Snowflake,
    name: &str,
    keywords: &str,
    enabled: bool,
    action: &str,
) -> Result<(), sqlx::Error> {
    tracing::info!(rule_id = id.as_i64(), "updating automod rule");

    sqlx::query(
        r#"
        UPDATE automod_rules
        SET name = $2, keywords = $3, enabled = $4, action = $5, updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(id.as_i64())
    .bind(name)
    .bind(keywords)
    .bind(enabled)
    .bind(action)
    .execute(pool)
    .await?;

    Ok(())
}

/// Delete an AutoMod rule.
#[tracing::instrument(skip(pool))]
pub async fn delete_rule(pool: &sqlx::PgPool, id: Snowflake) -> Result<(), sqlx::Error> {
    tracing::info!(rule_id = id.as_i64(), "deleting automod rule");

    sqlx::query("DELETE FROM automod_rules WHERE id = $1")
        .bind(id.as_i64())
        .execute(pool)
        .await?;

    Ok(())
}
