//! Repository: Application commands — create, list, delete.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;
use serde_json::Value;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppCommandRow {
    pub id: i64,
    pub application_id: i64,
    pub server_id: Option<i64>,
    pub name: String,
    pub description: String,
    pub command_type: i16,
    pub options: Value,
    pub default_member_permissions: Option<i64>,
    pub dm_permission: bool,
    pub version: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

pub async fn create_command(
    pool: &PgPool, id: Snowflake, application_id: Snowflake,
    server_id: Option<Snowflake>, name: &str, description: &str,
    command_type: i16, options: &Value,
) -> Result<AppCommandRow, sqlx::Error> {
    sqlx::query_as::<_, AppCommandRow>(
        "INSERT INTO application_commands (id, application_id, server_id, name, description, command_type, options)
         VALUES ($1,$2,$3,$4,$5,$6,$7) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64())
    .bind(server_id.map(|s| s.as_i64())).bind(name).bind(description)
    .bind(command_type).bind(options)
    .fetch_one(pool).await
}

pub async fn list_commands(
    pool: &PgPool, application_id: Snowflake, server_id: Option<Snowflake>,
) -> Result<Vec<AppCommandRow>, sqlx::Error> {
    if let Some(sid) = server_id {
        sqlx::query_as::<_, AppCommandRow>(
            "SELECT * FROM application_commands WHERE application_id = $1 AND server_id = $2 ORDER BY name"
        ).bind(application_id.as_i64()).bind(sid.as_i64()).fetch_all(pool).await
    } else {
        sqlx::query_as::<_, AppCommandRow>(
            "SELECT * FROM application_commands WHERE application_id = $1 AND server_id IS NULL ORDER BY name"
        ).bind(application_id.as_i64()).fetch_all(pool).await
    }
}

pub async fn delete_command(
    pool: &PgPool, command_id: Snowflake, application_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query(
        "DELETE FROM application_commands WHERE id = $1 AND application_id = $2"
    ).bind(command_id.as_i64()).bind(application_id.as_i64()).execute(pool).await?;
    Ok(r.rows_affected() > 0)
}
