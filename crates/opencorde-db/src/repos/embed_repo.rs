//! Repository: Message embeds — attach rich content to messages.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use serde_json::Value;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct EmbedRow {
    pub id: i64,
    pub message_id: i64,
    pub position: i16,
    pub payload: Value,
    pub created_at: DateTime<Utc>,
}

pub async fn create_embed(
    pool: &PgPool, id: Snowflake, message_id: Snowflake,
    position: i16, payload: &Value,
) -> Result<EmbedRow, sqlx::Error> {
    sqlx::query_as::<_, EmbedRow>(
        "INSERT INTO message_embeds (id, message_id, position, payload) VALUES ($1,$2,$3,$4) RETURNING *"
    ).bind(id.as_i64()).bind(message_id.as_i64()).bind(position).bind(payload)
    .fetch_one(pool).await
}

pub async fn get_embeds(
    pool: &PgPool, message_id: Snowflake,
) -> Result<Vec<EmbedRow>, sqlx::Error> {
    sqlx::query_as::<_, EmbedRow>(
        "SELECT * FROM message_embeds WHERE message_id = $1 ORDER BY position"
    ).bind(message_id.as_i64()).fetch_all(pool).await
}
