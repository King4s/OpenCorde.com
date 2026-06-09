//! Repository: Bot tokens — create, list, rotate, revoke.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct BotTokenRow {
    pub id: i64,
    pub application_id: i64,
    pub bot_user_id: i64,
    pub token_prefix: String,
    pub token_hash: String,
    pub intents: i64,
    pub label: Option<String>,
    pub last_used_at: Option<DateTime<Utc>>,
    pub created_by: i64,
    pub created_at: DateTime<Utc>,
    pub revoked_at: Option<DateTime<Utc>>,
}

pub async fn create_token(
    pool: &PgPool, id: Snowflake, application_id: Snowflake, bot_user_id: Snowflake,
    token_prefix: &str, token_hash: &str, intents: i64, created_by: Snowflake, label: Option<&str>,
) -> Result<BotTokenRow, sqlx::Error> {
    sqlx::query_as::<_, BotTokenRow>(
        "INSERT INTO bot_tokens (id, application_id, bot_user_id, token_prefix, token_hash, intents, created_by, label)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64()).bind(bot_user_id.as_i64())
    .bind(token_prefix).bind(token_hash).bind(intents).bind(created_by.as_i64()).bind(label)
    .fetch_one(pool).await
}

pub async fn list_tokens(
    pool: &PgPool, application_id: Snowflake,
) -> Result<Vec<BotTokenRow>, sqlx::Error> {
    sqlx::query_as::<_, BotTokenRow>(
        "SELECT * FROM bot_tokens WHERE application_id = $1 ORDER BY created_at DESC"
    ).bind(application_id.as_i64()).fetch_all(pool).await
}

pub async fn rotate_token(
    pool: &PgPool, token_id: Snowflake, application_id: Snowflake,
    new_prefix: &str, new_hash: &str, new_intents: i64,
) -> Result<Option<BotTokenRow>, sqlx::Error> {
    // Revoke old token and create new one in a transaction
    let mut tx = pool.begin().await?;
    
    let revoked = sqlx::query(
        "UPDATE bot_tokens SET revoked_at = NOW() WHERE id = $1 AND application_id = $2 AND revoked_at IS NULL"
    ).bind(token_id.as_i64()).bind(application_id.as_i64()).execute(&mut *tx).await?;
    
    if revoked.rows_affected() == 0 {
        tx.rollback().await?;
        return Ok(None);
    }
    
    let old: BotTokenRow = sqlx::query_as("SELECT * FROM bot_tokens WHERE id = $1")
        .bind(token_id.as_i64()).fetch_one(&mut *tx).await?;
    
    let mut generator = opencorde_core::snowflake::SnowflakeGenerator::new(11, 0);
    let new_id = generator.next_id();
    
    let new_row: BotTokenRow = sqlx::query_as(
        "INSERT INTO bot_tokens (id, application_id, bot_user_id, token_prefix, token_hash, intents, created_by, label)
         VALUES ($1,$2,$3,$4,$5,$6,$7,$8) RETURNING *"
    )
    .bind(new_id.as_i64()).bind(application_id.as_i64()).bind(old.bot_user_id)
    .bind(new_prefix).bind(new_hash).bind(new_intents).bind(old.created_by).bind(old.label)
    .fetch_one(&mut *tx).await?;
    
    tx.commit().await?;
    Ok(Some(new_row))
}

pub async fn revoke_token(
    pool: &PgPool, token_id: Snowflake, application_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "UPDATE bot_tokens SET revoked_at = NOW() WHERE id = $1 AND application_id = $2 AND revoked_at IS NULL"
    ).bind(token_id.as_i64()).bind(application_id.as_i64()).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}

/// Find active token by hash (for gateway authentication)
pub async fn find_by_hash(
    pool: &PgPool, token_hash: &str,
) -> Result<Option<BotTokenRow>, sqlx::Error> {
    sqlx::query_as::<_, BotTokenRow>(
        "SELECT * FROM bot_tokens WHERE token_hash = $1 AND revoked_at IS NULL"
    ).bind(token_hash).fetch_optional(pool).await
}
