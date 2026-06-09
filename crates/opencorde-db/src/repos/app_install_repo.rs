//! Repository: App installs — server grants for applications.

use chrono::{DateTime, Utc};
use opencorde_core::snowflake::Snowflake;
use sqlx::PgPool;

#[derive(Debug, Clone, sqlx::FromRow)]
pub struct AppInstallRow {
    pub id: i64,
    pub application_id: i64,
    pub server_id: i64,
    pub installed_by: i64,
    pub scopes: Vec<String>,
    pub created_at: DateTime<Utc>,
}

pub async fn create_install(
    pool: &PgPool, id: Snowflake, application_id: Snowflake,
    server_id: Snowflake, installed_by: Snowflake, scopes: &[String],
) -> Result<AppInstallRow, sqlx::Error> {
    sqlx::query_as::<_, AppInstallRow>(
        "INSERT INTO app_installs (id, application_id, server_id, installed_by, scopes)
         VALUES ($1, $2, $3, $4, $5) RETURNING *"
    )
    .bind(id.as_i64()).bind(application_id.as_i64())
    .bind(server_id.as_i64()).bind(installed_by.as_i64()).bind(scopes)
    .fetch_one(pool).await
}

pub async fn list_by_server(
    pool: &PgPool, server_id: Snowflake,
) -> Result<Vec<AppInstallRow>, sqlx::Error> {
    sqlx::query_as::<_, AppInstallRow>(
        "SELECT * FROM app_installs WHERE server_id = $1 ORDER BY created_at DESC"
    ).bind(server_id.as_i64()).fetch_all(pool).await
}

pub async fn delete_install(
    pool: &PgPool, application_id: Snowflake, server_id: Snowflake,
) -> Result<bool, sqlx::Error> {
    let r = sqlx::query("DELETE FROM app_installs WHERE application_id = $1 AND server_id = $2")
        .bind(application_id.as_i64()).bind(server_id.as_i64())
        .execute(pool).await?;
    Ok(r.rows_affected() > 0)
}
