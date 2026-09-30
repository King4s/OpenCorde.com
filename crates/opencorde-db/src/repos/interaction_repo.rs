//! Repository: Interactions — create, update response state, find by token.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct InteractionRow {
    pub id: i64,
    pub application_id: i64,
    pub token_hash: String,
    pub interaction_type: i16,
    pub command_id: Option<i64>,
    pub server_id: Option<i64>,
    pub channel_id: Option<i64>,
    pub user_id: i64,
    pub message_id: Option<i64>,
    pub data: Value,
    pub response_state: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub responded_at: Option<DateTime<Utc>>,
}

pub async fn create_interaction(
    pool: &PgPool,
    id: Snowflake,
    application_id: Snowflake,
    token_hash: &str,
    interaction_type: i16,
    command_id: Option<Snowflake>,
    server_id: Option<Snowflake>,
    channel_id: Option<Snowflake>,
    user_id: Snowflake,
    data: &Value,
    expires_at: DateTime<Utc>,
) -> Result<InteractionRow, sqlx::Error> {
    sqlx::query_as::<_, InteractionRow>(
        "INSERT INTO interactions (id,application_id,token_hash,interaction_type,command_id,server_id,channel_id,user_id,data,expires_at)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64()).bind(token_hash)
    .bind(interaction_type).bind(command_id.map(|s| s.as_i64()))
    .bind(server_id.map(|s| s.as_i64())).bind(channel_id.map(|s| s.as_i64()))
    .bind(user_id.as_i64()).bind(data).bind(expires_at)
    .fetch_one(pool).await
}

pub async fn find_by_token(
    pool: &PgPool,
    interaction_id: Snowflake,
    token_hash: &str,
) -> Result<Option<InteractionRow>, sqlx::Error> {
    sqlx::query_as::<_, InteractionRow>(
        "SELECT * FROM interactions WHERE id = $1 AND token_hash = $2 AND response_state = 'pending' AND expires_at > NOW()"
    ).bind(interaction_id.as_i64()).bind(token_hash).fetch_optional(pool).await
}

pub async fn mark_responded(
    pool: &PgPool,
    interaction_id: Snowflake,
    state: &str,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE interactions SET response_state = $2, responded_at = NOW() WHERE id = $1 AND response_state = 'pending'"
    ).bind(interaction_id.as_i64()).bind(state).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}
