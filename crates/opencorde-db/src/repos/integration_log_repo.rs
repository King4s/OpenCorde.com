//! Repository: Integration logs — track bot/app/webhook activity.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct IntegrationLogRow {
    pub id: i64,
    pub server_id: Option<i64>,
    pub application_id: Option<i64>,
    pub actor_bot_user_id: Option<i64>,
    pub action_type: String,
    pub status: String,
    pub metadata: Value,
    pub created_at: DateTime<Utc>,
}

pub async fn log_event(
    pool: &PgPool, id: Snowflake, server_id: Option<Snowflake>,
    application_id: Option<Snowflake>, actor_bot_user_id: Option<Snowflake>,
    action_type: &str, status: &str, metadata: &Value,
) -> Result<IntegrationLogRow, sqlx::Error> {
    sqlx::query_as::<_, IntegrationLogRow>(
        "INSERT INTO integration_logs (id,server_id,application_id,actor_bot_user_id,action_type,status,metadata)
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *"
    )
    .bind(id.as_i64()).bind(server_id.map(|s| s.as_i64()))
    .bind(application_id.map(|s| s.as_i64()))
    .bind(actor_bot_user_id.map(|s| s.as_i64()))
    .bind(action_type).bind(status).bind(metadata)
    .fetch_one(pool).await
}

pub async fn list_by_server(
    pool: &PgPool, server_id: Snowflake, limit: i64,
) -> Result<Vec<IntegrationLogRow>, sqlx::Error> {
    sqlx::query_as::<_, IntegrationLogRow>(
        "SELECT * FROM integration_logs WHERE server_id = $1 ORDER BY created_at DESC LIMIT $2"
    ).bind(server_id.as_i64()).bind(limit).fetch_all(pool).await
}

/// List integration logs with optional application_id filter.
pub async fn list_by_server_and_app(
    pool: &PgPool,
    server_id: Snowflake,
    application_id: Snowflake,
    limit: i64,
) -> Result<Vec<IntegrationLogRow>, sqlx::Error> {
    sqlx::query_as::<_, IntegrationLogRow>(
        "SELECT * FROM integration_logs WHERE server_id = $1 AND application_id = $2 ORDER BY created_at DESC LIMIT $3"
    ).bind(server_id.as_i64()).bind(application_id.as_i64()).bind(limit).fetch_all(pool).await
}
